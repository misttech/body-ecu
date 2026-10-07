// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;
use core::ptr;

use crate::heap::Heap;

/// Allocates `count` objects of `size` bytes each, zeroed. Returns null when
/// `count * size` is 0 or overflows, or the heap has no room.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn calloc(count: usize, size: usize) -> *mut c_void {
    let Some(total) = count.checked_mul(size) else {
        #[cfg(feature = "forkpoint-coverage")]
        forkpoint::assert_sometimes!(true, "calloc: returns NULL when count * size overflows");
        return ptr::null_mut();
    };
    let Some(p) = Heap::current().with(|h| h.alloc(total)) else {
        return ptr::null_mut();
    };
    // SAFETY: `p` holds at least `total` bytes.
    unsafe { p.write_bytes(0, total) };
    p.as_ptr().cast()
}
