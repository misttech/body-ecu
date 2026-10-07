// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Port of `libs/body/speed-simulator`: a potentiometer on an ADC channel stands in for
//! the accelerator. Every 100 ms in Run the sample is smoothed and quantized to 5 km/h
//! steps, published as a `speed_changed` event when it changes and on the signal bus
//! always; outside Run the speed is zero; a SOME/IP setter overrides the ADC.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::Cell;

use body_ecu_ports::{
    AdcInput, INVALID_TIMER_ID, ModeObserver, SignalBus, SignalValue, SomeIpMessage, SomeIpService,
    TimerId, TimerService, VehicleMode, ids, printk,
};

/// The simulator's configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct SpeedSimulatorConfig {
    /// The SOME/IP service.
    pub service_id: u16,
    /// `get_speed`.
    pub get_speed_method: u16,
    /// `set_speed`.
    pub set_speed_method: u16,
    /// `speed_changed`.
    pub speed_changed_event: u16,
    /// The event's group.
    pub eventgroup_id: u16,
    /// The ADC channel the potentiometer is on.
    pub adc_channel: u8,
    /// The speed at full scale.
    pub max_speed_kmh: f32,
    /// Samples this close to either end snap to it.
    pub adc_dead_zone: i32,
    /// How often the ADC is read.
    pub update_interval_ms: u32,
    /// The vehicle's speed.
    pub signal_speed: &'static str,
}

impl SpeedSimulatorConfig {
    /// The defaults, as a constant for a static.
    pub const DEFAULT: Self = Self {
        service_id: ids::speed_sensor::SERVICE_ID,
        get_speed_method: ids::speed_sensor::METHOD_GET_SPEED,
        set_speed_method: ids::speed_sensor::METHOD_SET_SPEED,
        speed_changed_event: ids::speed_sensor::EVENT_SPEED_CHANGED,
        eventgroup_id: ids::speed_sensor::EVENTGROUP_SPEED_EVENTS,
        adc_channel: 0,
        max_speed_kmh: 200.0,
        adc_dead_zone: 200,
        update_interval_ms: 100,
        signal_speed: "Vehicle.Speed",
    };
}

impl Default for SpeedSimulatorConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Round half away from zero, as `std::round` does. `core` has no float rounding, so
/// this does it through an integer, which the speeds here fit in.
fn round(value: f32) -> f32 {
    if value >= 0.0 { (value + 0.5) as i32 as f32 } else { (value - 0.5) as i32 as f32 }
}

/// The speed simulator (`SpeedSimulator`).
pub struct SpeedSimulator {
    adc: &'static dyn AdcInput,
    someip: &'static dyn SomeIpService,
    timer: &'static dyn TimerService,
    config: SpeedSimulatorConfig,
    signal_bus: Option<&'static dyn SignalBus>,
    timer_id: Cell<TimerId>,
    current_speed_kmh: Cell<f32>,
    smoothed_adc: Cell<f32>,
    last_sent_speed_kmh: Cell<f32>,
    tick_count: Cell<u32>,
    speed_override: Cell<f32>,
    current_mode: Cell<VehicleMode>,
}

impl SpeedSimulator {
    /// A simulator reading `adc`, serving over `someip`, ticking on `timer`.
    pub const fn new(
        adc: &'static dyn AdcInput,
        someip: &'static dyn SomeIpService,
        timer: &'static dyn TimerService,
        config: SpeedSimulatorConfig,
        signal_bus: Option<&'static dyn SignalBus>,
    ) -> Self {
        Self {
            adc,
            someip,
            timer,
            config,
            signal_bus,
            timer_id: Cell::new(0),
            current_speed_kmh: Cell::new(0.0),
            smoothed_adc: Cell::new(0.0),
            last_sent_speed_kmh: Cell::new(0.0),
            tick_count: Cell::new(0),
            speed_override: Cell::new(-1.0),
            current_mode: Cell::new(VehicleMode::Off),
        }
    }

    /// Register the methods and the event, and start the periodic read.
    pub fn init(&'static self) {
        self.someip.register_method(
            self.config.service_id,
            self.config.get_speed_method,
            Box::new(move |request| self.handle_get_speed(request)),
        );
        self.someip.register_method(
            self.config.service_id,
            self.config.set_speed_method,
            Box::new(move |request| self.handle_set_speed(request)),
        );
        self.someip.register_event(
            self.config.service_id,
            self.config.speed_changed_event,
            self.config.eventgroup_id,
        );

        self.timer_id.set(self.timer.start_periodic(
            self.config.update_interval_ms,
            Box::new(move || self.on_timer_tick()),
        ));

        if self.timer_id.get() == INVALID_TIMER_ID {
            printk!("[speed_sim] FAULT: periodic timer allocation failed\n");
        }

        printk!(
            "[speed_sim] Initialized (interval={}ms, max={} km/h)\n",
            self.config.update_interval_ms,
            self.config.max_speed_kmh as i32
        );
    }

