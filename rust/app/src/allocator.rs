// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The heap: Zephyr's kernel heap (`CONFIG_HEAP_MEM_POOL_SIZE`), which the C++ `new`
//! and the standard containers draw from, through the shim.

use core::alloc::{GlobalAlloc, Layout};

struct ZephyrHeap;

// SAFETY: `k_aligned_alloc` and `k_free` are a conforming allocator pair, thread safe
// within the kernel.
unsafe impl GlobalAlloc for ZephyrHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        body_ecu_ffi::malloc(layout.size(), layout.align())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        // SAFETY: `ptr` came from `alloc`, per the trait's contract.
        unsafe { body_ecu_ffi::free(ptr) }
    }
}

#[global_allocator]
static HEAP: ZephyrHeap = ZephyrHeap;
