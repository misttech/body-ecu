// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use crate::support::symbolize::{self, rivet_backtrace_hooks};

/// Registers the kernel's hooks for backtraces: which addresses are on the
/// running thread's stack, and the image's build ID note and code range.
/// `hooks` is kept, not copied, and replaces any registered before; null keeps
/// the current ones.
///
/// # Safety
///
/// `hooks` is null or lives as long as the program, as must any hooks it
/// replaces and everything they point at. Its `is_on_stack` holds only for
/// addresses readable as a word, and is safe to call from a fault or a panic.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn rivet_backtrace_set_hooks(hooks: *const rivet_backtrace_hooks) {
    // SAFETY: the caller guarantees `hooks` outlives every use.
    if let Some(hooks) = unsafe { hooks.as_ref() } {
        symbolize::set_hooks(hooks);
    }
}
