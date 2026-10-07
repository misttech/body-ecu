// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;
use core::ptr::NonNull;

use crate::heap::Heap;
use crate::support::cmpctmalloc::CmpctHeap;

/// C23's `free_sized`: `free`, given the size the allocation was requested
/// with. Like `free`, it ignores a pointer that is not the start of a live
/// allocation of the running domain's heap.
///
/// # Safety
///
/// `ptr` is null or an allocation of the running domain's heap not yet freed,
/// requested with `size` bytes.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn free_sized(ptr: *mut c_void, size: usize) {
    let Some(ptr) = NonNull::new(ptr.cast::<u8>()) else {
        return;
    };
    Heap::current().with(|h| {
        let owned = h.owns(ptr.as_ptr());
        #[cfg(feature = "forkpoint")]
        {
            forkpoint::assert_always_or_unreachable!(
                owned,
                "free_sized: ptr is a live allocation of the running domain's heap"
            );
            forkpoint::assert_always_or_unreachable!(
                // SAFETY: only read for a live allocation.
                !owned || unsafe { CmpctHeap::usable_size(ptr) } >= size,
                "free_sized: size is at most the allocation's size"
            );
        }
        if owned {
            // SAFETY: `owns` found a live allocation of this heap.
            let usable = unsafe { CmpctHeap::usable_size(ptr) };
            assert!(usable >= size, "expected {usable} got {size}");
            // SAFETY: `owns` found a live allocation of this heap.
            unsafe { h.free(ptr) };
        }
    });
}
