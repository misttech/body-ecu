// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;
use core::ptr::{self, NonNull};

use crate::heap::Heap;

/// Allocates `size` bytes aligned to `alignment`. Returns null when
/// `alignment` is not a power of two, `size` is 0, or the heap has no room.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn aligned_alloc(alignment: usize, size: usize) -> *mut c_void {
    if !alignment.is_power_of_two() {
        #[cfg(feature = "forkpoint-coverage")]
        forkpoint::assert_sometimes!(
            true,
            "aligned_alloc: rejects an alignment that is not a power of two"
        );
        return ptr::null_mut();
    }
    Heap::current()
        .with(|h| h.memalign(alignment, size))
        .map_or(ptr::null_mut(), NonNull::as_ptr)
        .cast()
}
