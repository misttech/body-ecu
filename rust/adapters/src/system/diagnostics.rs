// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `DiagnosticsSystem`: the UDS handler behind every diagnostic transport, as a lifecycle
//! component. The trouble code store is a static of the application's, which the handler
//! refers to; the C++ holds it as a member.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::RefCell;

use body_ecu_diagnostics::{DtcStore, TransportLayer, UdsServiceHandler};
use body_ecu_ports::DiagDataProvider;
use openbsw_lifecycle::{ComponentBase, LifecycleComponent};

/// The diagnostics system.
pub struct DiagnosticsSystem {
    base: ComponentBase,
    dtc_store: &'static DtcStore,
    handler: UdsServiceHandler,
    transports: RefCell<Vec<&'static dyn TransportLayer>>,
}

// SAFETY: see the crate documentation.
unsafe impl Sync for DiagnosticsSystem {}

impl DiagnosticsSystem {
    /// A system answering with the codes in `dtc_store`.
    pub const fn new(dtc_store: &'static DtcStore) -> Self {
        Self {
            base: ComponentBase::new(),
            dtc_store,
            handler: UdsServiceHandler::new(dtc_store),
            transports: RefCell::new(Vec::new()),
        }
    }

    /// Answer requests arriving on `transport`.
    pub fn add_transport(&self, transport: &'static dyn TransportLayer) {
        self.transports.borrow_mut().push(transport);
    }

    /// Ask `provider` for data identifiers too.
    pub fn add_provider(&self, provider: &'static dyn DiagDataProvider) {
        self.handler.add_provider(provider);
    }

    /// The handler.
    pub fn handler(&self) -> &UdsServiceHandler {
        &self.handler
    }

    /// The trouble code store.
    pub fn dtc_store(&self) -> &DtcStore {
        self.dtc_store
    }
}

impl LifecycleComponent for DiagnosticsSystem {
    fn base(&self) -> &ComponentBase {
        &self.base
    }

    fn init(&'static self) {
        for transport in self.transports.borrow().iter() {
            transport.set_request_handler(Box::new(move |req| self.handler.handle_request(req)));
        }
        self.transition_done();
    }

    fn run(&'static self) {
        self.transition_done();
    }

    fn shutdown(&'static self) {
        self.transition_done();
    }
}
