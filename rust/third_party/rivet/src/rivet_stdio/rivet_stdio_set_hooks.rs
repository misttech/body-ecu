// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use crate::support::stdout_lock::{self, rivet_stdio_hooks};

/// Registers the kernel's calls: how to take the standard I/O lock. `hooks` is kept,
/// not copied, and replaces any registered before; null keeps the current
/// ones. A call already in progress may still use the hooks it started with.
///
/// # Safety
///
/// `hooks` is null or lives as long as the program, as must any hooks it
/// replaces, and its calls are safe to make from any code that writes to a
/// stream.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn rivet_stdio_set_hooks(hooks: *const rivet_stdio_hooks) {
    // SAFETY: the caller guarantees `hooks` outlives every use.
    if let Some(hooks) = unsafe { hooks.as_ref() } {
        stdout_lock::set_hooks(hooks);
    }
}

#[cfg(test)]
mod tests {
    use core::ptr;
    use core::sync::atomic::Ordering;

    use super::*;
    use crate::puts;
    use crate::support::stdout_lock::tests::{self as lock_tests, COUNTING, HELD, LOCKS};

    #[test]
    fn puts_holds_the_registered_lock_and_null_keeps_it() {
        let _guard = lock_tests::take();
        // SAFETY: `COUNTING` is static, and null is allowed.
        unsafe {
            rivet_stdio_set_hooks(&raw const COUNTING);
            rivet_stdio_set_hooks(ptr::null());
        }
        let before = LOCKS.load(Ordering::Relaxed);
        // SAFETY: NUL-terminated. The host's `write` is the test's own stdout.
        let _ = unsafe { puts(c"locked line".as_ptr()) };
        assert_eq!(LOCKS.load(Ordering::Relaxed), before + 1);
        assert!(!HELD.load(Ordering::Acquire));
    }
}
