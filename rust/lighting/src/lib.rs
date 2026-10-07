// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Port of `libs/body/lighting`: the exterior lighting controller. Three lights on GPIO
//! pins, set and read over SOME/IP, reported as a `light_status_changed` event on every
//! change, all off when the vehicle switches off, and readable and controllable through
//! diagnostics.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::Cell;

use body_ecu_ports::{
    DiagData, DiagDataProvider, GpioPort, ModeObserver, SomeIpMessage, SomeIpService, VehicleMode,
    ids, printk,
};

/// A light.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LightId {
    /// The headlight.
    Headlight = 0,
    /// The turn signal.
    Turn = 1,
    /// The brake light.
    Brake = 2,
}

/// How many lights there are.
pub const LIGHT_COUNT: usize = 3;

/// The controller's configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LightingConfig {
    /// The SOME/IP service.
    pub service_id: u16,
    /// `set_light_state`.
    pub set_light_state_method: u16,
    /// `get_light_status`.
    pub get_light_status_method: u16,
    /// `light_status_changed`.
    pub light_status_changed_event: u16,
    /// The event's group.
    pub eventgroup_id: u16,
    /// The GPIO pin of each light.
    pub gpio_pins: [u32; LIGHT_COUNT],
    /// The diagnostic data identifier.
    pub diag_did: u16,
}

impl LightingConfig {
    /// The defaults, as a constant for a static.
    pub const DEFAULT: Self = Self {
        service_id: ids::lighting::SERVICE_ID,
        set_light_state_method: ids::lighting::METHOD_SET_LIGHT_STATE,
        get_light_status_method: ids::lighting::METHOD_GET_LIGHT_STATUS,
        light_status_changed_event: ids::lighting::EVENT_LIGHT_STATUS_CHANGED,
        eventgroup_id: ids::lighting::EVENTGROUP_LIGHTING_EVENTS,
        gpio_pins: [0, 1, 2],
        diag_did: 0xF100,
    };
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The exterior lighting controller (`LightingController`).
pub struct LightingController {
    gpio: &'static dyn GpioPort,
    someip: &'static dyn SomeIpService,
    config: LightingConfig,
    states: Cell<[bool; LIGHT_COUNT]>,
}

impl LightingController {
    /// A controller driving `gpio`, serving over `someip`, with `config`.
    pub const fn new(
        gpio: &'static dyn GpioPort,
        someip: &'static dyn SomeIpService,
        config: LightingConfig,
    ) -> Self {
        Self { gpio, someip, config, states: Cell::new([false; LIGHT_COUNT]) }
    }

    /// Register the methods and the event.
    pub fn init(&'static self) {
        self.someip.register_method(
            self.config.service_id,
            self.config.set_light_state_method,
            Box::new(move |request| self.handle_set_light_state(request)),
        );
        self.someip.register_method(
            self.config.service_id,
            self.config.get_light_status_method,
            Box::new(move |request| self.handle_get_light_status(request)),
        );
        self.someip.register_event(
            self.config.service_id,
            self.config.light_status_changed_event,
            self.config.eventgroup_id,
        );
    }

    /// Turn `id` on or off.
    pub fn set_light_state(&self, id: LightId, on: bool) -> bool {
        self.set_light_index(id as usize, on)
    }

    /// Turn light `idx` on or off; refused for an index past the lights, as the C++
    /// refuses a `LightId` cast from any other value.
    pub fn set_light_index(&self, idx: usize, on: bool) -> bool {
        if idx >= LIGHT_COUNT {
            return false;
        }
        let mut states = self.states.get();
        let old_state = states[idx];
        states[idx] = on;
        self.states.set(states);
        self.gpio.write(self.config.gpio_pins[idx], on);
        forkpoint::assert_always!(
            self.states.get()[idx] == on,
            "lighting: a light's state follows the request"
        );
        forkpoint::assert_sometimes!(on, "lighting: a light turns on");
        forkpoint::assert_sometimes!(!on, "lighting: a light turns off");
        forkpoint::lifecycle::send_event("lighting.light", ((idx as u64) << 8) | u64::from(on));
        if old_state != on {
            self.publish_light_status_changed();
        }
        true
    }

    /// Each light's state.
    pub fn get_light_status(&self) -> [bool; LIGHT_COUNT] {
        self.states.get()
    }

    fn handle_set_light_state(&self, request: &SomeIpMessage) -> SomeIpMessage {
        let mut response = request.clone();
        response.message_type = 0x80;
        if request.payload.len() < 2 {
            response.return_code = 0x01;
            return response;
        }
        let light_id = request.payload[0];
        let state = request.payload[1] != 0;
        printk!("[light] set id={} state={}\n", i32::from(light_id), i32::from(state));
        response.return_code =
            if self.set_light_index(usize::from(light_id), state) { 0x00 } else { 0x01 };
        printk!("[light] set done rc={}\n", i32::from(response.return_code));
        response
    }

    fn handle_get_light_status(&self, request: &SomeIpMessage) -> SomeIpMessage {
        let mut response = request.clone();
        response.message_type = 0x80;
        response.return_code = 0x00;
        response.payload = self.states.get().iter().map(|&s| u8::from(s)).collect();
        response
    }

