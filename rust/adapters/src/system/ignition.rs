// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `IgnitionSystem`: the ignition controller as a lifecycle component.

use body_ecu_ignition::{IgnitionConfig, IgnitionController};
use body_ecu_ports::{ButtonInput, SignalBus, TimerService};
use body_ecu_vehicle_mode::VehicleModeManager;
use openbsw_lifecycle::{ComponentBase, LifecycleComponent};

/// The ignition system.
pub struct IgnitionSystem {
    base: ComponentBase,
    controller: IgnitionController,
}

// SAFETY: see the crate documentation.
unsafe impl Sync for IgnitionSystem {}

impl IgnitionSystem {
    /// A system on `button`, driving `mode_manager` (the vehicle mode system's), cranking
    /// on `timer`.
    pub const fn new(
        button: &'static dyn ButtonInput,
        mode_manager: &'static VehicleModeManager,
        timer: &'static dyn TimerService,
        config: IgnitionConfig,
        signal_bus: Option<&'static dyn SignalBus>,
    ) -> Self {
        Self {
            base: ComponentBase::new(),
            controller: IgnitionController::new(button, mode_manager, timer, config, signal_bus),
        }
    }
}

impl LifecycleComponent for IgnitionSystem {
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
        self.transition_done();
    }
}
