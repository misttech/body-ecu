// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The memory helpers of Arm's run-time ABI, on bare-metal Arm.
//!
//! On an Arm EABI target the compiler turns copies and fills, in Rust and in C
//! alike, into calls to `__aeabi_memcpy`, `__aeabi_memset`, and their siblings
//! rather than to `memcpy` and `memset`. The compiler runtime that Rust links
//! defines them weakly, over its own generic byte loops, so without these the
//! copies inside rivet, and in C built against it, would bypass rivet's own
//! memory functions. Each helper here forwards to `memcpy`, `memmove`, or
//! `memset`, and, being strong, replaces the runtime's.
//!
//! The `4` and `8` variants promise pointers aligned to 4 or 8 bytes; rivet's
//! functions take any alignment, so the promise is not needed. The helpers
//! return nothing, and `__aeabi_memset` takes its fill byte last.

mod aeabi_memclr;
mod aeabi_memcpy;
mod aeabi_memmove;
mod aeabi_memset;

pub use aeabi_memclr::{__aeabi_memclr, __aeabi_memclr4, __aeabi_memclr8};
pub use aeabi_memcpy::{__aeabi_memcpy, __aeabi_memcpy4, __aeabi_memcpy8};
pub use aeabi_memmove::{__aeabi_memmove, __aeabi_memmove4, __aeabi_memmove8};
pub use aeabi_memset::{__aeabi_memset, __aeabi_memset4, __aeabi_memset8};