    /// The speed.
    pub fn get_speed_kmh(&self) -> f32 {
        self.current_speed_kmh.get()
    }

    fn send_speed(&self, speed: f32) {
        let payload = Self::serialize_float(speed);
        self.someip.send_event(self.config.service_id, self.config.speed_changed_event, &payload);
        self.last_sent_speed_kmh.set(speed);
    }

    fn publish_speed(&self, speed: f32) {
        if let Some(signal_bus) = self.signal_bus {
            signal_bus.publish(self.config.signal_speed, &SignalValue::Float(speed));
        }
    }

    fn on_timer_tick(&self) {
        if self.current_mode.get() != VehicleMode::Run {
            if self.current_speed_kmh.get() != 0.0 {
                self.current_speed_kmh.set(0.0);
                self.send_speed(0.0);
            }
            self.publish_speed(0.0);
            return;
        }

        const STEP: f32 = 5.0;
        if self.speed_override.get() >= 0.0 {
            let speed = (round(self.speed_override.get() / STEP) * STEP)
                .clamp(0.0, self.config.max_speed_kmh);
            self.current_speed_kmh.set(speed);

            if speed != self.last_sent_speed_kmh.get() {
                self.send_speed(speed);
            }
            self.publish_speed(speed);
            return;
        }

        let raw = self.adc.read(self.config.adc_channel);
        if raw < 0 {
            // An ADC read failure holds the last known speed rather than driving to 0.
            return;
        }
        let mut clamped = raw.clamp(0, 4095);

        if clamped < self.config.adc_dead_zone {
            clamped = 0;
        } else if clamped > (4095 - self.config.adc_dead_zone) {
            clamped = 4095;
        }

        const ALPHA: f32 = 0.03;
        let smoothed = self.smoothed_adc.get() + ALPHA * (clamped as f32 - self.smoothed_adc.get());
        self.smoothed_adc.set(smoothed);

        let continuous = (smoothed / 4095.0) * self.config.max_speed_kmh;
        let speed = (round(continuous / STEP) * STEP).clamp(0.0, self.config.max_speed_kmh);
        self.current_speed_kmh.set(speed);

        let tick_count = self.tick_count.get().wrapping_add(1);
        self.tick_count.set(tick_count);
        if tick_count.is_multiple_of(50) {
            printk!(
                "[speed_sim] adc_ch={} raw={} smooth={} speed={}\n",
                self.config.adc_channel,
                raw,
                smoothed as i32,
                speed as i32
            );
        }

        if speed != self.last_sent_speed_kmh.get() {
            self.send_speed(speed);
        }
        self.publish_speed(speed);
    }

    fn handle_get_speed(&self, request: &SomeIpMessage) -> SomeIpMessage {
        let mut response = request.clone();
        response.message_type = 0x80;
        response.return_code = 0x00;
        response.payload = Self::serialize_float(self.current_speed_kmh.get());
        response
    }

    fn handle_set_speed(&self, request: &SomeIpMessage) -> SomeIpMessage {
        let mut response = request.clone();
        response.message_type = 0x80;

        if request.payload.len() < 4 {
            response.return_code = 0x01;
            return response;
        }

        let value = Self::deserialize_float(&request.payload);
        if value < 0.0 {
            self.speed_override.set(-1.0);
            printk!("[speed_sim] Override cleared, resuming ADC\n");
        } else {
            self.speed_override.set(value.clamp(0.0, self.config.max_speed_kmh));
            printk!("[speed_sim] Speed override: {} km/h\n", self.speed_override.get() as i32);
        }

        response.return_code = 0x00;
        response
    }

