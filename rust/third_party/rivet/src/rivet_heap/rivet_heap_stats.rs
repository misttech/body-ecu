// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use super::heap_or_current;
use crate::heap::{HeapStats, rivet_heap};

/// Writes `heap`'s counters, or the running domain's heap's when `heap` is
/// null, to `stats`.
///
/// # Safety
///
/// `heap` is null or a heap from `rivet_heap_init`, and `stats` is writable
/// for a `struct rivet_heap_stats`.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn rivet_heap_stats(heap: *mut rivet_heap, stats: *mut HeapStats) {
    // SAFETY: the caller guarantees both pointers.
    unsafe { stats.write(heap_or_current(heap).stats()) };
}
