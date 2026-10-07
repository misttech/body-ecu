// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use crate::support::errno::{self, rivet_errno_hooks};

/// Registers the kernel's calls: which cell is the running thread's `errno`.
/// `hooks` is kept, not copied, and replaces any registered before; null
/// keeps the current ones. A call already in progress may still use the hooks
/// it started with.
///
/// # Safety
///
/// `hooks` is null or lives as long as the program, as must any hooks it
/// replaces, and its `current` is safe to call from any code that may set
/// `errno`, interrupt handlers included if they call such code.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn rivet_errno_set_hooks(hooks: *const rivet_errno_hooks) {
    // SAFETY: the caller guarantees `hooks` outlives every use.
    if let Some(hooks) = unsafe { hooks.as_ref() } {
        errno::set_hooks(hooks);
    }
}

#[cfg(test)]
mod tests {
    use core::ptr;

    use super::*;
    use crate::__errno_location;
    use crate::support::errno::tests as errno_tests;

    static NO_HOOKS: rivet_errno_hooks = rivet_errno_hooks { current: None };

    #[test]
    fn null_hooks_keep_the_current_ones() {
        let _guard = errno_tests::take();
        let before = __errno_location();
        // SAFETY: null is allowed, and `NO_HOOKS` is static.
        unsafe {
            rivet_errno_set_hooks(ptr::null());
            assert_eq!(__errno_location(), before);
            rivet_errno_set_hooks(&raw const NO_HOOKS);
        }
        assert_eq!(__errno_location(), before);
    }
}
