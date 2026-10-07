// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;
use core::ptr::{self, NonNull};

use crate::heap::Heap;

/// Allocates `size` bytes, aligned for any object, from the running domain's
/// heap. Returns null when `size` is 0 or the heap has no room.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn malloc(size: usize) -> *mut c_void {
    Heap::current().with(|h| h.alloc(size)).map_or(ptr::null_mut(), NonNull::as_ptr).cast()
}
