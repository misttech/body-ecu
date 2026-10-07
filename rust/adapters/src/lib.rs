// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Port of `libs/adapters/zephyr` and `libs/adapters/system`: the ports implemented on
//! Zephyr through the application's C shim ([`zephyr`]), and the domain modules wrapped as
//! OpenBSW lifecycle components, with the SOME/IP server and the diagnostic transports
//! ([`system`]).
//!
//! The systems are `Sync` as the C++ ones are: on the MCU the SOME/IP dispatch and the
//! work queue callbacks never run at once against the same state (the C++ uses no-op locks
//! there, `SomeIpSystem.h`'s `PlatformMutex`), and the lifecycle transitions run one at
//! a time on the sysadmin context.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

pub mod system;
pub mod zephyr;
