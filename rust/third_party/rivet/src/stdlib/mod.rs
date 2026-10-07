// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `stdlib.h`: `abort`, the number conversions, the absolute values, the
//! sorted-array functions, and with the `cmpctmalloc` feature the allocation
//! functions, over `crate::heap`. Without it, the firmware's allocator
//! provides them.

// Sizes and counts here come from C callers: arithmetic on them is checked,
// or wraps on purpose, so no argument can overflow into a wrong size.
#![deny(clippy::arithmetic_side_effects)]

mod abort;
mod abs;
#[cfg(feature = "cmpctmalloc")]
mod aligned_alloc;
mod atoi;
mod bsearch;
#[cfg(feature = "cmpctmalloc")]
mod calloc;
#[cfg(feature = "cmpctmalloc")]
mod free;
#[cfg(feature = "cmpctmalloc")]
mod free_sized;
mod labs;
mod llabs;
#[cfg(feature = "cmpctmalloc")]
mod malloc;
mod qsort;
#[cfg(feature = "cmpctmalloc")]
mod realloc;
mod strtol;
mod strtoll;
mod strtoul;
mod strtoull;

pub use abort::abort;
#[cfg(feature = "backtrace")]
pub(crate) use abort::abort_from;
pub use abs::abs;
pub use atoi::atoi;
pub use bsearch::bsearch;
pub use labs::labs;
pub use llabs::llabs;
pub use qsort::qsort;
pub use strtol::strtol;
pub use strtoll::strtoll;
pub use strtoul::strtoul;
pub use strtoull::strtoull;
#[cfg(feature = "cmpctmalloc")]
pub use {
    aligned_alloc::aligned_alloc, calloc::calloc, free::free, free_sized::free_sized,
    malloc::malloc, realloc::realloc,
};
