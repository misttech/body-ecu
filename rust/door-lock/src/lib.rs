// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Port of `libs/body/door-lock`: the door lock state machine. Locked and unlocked over
//! SOME/IP, toggled by the button, commanded over the signal bus, locked when the vehicle
//! enters Run, refused while the vehicle moves or a door is open, and readable and
//! controllable through diagnostics.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::Cell;

use body_ecu_ports::{
    ButtonInput, DiagData, DiagDataProvider, GpioPort, ModeObserver, SignalBus, SignalValue,
    SomeIpMessage, SomeIpService, VehicleMode, ids, printk,
};

/// The lock's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LockState {
    /// Unlocked.
    Unlocked = 0,
    /// Locked.
    Locked = 1,
    /// Failed: neither command works.
    Error = 2,
}

/// The controller's configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoorLockConfig {
    /// The SOME/IP service.
    pub service_id: u16,
    /// `lock`.
    pub lock_method: u16,
    /// `unlock`.
    pub unlock_method: u16,
    /// `get_status`.
    pub get_status_method: u16,
    /// `lock_state_changed`.
    pub lock_state_changed_event: u16,
    /// The event's group.
    pub eventgroup_id: u16,
    /// The GPIO pin of the lock actuator.
    pub lock_gpio_pin: u32,
    /// The diagnostic data identifier.
    pub diag_did: u16,
    /// The signal that reports the lock.
    pub signal_is_locked: &'static str,
    /// The signal that commands the lock.
    pub signal_command_lock: &'static str,
    /// The signal that answers a command.
    pub signal_command_response: &'static str,
    /// The vehicle's speed.
    pub signal_speed: &'static str,
    /// Whether the door is open.
    pub signal_door_open: &'static str,
}

impl DoorLockConfig {
    /// The defaults, as a constant for a static.
    pub const DEFAULT: Self = Self {
        service_id: ids::door_lock::SERVICE_ID,
        lock_method: ids::door_lock::METHOD_LOCK,
        unlock_method: ids::door_lock::METHOD_UNLOCK,
        get_status_method: ids::door_lock::METHOD_GET_STATUS,
        lock_state_changed_event: ids::door_lock::EVENT_LOCK_STATE_CHANGED,
        eventgroup_id: ids::door_lock::EVENTGROUP_DOOR_EVENTS,
        lock_gpio_pin: 10,
        diag_did: 0xF101,
        signal_is_locked: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
        signal_command_lock: "Vehicle.Command.Door.Lock",
        signal_command_response: "Vehicle.Command.Door.Response",
        signal_speed: "Vehicle.Speed",
        signal_door_open: "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
    };
}

impl Default for DoorLockConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The door lock controller (`DoorLockController`).
pub struct DoorLockController {
    gpio: &'static dyn GpioPort,
    button: &'static dyn ButtonInput,
    someip: &'static dyn SomeIpService,
    config: DoorLockConfig,
    signal_bus: Option<&'static dyn SignalBus>,
    state: Cell<LockState>,
}

impl DoorLockController {
    /// A controller driving `gpio`, toggled by `button`, serving over `someip`, with
    /// `config`, and on `signal_bus` when there is one.
    pub const fn new(
        gpio: &'static dyn GpioPort,
        button: &'static dyn ButtonInput,
        someip: &'static dyn SomeIpService,
        config: DoorLockConfig,
        signal_bus: Option<&'static dyn SignalBus>,
    ) -> Self {
        Self { gpio, button, someip, config, signal_bus, state: Cell::new(LockState::Unlocked) }
    }

    /// Register the methods, the event, the button, and the command signal.
    pub fn init(&'static self) {
        self.someip.register_method(
            self.config.service_id,
            self.config.lock_method,
            Box::new(move |request| self.handle_lock(request)),
        );
        self.someip.register_method(
            self.config.service_id,
            self.config.unlock_method,
            Box::new(move |request| self.handle_unlock(request)),
        );
        self.someip.register_method(
            self.config.service_id,
            self.config.get_status_method,
            Box::new(move |request| self.handle_get_status(request)),
        );
        self.someip.register_event(
            self.config.service_id,
            self.config.lock_state_changed_event,
            self.config.eventgroup_id,
        );

        self.button.on_press(Box::new(move || self.on_button_press()));

        if let Some(signal_bus) = self.signal_bus {
            signal_bus.subscribe(
                self.config.signal_command_lock,
                Box::new(move |path, value| self.on_command_signal(path, value)),
            );
        }
    }

