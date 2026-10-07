// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `AdcAdapter`: one channel of the board's ADC, through the shim.

use core::cell::Cell;

use body_ecu_ports::{AdcInput, printk};

/// The ADC.
pub struct AdcAdapter {
    configured_channel: Cell<u8>,
    resolution: Cell<u8>,
    configured: Cell<bool>,
}

// SAFETY: configured once at startup, read from the timer work queue afterwards.
unsafe impl Sync for AdcAdapter {}

impl AdcAdapter {
    /// The adapter over the shim's `adc1`.
    pub const fn new() -> Self {
        Self {
            configured_channel: Cell::new(0),
            resolution: Cell::new(12),
            configured: Cell::new(false),
        }
    }

    /// Configure `channel` at `resolution` bits.
    pub fn configure(&self, channel: u8, resolution: u8) -> bool {
        self.configured_channel.set(channel);
        self.resolution.set(resolution);
        let result = body_ecu_ffi::adc_configure(channel, resolution);
        if result == -19 {
            printk!("[adc] Device not ready\n");
            return false;
        }
        if result < 0 {
            printk!("[adc] Channel setup failed: {}\n", result);
            return false;
        }
        self.configured.set(true);
        printk!("[adc] Configured channel {} ({}-bit)\n", channel, resolution);
        true
    }
}

impl Default for AdcAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AdcInput for AdcAdapter {
    fn read(&self, channel: u8) -> i32 {
        if !self.configured.get() || channel != self.configured_channel.get() {
            printk!("[adc] read: not configured or wrong channel {}\n", channel);
            return -1;
        }
        let sample = body_ecu_ffi::adc_read(channel);
        if sample < 0 {
            printk!("[adc] read failed: {}\n", sample);
            return -1;
        }
        sample
    }
}
