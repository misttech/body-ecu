// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `errno`: where it lives, and the codes rivet's functions set.
//!
//! `errno` is the `int` that `__errno_location` points at. Until the kernel
//! says otherwise, that is one cell for the program. A kernel with threads
//! registers a `current` call through `rivet_errno_set_hooks` that names the
//! running thread's own cell, in its control block or at the base of its
//! stack, and from then on every `errno` read and write goes there, with no
//! saving and restoring on a switch. The values are newlib's, as
//! `include/errno.h` defines them, so code built against newlib's `errno.h`
//! agrees with rivet on every code it sets.

use core::cell::UnsafeCell;
use core::ffi::c_int;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

/// `EIO`: a stream's write failed.
pub(crate) const EIO: c_int = 5;
/// `EINVAL`: an argument is invalid, such as a `strtol` base or a `printf`
/// conversion.
pub(crate) const EINVAL: c_int = 22;
/// `ERANGE`: the result does not fit its type.
pub(crate) const ERANGE: c_int = 34;
/// `EOVERFLOW`: a `printf` result is longer than `int` can count.
#[cfg(target_os = "none")]
pub(crate) const EOVERFLOW: c_int = 139;

/// The calls a kernel makes available for `errno`, registered with
/// `rivet_errno_set_hooks`. Each may be null.
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct rivet_errno_hooks {
    /// The running thread's `errno`, or null for the program's one cell.
    pub current: Option<unsafe extern "C" fn() -> *mut c_int>,
}

zr::static_assert!(size_of::<rivet_errno_hooks>() == size_of::<usize>());
zr::static_assert!(align_of::<rivet_errno_hooks>() == align_of::<usize>());

static HOOKS: AtomicPtr<rivet_errno_hooks> = AtomicPtr::new(ptr::null_mut());

/// The `errno` of the program, used until the kernel names another.
struct Errno(UnsafeCell<c_int>);

// SAFETY: `errno` is C's, read and written through a raw pointer by one
// thread at a time: the one thread there is until the kernel registers its
// `current` hook, after which each thread has its own cell.
unsafe impl Sync for Errno {}

static ERRNO: Errno = Errno(UnsafeCell::new(0));

/// Registers the kernel's calls, replacing any registered before. A call
/// already in progress may still use the hooks it started with, so hooks
/// that are replaced must stay valid too, as a `&'static` does.
pub(crate) fn set_hooks(hooks: &'static rivet_errno_hooks) {
    HOOKS.store(ptr::from_ref(hooks).cast_mut(), Ordering::Release);
}

fn hooks() -> Option<&'static rivet_errno_hooks> {
    // SAFETY: `HOOKS` is null or was stored from a `&'static`.
    unsafe { HOOKS.load(Ordering::Acquire).as_ref() }
}

/// Where `errno` lives: the running thread's cell when the kernel names one,
/// else the program's.
pub(crate) fn location() -> *mut c_int {
    let current = hooks()
        .and_then(|h| h.current)
        // SAFETY: the kernel's hook takes no arguments and returns a cell
        // pointer or null.
        .map_or(ptr::null_mut(), |current| unsafe { current() });
    if current.is_null() { ERRNO.0.get() } else { current }
}

/// Sets `errno` to `code`.
pub(crate) fn set(code: c_int) {
    // SAFETY: `location` points at the program's cell, or at one the kernel
    // keeps live for the running thread, and each is written by one thread at
    // a time, per `Errno`'s `Sync` and the kernel's contract.
    unsafe { location().write(code) }
}

#[cfg(test)]
pub(crate) mod tests {
    use core::sync::atomic::AtomicBool;

    use super::*;

    static SERIAL: AtomicBool = AtomicBool::new(false);

    /// Holds `errno` for one test at a time: the host runs tests on several
    /// threads, and `errno` is one for the program.
    pub(crate) struct Guard;

    impl Drop for Guard {
        fn drop(&mut self) {
            SERIAL.store(false, Ordering::Release);
        }
    }

    /// Clears `errno` and keeps it for the caller until the guard drops.
    pub(crate) fn take() -> Guard {
        while SERIAL
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        set(0);
        Guard
    }

    /// The current `errno`.
    pub(crate) fn get() -> c_int {
        // SAFETY: as in `set`.
        unsafe { location().read() }
    }

    #[test]
    fn codes_are_newlibs() {
        assert_eq!((EINVAL, ERANGE), (22, 34));
        #[cfg(target_os = "none")]
        assert_eq!((EIO, EOVERFLOW), (5, 139));
    }

    #[test]
    fn set_writes_the_value_at_location() {
        let _guard = take();
        assert_eq!(get(), 0);
        set(ERANGE);
        assert_eq!(get(), ERANGE);
        assert_eq!(location(), location());
    }

    /// A thread's cell, as a kernel would keep it.
    struct Cell(UnsafeCell<c_int>);
    // SAFETY: the test writes it from one thread, under the guard.
    unsafe impl Sync for Cell {}
    static THREAD: Cell = Cell(UnsafeCell::new(0));

    unsafe extern "C" fn thread_errno() -> *mut c_int {
        THREAD.0.get()
    }

    unsafe extern "C" fn no_thread() -> *mut c_int {
        ptr::null_mut()
    }

    static THREAD_HOOKS: rivet_errno_hooks = rivet_errno_hooks { current: Some(thread_errno) };
    static NULL_HOOKS: rivet_errno_hooks = rivet_errno_hooks { current: Some(no_thread) };
    static NO_HOOKS: rivet_errno_hooks = rivet_errno_hooks { current: None };

    #[test]
    fn hooks_move_errno_to_the_running_thread() {
        let _guard = take();
        let program = location();
        set_hooks(&THREAD_HOOKS);
        assert_eq!(location(), THREAD.0.get());
        set(EINVAL);
        // SAFETY: the test's own cell, written once by `set`.
        assert_eq!(unsafe { THREAD.0.get().read() }, EINVAL);
        // SAFETY: the program's cell, untouched since the guard cleared it.
        assert_eq!(unsafe { program.read() }, 0);
        // A hook that names no thread, and no hook at all, fall back to the program's cell.
        set_hooks(&NULL_HOOKS);
        assert_eq!(location(), program);
        set_hooks(&NO_HOOKS);
        assert_eq!(location(), program);
    }
}
