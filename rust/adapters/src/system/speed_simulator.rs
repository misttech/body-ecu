// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `SpeedSimulatorSystem`: the speed simulator as a lifecycle component.

use body_ecu_ports::{AdcInput, SignalBus, SomeIpService, TimerService};
use body_ecu_speed_simulator::{SpeedSimulator, SpeedSimulatorConfig};
use openbsw_lifecycle::{ComponentBase, LifecycleComponent};

/// The speed simulator system.
pub struct SpeedSimulatorSystem {
    base: ComponentBase,
    simulator: SpeedSimulator,
}

// SAFETY: see the crate documentation.
unsafe impl Sync for SpeedSimulatorSystem {}

impl SpeedSimulatorSystem {
    /// A system reading `adc`, serving over `someip`, ticking on `timer`.
    pub const fn new(
        adc: &'static dyn AdcInput,
        someip: &'static dyn SomeIpService,
        timer: &'static dyn TimerService,
        config: SpeedSimulatorConfig,
        signal_bus: Option<&'static dyn SignalBus>,
    ) -> Self {
        Self {
            base: ComponentBase::new(),
            simulator: SpeedSimulator::new(adc, someip, timer, config, signal_bus),
        }
    }

    /// The simulator.
    pub fn simulator(&self) -> &SpeedSimulator {
        &self.simulator
    }
}

impl LifecycleComponent for SpeedSimulatorSystem {
    fn base(&self) -> &ComponentBase {
        &self.base
    }

    fn init(&'static self) {
        self.simulator.init();
        self.transition_done();
    }

    fn run(&'static self) {
        self.transition_done();
    }

    fn shutdown(&'static self) {
        self.transition_done();
    }
}
