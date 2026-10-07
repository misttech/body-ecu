// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `DoorLockSystem`: the door lock controller as a lifecycle component; shutting down unlocks.

use body_ecu_door_lock::{DoorLockConfig, DoorLockController};
use body_ecu_ports::{ButtonInput, GpioPort, SignalBus, SomeIpService};
use openbsw_lifecycle::{ComponentBase, LifecycleComponent};

/// The door lock system.
pub struct DoorLockSystem {
    base: ComponentBase,
    controller: DoorLockController,
}

// SAFETY: see the crate documentation.
unsafe impl Sync for DoorLockSystem {}

impl DoorLockSystem {
    /// A system driving `gpio`, toggled by `button`, serving over `someip`.
    pub const fn new(
        gpio: &'static dyn GpioPort,
        button: &'static dyn ButtonInput,
        someip: &'static dyn SomeIpService,
        config: DoorLockConfig,
        signal_bus: Option<&'static dyn SignalBus>,
    ) -> Self {
        Self {
            base: ComponentBase::new(),
            controller: DoorLockController::new(gpio, button, someip, config, signal_bus),
        }
    }

    /// The controller.
    pub fn controller(&self) -> &DoorLockController {
        &self.controller
    }
}

impl LifecycleComponent for DoorLockSystem {
    fn base(&self) -> &ComponentBase {
        &self.base
    }

    fn init(&'static self) {
        self.controller.init();
        self.transition_done();
    }

    fn run(&'static self) {
        self.transition_done();
    }

    fn shutdown(&'static self) {
        self.controller.unlock();
        self.transition_done();
    }
}
