// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `LightingSystem`: the lighting controller as a lifecycle component; shutting down turns every light off.

use body_ecu_lighting::{LIGHT_COUNT, LightingConfig, LightingController};
use body_ecu_ports::{GpioPort, SomeIpService};
use openbsw_lifecycle::{ComponentBase, LifecycleComponent};

/// The lighting system.
pub struct LightingSystem {
    base: ComponentBase,
    controller: LightingController,
}

// SAFETY: see the crate documentation.
unsafe impl Sync for LightingSystem {}

impl LightingSystem {
    /// A system driving `gpio` and serving over `someip`.
    pub const fn new(
        gpio: &'static dyn GpioPort,
        someip: &'static dyn SomeIpService,
        config: LightingConfig,
    ) -> Self {
        Self {
            base: ComponentBase::new(),
            controller: LightingController::new(gpio, someip, config),
        }
    }

    /// The controller.
    pub fn controller(&self) -> &LightingController {
        &self.controller
    }
}

impl LifecycleComponent for LightingSystem {
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
        for i in 0..LIGHT_COUNT {
            self.controller.set_light_index(i, false);
        }
        self.transition_done();
    }
}
