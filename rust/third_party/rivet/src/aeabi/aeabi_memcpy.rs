// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;

use crate::memcpy;

/// Copies `n` bytes from `src` to `dst`, as `memcpy` does, returning nothing.
///
/// # Safety
///
/// As for `memcpy`: `src` is readable and `dst` writable for `n` bytes, and the
/// two do not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "aapcs" fn __aeabi_memcpy(dst: *mut c_void, src: *const c_void, n: usize) {
    // SAFETY: the caller's contract is `memcpy`'s.
    unsafe { memcpy(dst, src, n) };
}

/// `__aeabi_memcpy` for pointers aligned to 4 bytes.
///
/// # Safety
///
/// As for `__aeabi_memcpy`.
#[unsafe(no_mangle)]
pub unsafe extern "aapcs" fn __aeabi_memcpy4(dst: *mut c_void, src: *const c_void, n: usize) {
    // SAFETY: the caller's contract is `memcpy`'s.
    unsafe { memcpy(dst, src, n) };
}

/// `__aeabi_memcpy` for pointers aligned to 8 bytes.
///
/// # Safety
///
/// As for `__aeabi_memcpy`.
#[unsafe(no_mangle)]
pub unsafe extern "aapcs" fn __aeabi_memcpy8(dst: *mut c_void, src: *const c_void, n: usize) {
    // SAFETY: the caller's contract is `memcpy`'s.
    unsafe { memcpy(dst, src, n) };
}
