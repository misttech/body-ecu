// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `VehicleModeSystem`: the vehicle mode manager as a lifecycle component; shutting down switches the vehicle off.

use body_ecu_ports::{SomeIpService, VehicleMode};
use body_ecu_vehicle_mode::{VehicleModeConfig, VehicleModeManager};
use openbsw_lifecycle::{ComponentBase, LifecycleComponent};

/// The vehicle mode system.
pub struct VehicleModeSystem {
    base: ComponentBase,
    manager: VehicleModeManager,
}

// SAFETY: see the crate documentation.
unsafe impl Sync for VehicleModeSystem {}

impl VehicleModeSystem {
    /// A system serving over `someip`.
    pub const fn new(someip: &'static dyn SomeIpService, config: VehicleModeConfig) -> Self {
        Self { base: ComponentBase::new(), manager: VehicleModeManager::new(someip, config) }
    }

    /// The manager.
    pub const fn manager(&self) -> &VehicleModeManager {
        &self.manager
    }
}

impl LifecycleComponent for VehicleModeSystem {
    fn base(&self) -> &ComponentBase {
        &self.base
    }

    fn init(&'static self) {
        self.manager.init();
        self.transition_done();
    }

    fn run(&'static self) {
        self.transition_done();
    }

    fn shutdown(&'static self) {
        self.manager.set_mode(VehicleMode::Off);
        self.transition_done();
    }
}