    /// A float from four big-endian bytes; 0 when there are fewer.
    pub fn deserialize_float(data: &[u8]) -> f32 {
        let Some(bytes) = data.get(..4) else { return 0.0 };
        f32::from_bits(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// A float as four big-endian bytes.
    pub fn serialize_float(value: f32) -> Vec<u8> {
        value.to_bits().to_be_bytes().to_vec()
    }
}

impl ModeObserver for SpeedSimulator {
    fn on_mode_changed(&self, _old_mode: VehicleMode, new_mode: VehicleMode) {
        self.current_mode.set(new_mode);
        if new_mode != VehicleMode::Run {
            self.current_speed_kmh.set(0.0);
            if self.current_speed_kmh.get() != self.last_sent_speed_kmh.get() {
                self.send_speed(0.0);
            }
            self.publish_speed(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use body_ecu_ports::mock::{
        MockAdcInput, MockSignalBus, MockSomeIpService, MockTimerService, leak,
    };

    use super::*;

    struct Fixture {
        adc: &'static MockAdcInput,
        someip: &'static MockSomeIpService,
        timer: &'static MockTimerService,
        signal_bus: Option<&'static MockSignalBus>,
        config: SpeedSimulatorConfig,
        sim: &'static SpeedSimulator,
    }

    fn fixture(with_signal_bus: bool) -> Fixture {
        let adc = leak(MockAdcInput::new());
        let someip = leak(MockSomeIpService::new());
        let timer = leak(MockTimerService::new());
        let signal_bus = with_signal_bus.then(|| leak(MockSignalBus::new()));
        let config = SpeedSimulatorConfig::default();
        let bus: Option<&'static dyn SignalBus> = signal_bus.map(|b| b as &dyn SignalBus);
        let sim = leak(SpeedSimulator::new(adc, someip, timer, config.clone(), bus));
        sim.init();
        assert_eq!(someip.method_count(), 2);
        assert_eq!(someip.events.borrow().len(), 1);
        assert_eq!(timer.periodics.borrow()[0].0, config.update_interval_ms);
        sim.on_mode_changed(VehicleMode::Off, VehicleMode::Run);
        Fixture { adc, someip, timer, signal_bus, config, sim }
    }

    impl Fixture {
        fn tick(&self, times: usize) {
            for _ in 0..times {
                self.timer.tick_first_periodic();
            }
        }
    }

    #[test]
    fn init_registers_timer_and_someip() {
        let f = fixture(false);
        assert_eq!(f.timer.periodics.borrow().len(), 1);
        assert_eq!(f.sim.get_speed_kmh(), 0.0);
    }

    #[test]
    fn zero_adc_stays_at_zero() {
        let f = fixture(false);
        f.adc.value.set(0);
        f.tick(50);
        assert!(f.someip.take_sent_events().is_empty());
        assert_eq!(f.sim.get_speed_kmh(), 0.0);
    }

    #[test]
    fn full_adc_converges_to_max_speed() {
        let f = fixture(false);
        f.adc.value.set(4095);
        f.tick(300);
        assert!(!f.someip.take_sent_events().is_empty());
        assert_eq!(f.sim.get_speed_kmh(), f.config.max_speed_kmh);
    }

    #[test]
    fn speed_is_quantized_to_5_kmh_steps() {
        let f = fixture(false);
        f.adc.value.set(2048);
        f.tick(300);
        assert!(!f.someip.take_sent_events().is_empty());
        let speed = f.sim.get_speed_kmh();
        assert_eq!(speed % 5.0, 0.0);
    }

    #[test]
    fn dead_zone_snaps_to_zero() {
        let f = fixture(false);
        f.adc.value.set(150);
        f.tick(50);
        assert!(f.someip.take_sent_events().is_empty());
        assert_eq!(f.sim.get_speed_kmh(), 0.0);
    }

    #[test]
    fn dead_zone_snaps_to_max() {
        let f = fixture(false);
        f.adc.value.set(3920);
        f.tick(300);
        assert!(!f.someip.take_sent_events().is_empty());
        assert_eq!(f.sim.get_speed_kmh(), f.config.max_speed_kmh);
    }

    #[test]
    fn no_event_when_speed_stable() {
        let f = fixture(false);
        f.adc.value.set(2048);
        f.tick(300);
        assert!(!f.someip.take_sent_events().is_empty());
        f.tick(100);
        assert!(f.someip.take_sent_events().is_empty());
    }

    #[test]
    fn noisy_adc_does_not_trigger_events() {
        let f = fixture(false);
        *f.adc.reader.borrow_mut() =
            Some(Box::new(|tick| 900 + if tick % 2 == 0 { -190 } else { 190 }));
        f.tick(300);
        assert!(!f.someip.take_sent_events().is_empty());
        let settled = f.sim.get_speed_kmh();
        f.tick(200);
        assert!(f.someip.take_sent_events().is_empty());
        assert_eq!(f.sim.get_speed_kmh(), settled);
    }

    #[test]
    fn get_speed_method() {
        let f = fixture(false);
        f.adc.value.set(2048);
        f.tick(300);
        let request = SomeIpMessage {
            service_id: f.config.service_id,
            method_id: f.config.get_speed_method,
            ..Default::default()
        };
        let response = f.someip.call(f.config.service_id, f.config.get_speed_method, &request);
        assert_eq!(response.message_type, 0x80);
        assert_eq!(response.return_code, 0x00);
        assert_eq!(response.payload.len(), 4);
        assert_eq!(SpeedSimulator::deserialize_float(&response.payload), f.sim.get_speed_kmh());
    }

    #[test]
    fn publishes_to_signal_bus() {
        let f = fixture(true);
        f.adc.value.set(2048);
        f.tick(300);
        assert!(f.signal_bus.unwrap().publications(f.config.signal_speed) >= 1);
        assert!(f.sim.get_speed_kmh() > 0.0);
    }

    #[test]
    fn speed_zero_when_not_in_run_mode() {
        let f = fixture(false);
        f.adc.value.set(4095);
        f.tick(300);
        assert!(f.sim.get_speed_kmh() > 0.0);
        f.someip.take_sent_events();
        f.sim.on_mode_changed(VehicleMode::Run, VehicleMode::Accessory);
        assert!(!f.someip.take_sent_events().is_empty());
        assert_eq!(f.sim.get_speed_kmh(), 0.0);
        f.tick(50);
        assert!(f.someip.take_sent_events().is_empty());
        assert_eq!(f.sim.get_speed_kmh(), 0.0);
    }

    #[test]
    fn speed_resumes_when_back_in_run() {
        let f = fixture(false);
        f.adc.value.set(4095);
        f.tick(300);
        assert!(f.sim.get_speed_kmh() > 0.0);
        f.sim.on_mode_changed(VehicleMode::Run, VehicleMode::Accessory);
        assert_eq!(f.sim.get_speed_kmh(), 0.0);
        f.sim.on_mode_changed(VehicleMode::Accessory, VehicleMode::Run);
        f.tick(300);
        assert!(f.sim.get_speed_kmh() > 0.0);
    }

    #[test]
    fn off_mode_speed_is_zero() {
        let f = fixture(false);
        f.adc.value.set(2048);
        f.sim.on_mode_changed(VehicleMode::Run, VehicleMode::Off);
        f.tick(50);
        assert_eq!(f.sim.get_speed_kmh(), 0.0);
    }

    #[test]
    fn crank_mode_speed_is_zero() {
        let f = fixture(false);
        f.adc.value.set(2048);
        f.sim.on_mode_changed(VehicleMode::Run, VehicleMode::Crank);
        f.tick(50);
        assert_eq!(f.sim.get_speed_kmh(), 0.0);
    }

    #[test]
    fn adc_failure_holds_last_speed() {
        let f = fixture(false);
        f.adc.value.set(4095);
        f.tick(300);
        let speed_before = f.sim.get_speed_kmh();
        assert!(speed_before > 0.0);
        f.someip.take_sent_events();
        f.adc.value.set(-1);
        f.tick(50);
        assert!(f.someip.take_sent_events().is_empty());
        assert_eq!(
            f.sim.get_speed_kmh(),
            speed_before,
            "ADC failure must hold last known speed, not drive to zero"
        );
    }

    #[test]
    fn the_setter_overrides_and_resumes_the_adc() {
        let f = fixture(false);
        let set = |value: f32| SomeIpMessage {
            service_id: f.config.service_id,
            method_id: f.config.set_speed_method,
            payload: SpeedSimulator::serialize_float(value),
            ..Default::default()
        };
        assert_eq!(
            f.someip.call(f.config.service_id, f.config.set_speed_method, &set(42.0)).return_code,
            0
        );
        f.tick(1);
        assert_eq!(f.sim.get_speed_kmh(), 40.0, "quantized to a step");
        assert_eq!(
            f.someip.call(f.config.service_id, f.config.set_speed_method, &set(-1.0)).return_code,
            0
        );
        f.adc.value.set(0);
        f.tick(1);
        assert_eq!(f.sim.get_speed_kmh(), 0.0);
        let short = SomeIpMessage { payload: alloc::vec![1, 2], ..set(0.0) };
        assert_eq!(
            f.someip.call(f.config.service_id, f.config.set_speed_method, &short).return_code,
            1
        );
    }
}
