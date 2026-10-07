// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The ports on Zephyr (`libs/adapters/zephyr`), through `body_ecu_ffi`.

mod button;
mod gpio;
mod signal_bus;
mod timer;

#[cfg(feature = "adc")]
mod adc;
#[cfg(feature = "can")]
mod can;

#[cfg(feature = "adc")]
pub use adc::AdcAdapter;
pub use button::ButtonAdapter;
#[cfg(feature = "can")]
pub use can::CanAdapter;
pub use gpio::GpioAdapter;
pub use signal_bus::LocalSignalBus;
pub use timer::{MAX_TIMERS, ZephyrTimerService};