    fn check_safety_constraints(&self) -> bool {
        let Some(signal_bus) = self.signal_bus else { return true };

        if let Some(SignalValue::Float(speed)) = signal_bus.get(self.config.signal_speed)
            && speed > 5.0
        {
            return false;
        }

        if let Some(SignalValue::Bool(true)) = signal_bus.get(self.config.signal_door_open) {
            return false;
        }

        true
    }

    /// Lock the door; returns whether it is locked.
    pub fn lock(&self) -> bool {
        if self.state.get() == LockState::Error {
            return false;
        }
        if self.state.get() == LockState::Locked {
            return true;
        }
        if !self.check_safety_constraints() {
            return false;
        }

        let old = self.state.get();
        self.state.set(LockState::Locked);
        self.gpio.write(self.config.lock_gpio_pin, true);
        self.publish_state_changed(old, LockState::Locked);
        true
    }

    /// Unlock the door; returns whether it is unlocked.
    pub fn unlock(&self) -> bool {
        if self.state.get() == LockState::Error {
            return false;
        }
        if self.state.get() == LockState::Unlocked {
            return true;
        }
        if !self.check_safety_constraints() {
            return false;
        }

        let old = self.state.get();
        self.state.set(LockState::Unlocked);
        self.gpio.write(self.config.lock_gpio_pin, false);
        self.publish_state_changed(old, LockState::Unlocked);
        true
    }

    /// The lock's state.
    pub fn get_state(&self) -> LockState {
        self.state.get()
    }

    /// Put the lock in its error state.
    pub fn set_error(&self) {
        let old = self.state.get();
        self.state.set(LockState::Error);
        self.publish_state_changed(old, LockState::Error);
    }

    fn on_button_press(&self) {
        if self.state.get() == LockState::Locked {
            printk!("[door_lock] Button pressed -- unlocking\n");
            self.unlock();
        } else if self.state.get() == LockState::Unlocked {
            printk!("[door_lock] Button pressed -- locking\n");
            self.lock();
        }
    }

    fn on_command_signal(&self, _path: &str, value: &SignalValue) {
        let SignalValue::Bool(cmd) = value else { return };

        let success = if *cmd { self.lock() } else { self.unlock() };

        if let Some(signal_bus) = self.signal_bus {
            let response_code: i32 = if success { 0x00 } else { 0x01 };
            signal_bus
                .publish(self.config.signal_command_response, &SignalValue::Int(response_code));
        }
    }

    fn handle_lock(&self, request: &SomeIpMessage) -> SomeIpMessage {
        let mut response = request.clone();
        response.message_type = 0x80;
        response.return_code = if self.lock() { 0x00 } else { 0x01 };
        response
    }

    fn handle_unlock(&self, request: &SomeIpMessage) -> SomeIpMessage {
        let mut response = request.clone();
        response.message_type = 0x80;
        response.return_code = if self.unlock() { 0x00 } else { 0x01 };
        response
    }

    fn handle_get_status(&self, request: &SomeIpMessage) -> SomeIpMessage {
        let mut response = request.clone();
        response.message_type = 0x80;
        response.return_code = 0x00;
        response.payload = alloc::vec![self.state.get() as u8];
        response
    }

    fn publish_state_changed(&self, old_state: LockState, new_state: LockState) {
        forkpoint::assert_always!(
            old_state != new_state,
            "door_lock: a state change changes the state"
        );
        forkpoint::assert_sometimes!(new_state == LockState::Locked, "door_lock: the door locks");
        forkpoint::assert_sometimes!(
            new_state == LockState::Unlocked,
            "door_lock: the door unlocks"
        );
        forkpoint::lifecycle::send_event("door_lock.state", new_state as u64);
        let payload: Vec<u8> = alloc::vec![old_state as u8, new_state as u8];
        self.someip.send_event(
            self.config.service_id,
            self.config.lock_state_changed_event,
            &payload,
        );

        if let Some(signal_bus) = self.signal_bus {
            signal_bus.publish(
                self.config.signal_is_locked,
                &SignalValue::Bool(new_state == LockState::Locked),
            );
        }
    }
}

impl ModeObserver for DoorLockController {
    fn on_mode_changed(&self, _old_mode: VehicleMode, new_mode: VehicleMode) {
        if new_mode == VehicleMode::Run {
            let locked = self.lock();
            forkpoint::assert_sometimes!(locked, "door_lock: entering Run locks the door");
        }
    }
}

