// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_int, c_void};

use crate::heap::DEFAULT_HEAP;

/// Gives the `size` bytes at `base` to the default heap, the one `malloc`
/// uses outside any domain. Returns 0, or -1 when the default heap already
/// has four regions or the memory is too small to use.
///
/// # Safety
///
/// `base` is writable for `size` bytes that nothing else uses for as long as
/// the program runs.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn rivet_heap_add(base: *mut c_void, size: usize) -> c_int {
    // SAFETY: the caller gives the memory to the heap.
    if DEFAULT_HEAP.with(|h| unsafe { h.add_region(base.cast(), size) }) { 0 } else { -1 }
}
