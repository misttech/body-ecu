// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;

use crate::memmove;

/// Copies `n` bytes from `src` to `dst`, which may overlap, as `memmove` does,
/// returning nothing.
///
/// # Safety
///
/// As for `memmove`: `src` is readable and `dst` writable for `n` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "aapcs" fn __aeabi_memmove(dst: *mut c_void, src: *const c_void, n: usize) {
    // SAFETY: the caller's contract is `memmove`'s.
    unsafe { memmove(dst, src, n) };
}

/// `__aeabi_memmove` for pointers aligned to 4 bytes.
///
/// # Safety
///
/// As for `__aeabi_memmove`.
#[unsafe(no_mangle)]
pub unsafe extern "aapcs" fn __aeabi_memmove4(dst: *mut c_void, src: *const c_void, n: usize) {
    // SAFETY: the caller's contract is `memmove`'s.
    unsafe { memmove(dst, src, n) };
}

/// `__aeabi_memmove` for pointers aligned to 8 bytes.
///
/// # Safety
///
/// As for `__aeabi_memmove`.
#[unsafe(no_mangle)]
pub unsafe extern "aapcs" fn __aeabi_memmove8(dst: *mut c_void, src: *const c_void, n: usize) {
    // SAFETY: the caller's contract is `memmove`'s.
    unsafe { memmove(dst, src, n) };
}
