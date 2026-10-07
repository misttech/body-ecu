// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;
use core::ptr::NonNull;

use crate::heap::Heap;

/// Frees an allocation of the running domain's heap. A null `ptr` does
/// nothing, and so does any pointer that is not the start of a live
/// allocation of that heap, which would otherwise corrupt it.
///
/// # Safety
///
/// `ptr` is null or an allocation of the running domain's heap not yet freed.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    let Some(ptr) = NonNull::new(ptr.cast::<u8>()) else {
        return;
    };
    Heap::current().with(|h| {
        let owned = h.owns(ptr.as_ptr());
        #[cfg(feature = "forkpoint")]
        forkpoint::assert_always_or_unreachable!(
            owned,
            "free: ptr is a live allocation of the running domain's heap"
        );
        if owned {
            // SAFETY: `owns` found a live allocation of this heap.
            unsafe { h.free(ptr) };
        }
    });
}
