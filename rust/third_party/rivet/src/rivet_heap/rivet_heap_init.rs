// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;
use core::ptr;

use crate::heap::{HEAP_SENTINELS, HEAP_STATE, Heap, rivet_heap};
use crate::support::cmpctmalloc::HEAP_ALIGNMENT;

/// Builds a heap in the `size` bytes at `arena` and returns it: the heap's
/// own state at the start, and its memory after. Called again on the same
/// arena, it frees every allocation, as a domain restart needs. Returns null
/// when `arena` cannot hold the heap and `RIVET_HEAP_OVERHEAD` bytes.
///
/// # Safety
///
/// `arena` is writable for `size` bytes that nothing else uses, and lives as
/// long as the program; `RIVET_HEAP_ARENA` declares such memory.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn rivet_heap_init(arena: *mut c_void, size: usize) -> *mut rivet_heap {
    let arena = arena.cast::<u8>();
    let skip = arena.align_offset(HEAP_ALIGNMENT);
    // The memory after the alignment skip and the heap's state; it must hold
    // the sentinels, so that `size` covers `RIVET_HEAP_OVERHEAD`.
    let memory = size.checked_sub(skip).and_then(|room| room.checked_sub(HEAP_STATE));
    let Some(memory) = memory.filter(|memory| *memory >= HEAP_SENTINELS) else {
        return ptr::null_mut();
    };
    if arena.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: the heap and its memory lie in the caller's `size` bytes, and the
    // heap is aligned for its type, which needs no more than `HEAP_ALIGNMENT`.
    unsafe {
        #[expect(
            clippy::cast_ptr_alignment,
            reason = "`skip` aligns `arena` to `HEAP_ALIGNMENT`, enough for `Heap`"
        )]
        let heap = arena.add(skip).cast::<Heap>();
        heap.write(Heap::new());
        let added = (*heap).unlocked().add_region(heap.cast::<u8>().add(HEAP_STATE), memory);
        #[cfg(feature = "forkpoint-coverage")]
        forkpoint::assert_sometimes!(true, "rivet_heap_init: builds a heap in an arena");
        if added { heap } else { ptr::null_mut() }
    }
}

zr::static_assert!(core::mem::align_of::<Heap>() <= HEAP_ALIGNMENT);
