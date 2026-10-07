// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;
use core::ptr::NonNull;

use super::heap_or_current;
use crate::heap::rivet_heap;

/// Frees `ptr`, an allocation of `heap`, or of the running domain's heap when
/// `heap` is null. A null `ptr` does nothing, and so does any pointer that is
/// not the start of a live allocation of that heap.
///
/// # Safety
///
/// `heap` is null or a heap from `rivet_heap_init`, and `ptr` is null or an
/// allocation of that heap not yet freed.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn rivet_heap_free(heap: *mut rivet_heap, ptr: *mut c_void) {
    let Some(ptr) = NonNull::new(ptr.cast::<u8>()) else {
        return;
    };
    // SAFETY: the caller guarantees `heap`.
    let heap = unsafe { heap_or_current(heap) };
    heap.with(|h| {
        let owned = h.owns(ptr.as_ptr());
        #[cfg(feature = "forkpoint")]
        forkpoint::assert_always_or_unreachable!(
            owned,
            "rivet_heap_free: ptr is a live allocation of heap"
        );
        if owned {
            // SAFETY: `owns` found a live allocation of this heap.
            unsafe { h.free(ptr) };
        }
    });
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::heap::HeapStats;
    use crate::rivet_heap::{rivet_heap_alloc, rivet_heap_init, rivet_heap_stats};

    fn free_bytes(heap: *mut rivet_heap) -> usize {
        let mut stats = HeapStats::default();
        // SAFETY: `heap` was built by `rivet_heap_init`.
        unsafe { rivet_heap_stats(heap, &raw mut stats) };
        stats.free
    }

    // Breaks the contract on purpose, so it runs here rather than in the suite,
    // which Forkpoint judges.
    #[test]
    fn rivet_heap_free_ignores_what_is_not_a_live_allocation() {
        let arena = std::boxed::Box::into_raw(std::vec![0u64; 1024].into_boxed_slice());
        // SAFETY: the arena is the heap's until it is freed at the end, and
        // every pointer below either is `p` or must be ignored.
        unsafe {
            let heap = rivet_heap_init(arena.cast(), 8192);
            let p = rivet_heap_alloc(heap, 64, 0).cast::<u8>();
            let before = free_bytes(heap);
            for bad in [p.wrapping_add(16), p.wrapping_add(1), p.wrapping_add(8)] {
                rivet_heap_free(heap, bad.cast());
            }
            assert_eq!(free_bytes(heap), before);
            rivet_heap_free(heap, p.cast());
            let freed = free_bytes(heap);
            assert!(freed > before);
            // A second free of the same block changes nothing either.
            rivet_heap_free(heap, p.cast());
            assert_eq!(free_bytes(heap), freed);
            drop(std::boxed::Box::from_raw(arena));
        }
    }
}
