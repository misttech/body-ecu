// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `rivet_heap.h`: heaps for memory domains, and the default heap `malloc`
//! uses without them. See `crate::heap` for the design.

// Arena sizes here come from C callers: arithmetic on them is checked, or
// wraps on purpose, so no argument can overflow into a wrong bound.
#![deny(clippy::arithmetic_side_effects)]

mod rivet_heap_add;
mod rivet_heap_alloc;
mod rivet_heap_free;
mod rivet_heap_init;
mod rivet_heap_set_hooks;
mod rivet_heap_stats;

pub use rivet_heap_add::rivet_heap_add;
pub use rivet_heap_alloc::rivet_heap_alloc;
pub use rivet_heap_free::rivet_heap_free;
pub use rivet_heap_init::rivet_heap_init;
pub use rivet_heap_set_hooks::rivet_heap_set_hooks;
pub use rivet_heap_stats::rivet_heap_stats;

use crate::heap::{Heap, rivet_heap};

/// `heap`, or the running domain's heap when it is null.
///
/// # Safety
///
/// `heap` is null or a heap from `rivet_heap_init`, which lives as long as
/// the program.
unsafe fn heap_or_current(heap: *mut rivet_heap) -> &'static Heap {
    // SAFETY: the caller guarantees `heap`.
    unsafe { heap.as_ref() }.unwrap_or_else(Heap::current)
}
