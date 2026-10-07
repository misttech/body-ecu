// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The standard I/O lock, around every stream's output.
//!
//! `printf`, `fprintf`, `puts`, and the other output functions each write
//! through a stream's callback, `printf` in several calls. Two threads writing
//! at once would interleave their bytes. A kernel with threads registers a
//! `lock` and an `unlock` through `rivet_stdio_set_hooks`, and each of those
//! functions holds the lock for the whole of its output, whatever the stream,
//! so each call's bytes come out together. Without hooks, the streams are for
//! one thread at a time.

use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

/// The calls a kernel makes available for the streams, registered with
/// `rivet_stdio_set_hooks`. Each may be null.
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct rivet_stdio_hooks {
    /// Takes the standard I/O lock: a mutex, or a critical section.
    pub lock: Option<unsafe extern "C" fn()>,
    /// Releases what `lock` took.
    pub unlock: Option<unsafe extern "C" fn()>,
}

zr::static_assert!(size_of::<rivet_stdio_hooks>() == 2 * size_of::<usize>());
zr::static_assert!(align_of::<rivet_stdio_hooks>() == align_of::<usize>());

static HOOKS: AtomicPtr<rivet_stdio_hooks> = AtomicPtr::new(ptr::null_mut());

/// Registers the kernel's calls, replacing any registered before. A call
/// already in progress may still use the hooks it started with, so hooks that
/// are replaced must stay valid too, as a `&'static` does.
pub(crate) fn set_hooks(hooks: &'static rivet_stdio_hooks) {
    HOOKS.store(ptr::from_ref(hooks).cast_mut(), Ordering::Release);
}

/// Holds the standard I/O lock until it drops, through the hooks it
/// took the lock with.
struct Held(Option<&'static rivet_stdio_hooks>);

impl Drop for Held {
    fn drop(&mut self) {
        if let Some(unlock) = self.0.and_then(|hooks| hooks.unlock) {
            // SAFETY: the kernel's hook takes no arguments, and releases the
            // lock this guard took through the same hooks.
            unsafe { unlock() };
        }
    }
}

/// Runs `f` holding the standard I/O lock, if the kernel gave one, and
/// releases it however `f` returns.
pub(crate) fn locked<R>(f: impl FnOnce() -> R) -> R {
    // SAFETY: `HOOKS` is null or was stored from a `&'static`.
    let hooks = unsafe { HOOKS.load(Ordering::Acquire).as_ref() };
    if let Some(lock) = hooks.and_then(|hooks| hooks.lock) {
        // SAFETY: the kernel's hook takes no arguments.
        unsafe { lock() };
    }
    let _held = Held(hooks);
    f()
}

#[cfg(test)]
pub(crate) mod tests {
    use core::sync::atomic::{AtomicBool, AtomicUsize};

    use super::*;

    static SERIAL: AtomicBool = AtomicBool::new(false);

    /// Holds the stdio hooks for one test at a time, and puts back none when
    /// it drops: the host runs tests on several threads.
    pub(crate) struct Guard;

    impl Drop for Guard {
        fn drop(&mut self) {
            set_hooks(&NO_HOOKS);
            SERIAL.store(false, Ordering::Release);
        }
    }

    pub(crate) fn take() -> Guard {
        while SERIAL
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        Guard
    }

    static NO_HOOKS: rivet_stdio_hooks = rivet_stdio_hooks { lock: None, unlock: None };

    /// Whether the counting lock is held, and how often it was taken.
    pub(crate) static HELD: AtomicBool = AtomicBool::new(false);
    pub(crate) static LOCKS: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn lock() {
        assert!(!HELD.swap(true, Ordering::AcqRel), "lock is not reentered");
        LOCKS.fetch_add(1, Ordering::Relaxed);
    }

    unsafe extern "C" fn unlock() {
        assert!(HELD.swap(false, Ordering::AcqRel), "unlock follows lock");
    }

    pub(crate) static COUNTING: rivet_stdio_hooks =
        rivet_stdio_hooks { lock: Some(lock), unlock: Some(unlock) };

    #[test]
    fn locked_holds_the_lock_around_f_and_releases_it() {
        let _guard = take();
        set_hooks(&COUNTING);
        let before = LOCKS.load(Ordering::Relaxed);
        let inside = locked(|| HELD.load(Ordering::Acquire));
        assert!(inside);
        assert!(!HELD.load(Ordering::Acquire));
        assert_eq!(LOCKS.load(Ordering::Relaxed), before + 1);
    }

    #[test]
    fn locked_without_hooks_just_runs_f() {
        let _guard = take();
        assert_eq!(locked(|| 7), 7);
        assert!(!HELD.load(Ordering::Acquire));
    }
}
