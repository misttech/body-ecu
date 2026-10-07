// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;

use crate::memset;

/// Fills `n` bytes at `dst` with zeros, returning nothing.
///
/// # Safety
///
/// As for `memset`: `dst` is writable for `n` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "aapcs" fn __aeabi_memclr(dst: *mut c_void, n: usize) {
    // SAFETY: the caller's contract is `memset`'s.
    unsafe { memset(dst, 0, n) };
}

/// `__aeabi_memclr` for pointers aligned to 4 bytes.
///
/// # Safety
///
/// As for `__aeabi_memclr`.
#[unsafe(no_mangle)]
pub unsafe extern "aapcs" fn __aeabi_memclr4(dst: *mut c_void, n: usize) {
    // SAFETY: the caller's contract is `memset`'s.
    unsafe { memset(dst, 0, n) };
}

/// `__aeabi_memclr` for pointers aligned to 8 bytes.
///
/// # Safety
///
/// As for `__aeabi_memclr`.
#[unsafe(no_mangle)]
pub unsafe extern "aapcs" fn __aeabi_memclr8(dst: *mut c_void, n: usize) {
    // SAFETY: the caller's contract is `memset`'s.
    unsafe { memset(dst, 0, n) };
}
