// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `GpioAdapter`: the board's LEDs as a GPIO port. The shim owns the `gpio_dt_spec`s of
//! the `led0`, `led1`, and `led2` aliases, so the active level is the Device Tree's. On
//! the NUCLEO-H753ZI, pins 0 to 2 are the on-board LEDs (green LD1, yellow LD2, red LD3).

use body_ecu_ports::{GpioPort, printk};

/// The LEDs as a GPIO port.
pub struct GpioAdapter;

impl GpioAdapter {
    /// The port over the shim's LEDs.
    pub const fn new() -> Self {
        Self
    }

    /// Configure every LED as an inactive output; `false` when one is not ready or
    /// refuses.
    pub fn configure(&self) -> bool {
        match body_ecu_ffi::led_configure() {
            Ok(()) => true,
            Err((index, error)) => {
                let pin = body_ecu_ffi::led_port_and_pin(index).1;
                if error == -19 {
                    printk!("[gpio] Port not ready for LED {} (pin {})\n", index, pin);
                } else {
                    printk!("[gpio] Failed to configure LED {} (pin {}): {}\n", index, pin, error);
                }
                false
            }
        }
    }
}

impl Default for GpioAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl GpioPort for GpioAdapter {
    fn write(&self, pin: u32, value: bool) {
        if pin >= body_ecu_ffi::led_count() {
            return;
        }
        body_ecu_ffi::led_set(pin, value);
    }

    fn read(&self, pin: u32) -> bool {
        if pin >= body_ecu_ffi::led_count() {
            return false;
        }
        body_ecu_ffi::led_get(pin)
    }
}