impl DiagDataProvider for DoorLockController {
    fn read_data(&self, did: u16) -> Option<DiagData> {
        if did != self.config.diag_did {
            return None;
        }
        Some(DiagData { did, data: alloc::vec![self.state.get() as u8] })
    }

    fn io_control(&self, did: u16, control_param: &[u8]) -> bool {
        if did != self.config.diag_did {
            return false;
        }
        if control_param.is_empty() {
            return false;
        }
        if control_param[0] != 0 { self.lock() } else { self.unlock() }
    }
}

#[cfg(test)]
mod tests {
    use body_ecu_ports::mock::{
        MockButtonInput, MockGpioPort, MockSignalBus, MockSomeIpService, leak,
    };

    use super::*;

    struct Fixture {
        gpio: &'static MockGpioPort,
        button: &'static MockButtonInput,
        someip: &'static MockSomeIpService,
        signal_bus: Option<&'static MockSignalBus>,
        config: DoorLockConfig,
        ctrl: &'static DoorLockController,
    }

    fn fixture(with_signal_bus: bool) -> Fixture {
        let gpio = leak(MockGpioPort::new());
        let button = leak(MockButtonInput::new());
        let someip = leak(MockSomeIpService::new());
        let signal_bus = with_signal_bus.then(|| leak(MockSignalBus::new()));
        let config = DoorLockConfig::default();
        let bus: Option<&'static dyn SignalBus> = signal_bus.map(|b| b as &dyn SignalBus);
        let ctrl = leak(DoorLockController::new(gpio, button, someip, config.clone(), bus));
        ctrl.init();
        assert_eq!(someip.method_count(), 3);
        assert_eq!(someip.events.borrow().len(), 1);
        assert_eq!(button.registrations.get(), 1);
        if let Some(bus) = signal_bus {
            assert_eq!(bus.subscribed_paths(), [config.signal_command_lock]);
        }
        Fixture { gpio, button, someip, signal_bus, config, ctrl }
    }

    #[test]
    fn lock_unlock_transition() {
        let f = fixture(false);
        assert_eq!(f.ctrl.get_state(), LockState::Unlocked);
        assert!(f.ctrl.lock());
        assert_eq!(f.gpio.take_writes(), [(f.config.lock_gpio_pin, true)]);
        assert_eq!(f.someip.take_sent_events().len(), 1);
        assert_eq!(f.ctrl.get_state(), LockState::Locked);
        assert!(f.ctrl.unlock());
        assert_eq!(f.gpio.take_writes(), [(f.config.lock_gpio_pin, false)]);
        assert_eq!(f.someip.take_sent_events().len(), 1);
        assert_eq!(f.ctrl.get_state(), LockState::Unlocked);
    }

    #[test]
    fn double_lock_no_op() {
        let f = fixture(false);
        f.ctrl.lock();
        assert_eq!(f.gpio.take_writes().len(), 1);
        assert_eq!(f.someip.take_sent_events().len(), 1);
        assert!(f.ctrl.lock());
        assert!(f.gpio.take_writes().is_empty());
        assert!(f.someip.take_sent_events().is_empty());
    }

    #[test]
    fn get_status() {
        let f = fixture(false);
        assert_eq!(f.ctrl.get_state(), LockState::Unlocked);
        f.ctrl.lock();
        assert_eq!(f.gpio.take_writes().len(), 1);
        assert_eq!(f.someip.take_sent_events().len(), 1);
        assert_eq!(f.ctrl.get_state(), LockState::Locked);
    }

    #[test]
    fn lock_state_changed_event() {
        let f = fixture(false);
        f.ctrl.lock();
        assert_eq!(f.gpio.take_writes().len(), 1);
        let events = f.someip.take_sent_events();
        assert_eq!(events.len(), 1);
        let (service, event, payload) = &events[0];
        assert_eq!((*service, *event), (f.config.service_id, f.config.lock_state_changed_event));
        assert_eq!(payload.as_slice(), [LockState::Unlocked as u8, LockState::Locked as u8]);
    }

    #[test]
    fn button_toggle() {
        let f = fixture(false);
        assert!(f.button.has_callback());
        f.button.press();
        assert_eq!(f.gpio.take_writes(), [(f.config.lock_gpio_pin, true)]);
        assert_eq!(f.someip.take_sent_events().len(), 1);
        assert_eq!(f.ctrl.get_state(), LockState::Locked);
        f.button.press();
        assert_eq!(f.gpio.take_writes(), [(f.config.lock_gpio_pin, false)]);
        assert_eq!(f.someip.take_sent_events().len(), 1);
        assert_eq!(f.ctrl.get_state(), LockState::Unlocked);
    }

