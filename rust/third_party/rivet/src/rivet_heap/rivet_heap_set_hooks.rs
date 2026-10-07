// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use crate::heap::{self, rivet_heap_hooks};

/// Registers the kernel's calls: which heap is running, and how to lock one.
/// `hooks` is kept, not copied, and replaces any registered before; null
/// keeps the current ones. A call already in progress may still use the hooks
/// it started with.
///
/// # Safety
///
/// `hooks` is null or lives as long as the program, as must any hooks it
/// replaces, and its calls are safe to make from any code that allocates.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn rivet_heap_set_hooks(hooks: *const rivet_heap_hooks) {
    // SAFETY: the caller guarantees `hooks` outlives every use.
    if let Some(hooks) = unsafe { hooks.as_ref() } {
        heap::set_hooks(hooks);
    }
}
