// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;
use core::ptr::{self, NonNull};

use super::heap_or_current;
use crate::heap::rivet_heap;

/// Allocates `size` bytes aligned to `align` from `heap`, or from the running
/// domain's heap when `heap` is null. `align` is a power of two, or 0 for
/// the alignment `malloc` gives. Returns null when `size` is 0, `align` is
/// not a power of two, or the heap has no room.
///
/// # Safety
///
/// `heap` is null or a heap from `rivet_heap_init`.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn rivet_heap_alloc(
    heap: *mut rivet_heap,
    size: usize,
    align: usize,
) -> *mut c_void {
    if align != 0 && !align.is_power_of_two() {
        return ptr::null_mut();
    }
    // SAFETY: the caller guarantees `heap`.
    let heap = unsafe { heap_or_current(heap) };
    heap.with(|h| h.memalign(align, size)).map_or(ptr::null_mut(), NonNull::as_ptr).cast()
}
