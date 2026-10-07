// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_int, c_void};

use crate::memset;

/// Fills `n` bytes at `dst` with `c` converted to `unsigned char`, as `memset`
/// does, returning nothing. Unlike `memset`, it takes the count before the
/// byte.
///
/// # Safety
///
/// As for `memset`: `dst` is writable for `n` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "aapcs" fn __aeabi_memset(dst: *mut c_void, n: usize, c: c_int) {
    // SAFETY: the caller's contract is `memset`'s.
    unsafe { memset(dst, c, n) };
}

/// `__aeabi_memset` for pointers aligned to 4 bytes.
///
/// # Safety
///
/// As for `__aeabi_memset`.
#[unsafe(no_mangle)]
pub unsafe extern "aapcs" fn __aeabi_memset4(dst: *mut c_void, n: usize, c: c_int) {
    // SAFETY: the caller's contract is `memset`'s.
    unsafe { memset(dst, c, n) };
}

/// `__aeabi_memset` for pointers aligned to 8 bytes.
///
/// # Safety
///
/// As for `__aeabi_memset`.
#[unsafe(no_mangle)]
pub unsafe extern "aapcs" fn __aeabi_memset8(dst: *mut c_void, n: usize, c: c_int) {
    // SAFETY: the caller's contract is `memset`'s.
    unsafe { memset(dst, c, n) };
}
