// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `CanAdapter`: the CAN FD controller, through the shim. Not built for the Forkpoint
//! image, whose board configuration leaves CAN out, as the C++ build does; kept for
//! parity with `libs/adapters/zephyr/CanAdapter.cpp`.

use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use body_ecu_ports::{CanBus, CanFrame, CanRxCallback};

/// The CAN controller.
pub struct CanAdapter {
    rx_callbacks: RefCell<Vec<CanRxCallback>>,
    filter_id: Cell<i32>,
}

// SAFETY: callbacks are registered during init and the list is frozen by
// `start_receiving`, which the receive path then reads without a lock, as the C++ does.
unsafe impl Sync for CanAdapter {}

impl CanAdapter {
    /// The adapter over the shim's chosen CAN controller.
    pub const fn new() -> Self {
        Self { rx_callbacks: RefCell::new(Vec::new()), filter_id: Cell::new(-1) }
    }

    /// A frame arrived: hand it to every callback (the C++ `rxDispatch`).
    pub fn dispatch(&self, frame: &CanFrame) {
        for callback in self.rx_callbacks.borrow().iter() {
            callback(frame);
        }
    }

    /// Install the receive filter after every callback is registered.
    pub fn start_receiving(&self) {
        if self.filter_id.get() >= 0 {
            return;
        }
        self.filter_id.set(0);
    }
}

impl Default for CanAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CanBus for CanAdapter {
    fn send(&self, _frame: &CanFrame) -> bool {
        false
    }

    fn add_rx_callback(&self, callback: CanRxCallback) {
        if self.filter_id.get() >= 0 {
            return;
        }
        self.rx_callbacks.borrow_mut().push(callback);
    }
}
