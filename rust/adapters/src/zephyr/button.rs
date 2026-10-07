// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `ButtonAdapter`: the user button through its GPIO interrupt. The shim takes the
//! interrupt and submits a work item; the system work queue then calls
//! `rust_button_pressed`, which the application routes to [`ButtonAdapter::pressed`].

use core::cell::RefCell;

use body_ecu_ports::{ButtonCallback, ButtonInput, printk};

/// The user button.
pub struct ButtonAdapter {
    callback: RefCell<Option<ButtonCallback>>,
}

// SAFETY: the callback is set during startup and run from the system work queue; the
// C++ adapter keeps the same single `std::function` with no lock.
unsafe impl Sync for ButtonAdapter {}

impl ButtonAdapter {
    /// The adapter over the shim's `sw0` button.
    pub const fn new() -> Self {
        Self { callback: RefCell::new(None) }
    }

    /// Configure the pin and its interrupt; `false` when the port is not ready or the
    /// driver refuses.
    pub fn configure(&self) -> bool {
        match body_ecu_ffi::button_configure() {
            Ok(()) => {
                let (pin, flags) = body_ecu_ffi::button_pin_and_flags();
                printk!("[button] Configured on pin {} (flags=0x{:x})\n", pin, flags);
                true
            }
            Err((0, _)) => {
                printk!("[button] GPIO port not ready\n");
                false
            }
            Err((1, error)) => {
                printk!("[button] Failed to configure pin: {}\n", error);
                false
            }
            Err((_, error)) => {
                printk!("[button] Failed to configure interrupt: {}\n", error);
                false
            }
        }
    }

    /// The button was pressed: run the callback (the C++ `workHandler`).
    pub fn pressed(&self) {
        if let Some(callback) = self.callback.borrow().as_ref() {
            callback();
        }
    }
}

impl Default for ButtonAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ButtonInput for ButtonAdapter {
    /// The last callback registered is the one that runs, as the C++ keeps one.
    fn on_press(&self, callback: ButtonCallback) {
        *self.callback.borrow_mut() = Some(callback);
    }
}