    fn publish_light_status_changed(&self) {
        let payload: Vec<u8> = self.states.get().iter().map(|&s| u8::from(s)).collect();
        self.someip.send_event(
            self.config.service_id,
            self.config.light_status_changed_event,
            &payload,
        );
    }
}

impl ModeObserver for LightingController {
    fn on_mode_changed(&self, _old_mode: VehicleMode, new_mode: VehicleMode) {
        if new_mode == VehicleMode::Off {
            for i in 0..LIGHT_COUNT {
                self.set_light_index(i, false);
            }
        }
    }
}

impl DiagDataProvider for LightingController {
    fn read_data(&self, did: u16) -> Option<DiagData> {
        if did != self.config.diag_did {
            return None;
        }
        Some(DiagData { did, data: self.states.get().iter().map(|&s| u8::from(s)).collect() })
    }

    fn io_control(&self, did: u16, control_param: &[u8]) -> bool {
        if did != self.config.diag_did {
            return false;
        }
        if control_param.len() < 2 {
            return false;
        }
        let light_id = control_param[0];
        let state = control_param[1] != 0;
        self.set_light_index(usize::from(light_id), state)
    }
}

#[cfg(test)]
mod tests {
    use body_ecu_ports::mock::{MockGpioPort, MockSomeIpService, leak};

    use super::*;

    struct Fixture {
        gpio: &'static MockGpioPort,
        someip: &'static MockSomeIpService,
        config: LightingConfig,
        ctrl: &'static LightingController,
    }

    fn fixture() -> Fixture {
        let gpio = leak(MockGpioPort::new());
        let someip = leak(MockSomeIpService::new());
        let config = LightingConfig::default();
        let ctrl = leak(LightingController::new(gpio, someip, config.clone()));
        ctrl.init();
        Fixture { gpio, someip, config, ctrl }
    }

    #[test]
    fn set_light_state() {
        let f = fixture();
        assert!(f.ctrl.set_light_state(LightId::Headlight, true));
        assert_eq!(f.gpio.take_writes(), [(f.config.gpio_pins[0], true)]);
    }

    #[test]
    fn get_light_status() {
        let f = fixture();
        f.ctrl.set_light_state(LightId::Turn, true);
        assert_eq!(f.gpio.take_writes().len(), 1);
        assert_eq!(f.ctrl.get_light_status(), [false, true, false]);
    }

    #[test]
    fn light_status_changed_event() {
        let f = fixture();
        f.ctrl.set_light_state(LightId::Headlight, true);
        assert_eq!(f.gpio.take_writes().len(), 1);
        let events = f.someip.take_sent_events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            (events[0].0, events[0].1),
            (f.config.service_id, f.config.light_status_changed_event)
        );
    }

    #[test]
    fn no_event_on_same_state() {
        let f = fixture();
        f.ctrl.set_light_state(LightId::Headlight, true);
        f.ctrl.set_light_state(LightId::Headlight, true);
        assert_eq!(f.gpio.take_writes().len(), 2);
        assert_eq!(f.someip.take_sent_events().len(), 1);
    }

    #[test]
    fn invalid_light_id() {
        let f = fixture();
        assert!(!f.ctrl.set_light_index(99, true));
        assert!(f.gpio.take_writes().is_empty());
    }

    #[test]
    fn all_lights_off_on_mode_change() {
        let f = fixture();
        f.ctrl.set_light_state(LightId::Headlight, true);
        f.ctrl.set_light_state(LightId::Turn, true);
        f.ctrl.set_light_state(LightId::Brake, true);
        f.ctrl.on_mode_changed(VehicleMode::Run, VehicleMode::Off);
        assert!(f.gpio.take_writes().len() >= 3);
        assert_eq!(f.ctrl.get_light_status(), [false, false, false]);
    }

    #[test]
    fn handle_set_light_state_method() {
        let f = fixture();
        let request = SomeIpMessage {
            service_id: f.config.service_id,
            method_id: f.config.set_light_state_method,
            payload: alloc::vec![0x00, 0x01],
            ..Default::default()
        };
        let response =
            f.someip.call(f.config.service_id, f.config.set_light_state_method, &request);
        assert_eq!(response.return_code, 0x00);
        assert_eq!(response.message_type, 0x80);
        assert_eq!(f.gpio.take_writes(), [(f.config.gpio_pins[0], true)]);
        assert_eq!(f.someip.take_sent_events().len(), 1);
    }

    #[test]
    fn diag_read_data() {
        let f = fixture();
        f.ctrl.set_light_state(LightId::Headlight, true);
        assert_eq!(f.gpio.take_writes().len(), 1);
        let data = f.ctrl.read_data(f.config.diag_did).expect("the did is served");
        assert_eq!(data.data, [1, 0, 0]);
    }

    #[test]
    fn diag_read_unknown_did() {
        let f = fixture();
        assert!(f.ctrl.read_data(0xFFFF).is_none());
    }
}
