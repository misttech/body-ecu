// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Port of `libs/body/ignition`: the ignition button. In Accessory a press cranks for a
//! while and then runs; in Run a press returns to Accessory, unless the vehicle moves.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::boxed::Box;
use core::cell::Cell;

use body_ecu_ports::{
    ButtonInput, INVALID_TIMER_ID, SignalBus, SignalValue, TimerService, VehicleMode, printk,
};
use body_ecu_vehicle_mode::VehicleModeManager;

/// The controller's configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IgnitionConfig {
    /// How long cranking takes.
    pub crank_duration_ms: u32,
    /// The vehicle's speed.
    pub signal_speed: &'static str,
}

impl IgnitionConfig {
    /// The defaults, as a constant for a static.
    pub const DEFAULT: Self = Self { crank_duration_ms: 1000, signal_speed: "Vehicle.Speed" };
}

impl Default for IgnitionConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The ignition controller (`IgnitionController`).
pub struct IgnitionController {
    button: &'static dyn ButtonInput,
    mode_manager: &'static VehicleModeManager,
    timer: &'static dyn TimerService,
    config: IgnitionConfig,
    signal_bus: Option<&'static dyn SignalBus>,
    cranking: Cell<bool>,
}

impl IgnitionController {
    /// A controller on `button`, driving `mode_manager`, cranking on `timer`.
    pub const fn new(
        button: &'static dyn ButtonInput,
        mode_manager: &'static VehicleModeManager,
        timer: &'static dyn TimerService,
        config: IgnitionConfig,
        signal_bus: Option<&'static dyn SignalBus>,
    ) -> Self {
        Self { button, mode_manager, timer, config, signal_bus, cranking: Cell::new(false) }
    }

    /// Take the button, and leave Off for Accessory.
    pub fn init(&'static self) {
        self.button.on_press(Box::new(move || self.on_button_press()));

        if self.mode_manager.get_mode() == VehicleMode::Off {
            printk!("[ignition] Init: Off -> Accessory\n");
            self.mode_manager.set_mode(VehicleMode::Accessory);
        }
    }

    fn on_button_press(&'static self) {
        let mode = self.mode_manager.get_mode();
        forkpoint::assert_always!(
            !self.cranking.get() || mode == VehicleMode::Crank,
            "ignition: cranking only while the mode is Crank"
        );
        forkpoint::assert_sometimes!(
            mode == VehicleMode::Run,
            "ignition: the button is pressed in Run"
        );
        forkpoint::lifecycle::send_event("ignition.button", mode as u64);

        if self.cranking.get() {
            printk!("[ignition] Button ignored (cranking)\n");
            return;
        }

        match mode {
            VehicleMode::Accessory => {
                printk!("[ignition] Accessory -> Crank\n");
                if self.mode_manager.set_mode(VehicleMode::Crank) {
                    self.cranking.set(true);
                    let tid = self.timer.start_one_shot(
                        self.config.crank_duration_ms,
                        Box::new(move || {
                            printk!("[ignition] Crank -> Run\n");
                            self.mode_manager.set_mode(VehicleMode::Run);
                            self.cranking.set(false);
                        }),
                    );
                    if tid == INVALID_TIMER_ID {
                        printk!(
                            "[ignition] FAULT: crank timer allocation failed, reverting to Accessory\n"
                        );
                        self.cranking.set(false);
                        self.mode_manager.set_mode(VehicleMode::Accessory);
                    }
                }
            }
            VehicleMode::Run => {
                let speed = self.get_current_speed();
                if speed > 0.0 {
                    printk!("[ignition] Cannot turn off: speed={} km/h\n", speed as i32);
                } else {
                    printk!("[ignition] Run -> Accessory\n");
                    self.mode_manager.set_mode(VehicleMode::Accessory);
                }
            }
            _ => {
                printk!("[ignition] Button ignored in mode {}\n", mode as u8);
            }
        }
    }

