// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `DoCanTransport`: diagnostics over classic CAN single frames, request on 0x600,
//! response on 0x601. Multi-frame messages are left to OpenBSW's ISO-TP in the full build.

use alloc::boxed::Box;
use core::cell::{Cell, RefCell};

use body_ecu_diagnostics::{DiagRequestHandler, TransportLayer};
use body_ecu_ports::{CanBus, CanFrame};
use openbsw_lifecycle::{ComponentBase, LifecycleComponent};

/// The DoCAN transport.
pub struct DoCanTransport {
    base: ComponentBase,
    can: &'static dyn CanBus,
    handler: RefCell<Option<DiagRequestHandler>>,
    rx_registered: Cell<bool>,
}

// SAFETY: see the crate documentation.
unsafe impl Sync for DoCanTransport {}

impl DoCanTransport {
    /// The identifier requests arrive on.
    pub const DIAG_RX_CAN_ID: u32 = 0x600;
    /// The identifier responses go out on.
    pub const DIAG_TX_CAN_ID: u32 = 0x601;

    /// A transport on `can`.
    pub const fn new(can: &'static dyn CanBus) -> Self {
        Self {
            base: ComponentBase::new(),
            can,
            handler: RefCell::new(None),
            rx_registered: Cell::new(false),
        }
    }

    /// Register the receive callback; the C++ does it in `init`, which tests call.
    pub fn init_rx(&'static self) {
        if !self.rx_registered.get() {
            self.can.add_rx_callback(Box::new(move |frame| self.on_can_frame(frame)));
            self.rx_registered.set(true);
        }
    }

    fn on_can_frame(&self, frame: &CanFrame) {
        if frame.id != Self::DIAG_RX_CAN_ID {
            return;
        }
        let handler = self.handler.borrow();
        let Some(handler) = handler.as_ref() else { return };

        // Single-frame: PCI byte 0 contains length
        let sf_dl = usize::from(frame.data[0] & 0x0F);
        if sf_dl == 0 || sf_dl > 7 {
            return;
        }

        let response = handler(&frame.data[1..1 + sf_dl]);
        self.send_response(&response);
    }
}

impl TransportLayer for DoCanTransport {
    fn set_request_handler(&self, handler: DiagRequestHandler) {
        *self.handler.borrow_mut() = Some(handler);
    }

    /// A single-frame response for payloads of at most 7 bytes (SF_DL in byte 0).
    fn send_response(&self, response: &[u8]) {
        let mut frame = CanFrame { id: Self::DIAG_TX_CAN_ID, ..CanFrame::default() };
        if response.len() <= 7 {
            frame.dlc = (response.len() + 1) as u8;
            frame.data[0] = response.len() as u8; // SF PCI
            frame.data[1..1 + response.len()].copy_from_slice(response);
            self.can.send(&frame);
        }
        // Multi-frame responses delegated to OpenBSW's ISO-TP in full build.
    }
}

impl LifecycleComponent for DoCanTransport {
    fn base(&self) -> &ComponentBase {
        &self.base
    }

    fn init(&'static self) {
        self.init_rx();
        self.transition_done();
    }

    fn run(&'static self) {}

    fn shutdown(&'static self) {
        *self.handler.borrow_mut() = None;
        self.transition_done();
    }
}