    #[test]
    fn error_state() {
        let f = fixture(false);
        f.ctrl.set_error();
        assert_eq!(f.someip.take_sent_events().len(), 1);
        assert_eq!(f.ctrl.get_state(), LockState::Error);
        assert!(!f.ctrl.lock());
        assert!(!f.ctrl.unlock());
    }

    #[test]
    fn auto_lock_on_run_mode() {
        let f = fixture(false);
        f.ctrl.on_mode_changed(VehicleMode::Accessory, VehicleMode::Run);
        assert_eq!(f.gpio.take_writes(), [(f.config.lock_gpio_pin, true)]);
        assert_eq!(f.someip.take_sent_events().len(), 1);
        assert_eq!(f.ctrl.get_state(), LockState::Locked);
    }

    #[test]
    fn lock_publishes_to_signal_bus() {
        let f = fixture(true);
        let bus = f.signal_bus.unwrap();
        assert!(f.ctrl.lock());
        assert_eq!(f.gpio.take_writes().len(), 1);
        assert_eq!(f.someip.take_sent_events().len(), 1);
        assert_eq!(
            bus.published.borrow().as_slice(),
            [(f.config.signal_is_locked.into(), SignalValue::Bool(true))]
        );
    }

    #[test]
    fn unlock_publishes_to_signal_bus() {
        let f = fixture(true);
        let bus = f.signal_bus.unwrap();
        f.ctrl.lock();
        f.ctrl.unlock();
        assert_eq!(f.gpio.take_writes().len(), 2);
        assert_eq!(f.someip.take_sent_events().len(), 2);
        assert_eq!(bus.publications(f.config.signal_is_locked), 2);
    }

    #[test]
    fn command_signal_locks_vehicle() {
        let f = fixture(true);
        let bus = f.signal_bus.unwrap();
        bus.deliver(f.config.signal_command_lock, &SignalValue::Bool(true));
        assert_eq!(f.gpio.take_writes(), [(f.config.lock_gpio_pin, true)]);
        assert_eq!(f.someip.take_sent_events().len(), 1);
        assert_eq!(bus.publications(f.config.signal_is_locked), 1);
        assert_eq!(bus.publications(f.config.signal_command_response), 1);
        assert_eq!(f.ctrl.get_state(), LockState::Locked);
    }

    #[test]
    fn command_signal_unlocks_vehicle() {
        let f = fixture(true);
        let bus = f.signal_bus.unwrap();
        f.ctrl.lock();
        bus.deliver(f.config.signal_command_lock, &SignalValue::Bool(false));
        assert_eq!(f.gpio.take_writes().len(), 2);
        assert_eq!(f.someip.take_sent_events().len(), 2);
        assert_eq!(bus.publications(f.config.signal_is_locked), 2);
        assert_eq!(bus.publications(f.config.signal_command_response), 1);
        assert_eq!(f.ctrl.get_state(), LockState::Unlocked);
    }

    #[test]
    fn lock_rejected_when_vehicle_moving() {
        let f = fixture(true);
        let bus = f.signal_bus.unwrap();
        bus.set(f.config.signal_speed, SignalValue::Float(10.0));
        assert!(!f.ctrl.lock());
        assert_eq!(f.ctrl.get_state(), LockState::Unlocked);
    }

    #[test]
    fn lock_rejected_when_door_open() {
        let f = fixture(true);
        let bus = f.signal_bus.unwrap();
        bus.set(f.config.signal_door_open, SignalValue::Bool(true));
        assert!(!f.ctrl.lock());
        assert_eq!(f.ctrl.get_state(), LockState::Unlocked);
    }

    #[test]
    fn lock_allowed_when_stationary_door_closed() {
        let f = fixture(true);
        let bus = f.signal_bus.unwrap();
        bus.set(f.config.signal_speed, SignalValue::Float(0.0));
        bus.set(f.config.signal_door_open, SignalValue::Bool(false));
        assert!(f.ctrl.lock());
        assert_eq!(f.gpio.take_writes().len(), 1);
        assert_eq!(f.someip.take_sent_events().len(), 1);
        assert_eq!(bus.publications(f.config.signal_is_locked), 1);
        assert_eq!(f.ctrl.get_state(), LockState::Locked);
    }
}
