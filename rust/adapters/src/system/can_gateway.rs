// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `CanGatewaySystem`: the CAN gateway as a lifecycle component.

use body_ecu_can_gateway::{CanGateway, ServiceMapping};
use body_ecu_ports::{CanBus, SomeIpService};
use openbsw_lifecycle::{ComponentBase, LifecycleComponent};

/// The CAN gateway system.
pub struct CanGatewaySystem {
    base: ComponentBase,
    gateway: CanGateway,
}

// SAFETY: see the crate documentation.
unsafe impl Sync for CanGatewaySystem {}

impl CanGatewaySystem {
    /// A system between `can` and `someip`.
    pub const fn new(can: &'static dyn CanBus, someip: &'static dyn SomeIpService) -> Self {
        Self { base: ComponentBase::new(), gateway: CanGateway::new(can, someip) }
    }

    /// Add `mapping`.
    pub fn add_mapping(&self, mapping: ServiceMapping) {
        self.gateway.add_mapping(mapping);
    }

    /// The gateway.
    pub fn gateway(&self) -> &CanGateway {
        &self.gateway
    }
}

impl LifecycleComponent for CanGatewaySystem {
    fn base(&self) -> &ComponentBase {
        &self.base
    }

    fn init(&'static self) {
        self.transition_done();
    }

    fn run(&'static self) {
        self.gateway.start();
        self.transition_done();
    }

    fn shutdown(&'static self) {
        self.gateway.stop();
        self.transition_done();
    }
}