    fn get_current_speed(&self) -> f32 {
        let Some(signal_bus) = self.signal_bus else { return 0.0 };
        match signal_bus.get(self.config.signal_speed) {
            Some(SignalValue::Float(speed)) => speed,
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use body_ecu_ports::mock::{
        MockButtonInput, MockSignalBus, MockSomeIpService, MockTimerService, leak,
    };
    use body_ecu_vehicle_mode::VehicleModeConfig;

    use super::*;

    struct Fixture {
        button: &'static MockButtonInput,
        timer: &'static MockTimerService,
        signal_bus: &'static MockSignalBus,
        mode_manager: &'static VehicleModeManager,
        config: IgnitionConfig,
    }

    fn fixture() -> Fixture {
        let button = leak(MockButtonInput::new());
        let someip = leak(MockSomeIpService::new());
        let timer = leak(MockTimerService::new());
        let signal_bus = leak(MockSignalBus::new());
        signal_bus.set("Vehicle.Speed", SignalValue::Float(0.0));
        let mode_manager = leak(VehicleModeManager::new(someip, VehicleModeConfig::default()));
        mode_manager.init();
        Fixture { button, timer, signal_bus, mode_manager, config: IgnitionConfig::default() }
    }

    impl Fixture {
        fn init_controller(&self) -> &'static IgnitionController {
            let controller = leak(IgnitionController::new(
                self.button,
                self.mode_manager,
                self.timer,
                self.config.clone(),
                Some(self.signal_bus),
            ));
            controller.init();
            controller
        }

        fn press_button(&self) {
            assert!(self.button.has_callback());
            self.button.press();
        }

        fn set_speed(&self, speed: f32) {
            self.signal_bus.set("Vehicle.Speed", SignalValue::Float(speed));
        }

        fn fire_crank_timer(&self) {
            self.timer.fire_last_one_shot();
        }
    }

    #[test]
    fn init_transitions_to_accessory() {
        let f = fixture();
        f.init_controller();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Accessory);
    }

    #[test]
    fn button_in_accessory_starts_cranking() {
        let f = fixture();
        f.init_controller();
        f.press_button();
        assert_eq!(f.timer.one_shots.borrow()[0].0, f.config.crank_duration_ms);
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Crank);
    }

    #[test]
    fn crank_timer_transitions_to_run() {
        let f = fixture();
        f.init_controller();
        f.press_button();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Crank);
        f.fire_crank_timer();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Run);
    }

    #[test]
    fn button_in_run_goes_to_accessory() {
        let f = fixture();
        f.init_controller();
        f.press_button();
        f.fire_crank_timer();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Run);
        f.press_button();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Accessory);
    }

    #[test]
    fn button_in_run_blocked_by_speed() {
        let f = fixture();
        f.init_controller();
        f.press_button();
        f.fire_crank_timer();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Run);
        f.set_speed(50.0);
        f.press_button();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Run);
    }

    #[test]
    fn button_ignored_during_crank() {
        let f = fixture();
        f.init_controller();
        f.press_button();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Crank);
        f.press_button();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Crank);
    }

    #[test]
    fn full_cycle_accessory_run_accessory() {
        let f = fixture();
        f.init_controller();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Accessory);
        f.press_button();
        f.fire_crank_timer();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Run);
        f.press_button();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Accessory);
        f.press_button();
        f.fire_crank_timer();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Run);
    }

    #[test]
    fn speed_zero_allows_turn_off() {
        let f = fixture();
        f.init_controller();
        f.press_button();
        f.fire_crank_timer();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Run);
        f.set_speed(50.0);
        f.press_button();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Run);
        f.set_speed(0.0);
        f.press_button();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Accessory);
    }

    #[test]
    fn timer_exhaustion_reverts_to_accessory() {
        let f = fixture();
        f.timer.next_id.set(INVALID_TIMER_ID);
        f.init_controller();
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Accessory);
        f.press_button();
        // Timer allocation failed: the controller must revert to Accessory, not stay
        // stuck in Crank indefinitely.
        assert_eq!(f.mode_manager.get_mode(), VehicleMode::Accessory);
    }
}
