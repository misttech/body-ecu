// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `DoIpTransport` as the Zephyr build has it: a stub. The TCP server of the POSIX build
//! is not available on Zephyr, so the transport only reports itself on init.

use core::cell::{Cell, RefCell};

use body_ecu_diagnostics::{DiagRequestHandler, TransportLayer};
use body_ecu_ports::printk;
use openbsw_lifecycle::{ComponentBase, LifecycleComponent};

/// The DoIP transport.
pub struct DoIpTransport {
    base: ComponentBase,
    handler: RefCell<Option<DiagRequestHandler>>,
    port: u16,
    running: Cell<bool>,
}

// SAFETY: see the crate documentation.
unsafe impl Sync for DoIpTransport {}

impl DoIpTransport {
    /// The TCP port a DoIP server listens on.
    pub const DEFAULT_PORT: u16 = 13400;
    /// This ECU's logical address.
    pub const LOGICAL_ADDRESS: u16 = 0x0E80;

    /// A transport on `port`.
    pub const fn new(port: u16) -> Self {
        Self {
            base: ComponentBase::new(),
            handler: RefCell::new(None),
            port,
            running: Cell::new(false),
        }
    }

    /// Whether a tester is connected: never, on Zephyr.
    pub fn is_connected(&self) -> bool {
        false
    }

    /// Whether the server listens: never, on Zephyr.
    pub fn is_listening(&self) -> bool {
        false
    }

    /// The port.
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl TransportLayer for DoIpTransport {
    fn set_request_handler(&self, handler: DiagRequestHandler) {
        *self.handler.borrow_mut() = Some(handler);
    }

    fn send_response(&self, _response: &[u8]) {}
}

impl LifecycleComponent for DoIpTransport {
    fn base(&self) -> &ComponentBase {
        &self.base
    }

    fn init(&'static self) {
        self.running.set(true);
        printk!("[DoIP] Stub on Zephyr (TCP server not available)\n");
        self.transition_done();
    }

    fn run(&'static self) {}

    fn shutdown(&'static self) {
        self.running.set(false);
        *self.handler.borrow_mut() = None;
        self.transition_done();
    }
}
