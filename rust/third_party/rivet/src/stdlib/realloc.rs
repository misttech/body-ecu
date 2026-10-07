// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;
use core::ptr::{self, NonNull};

use crate::heap::Heap;
use crate::support::cmpctmalloc::CmpctHeap;

/// Resizes the allocation at `ptr` to `size` bytes. It resizes in place
/// when it can: shrinking always does, which a full heap needs, and growing
/// does when the block's right neighbor is free. Otherwise it moves: a new
/// block gets the first `min(old, size)` bytes, and `ptr` is freed.
///
/// A null `ptr` is `malloc(size)`; a zero `size` is `free(ptr)` and returns
/// null. On failure, and for a `ptr` that is not the start of a live
/// allocation of the running domain's heap, nothing changes and null is
/// returned.
///
/// # Safety
///
/// `ptr` is null or an allocation of the running domain's heap not yet freed.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    let raw = |p: Option<NonNull<u8>>| p.map_or(ptr::null_mut(), NonNull::as_ptr);
    Heap::current()
        .with(|h| {
            let Some(old) = NonNull::new(ptr.cast::<u8>()) else {
                return raw(h.alloc(size));
            };
            let owned = h.owns(old.as_ptr());
            #[cfg(feature = "forkpoint")]
            forkpoint::assert_always_or_unreachable!(
                owned,
                "realloc: ptr is a live allocation of the running domain's heap"
            );
            if !owned {
                return ptr::null_mut();
            }
            // SAFETY: `owns` found a live allocation of this heap.
            unsafe {
                if size == 0 {
                    h.free(old);
                    return ptr::null_mut();
                }
                let old_size = CmpctHeap::usable_size(old);
                if h.resize_in_place(old, size) {
                    #[cfg(feature = "forkpoint-coverage")]
                    {
                        forkpoint::assert_sometimes!(
                            size <= old_size,
                            "realloc: shrinks a block in place"
                        );
                        forkpoint::assert_sometimes!(
                            size > old_size,
                            "realloc: grows a block in place"
                        );
                    }
                    return old.as_ptr();
                }
                let new = h.alloc(size);
                if let Some(new) = new {
                    #[cfg(feature = "forkpoint-coverage")]
                    forkpoint::assert_sometimes!(true, "realloc: moves a block to grow it");
                    // The two blocks are different, and `new` holds more than
                    // `old`, or resizing in place would have succeeded.
                    ptr::copy_nonoverlapping(old.as_ptr(), new.as_ptr(), old_size.min(size));
                    h.free(old);
                }
                raw(new)
            }
        })
        .cast()
}
