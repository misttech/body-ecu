// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_char;

use crate::support::cstr::{self, CStrCursor};

/// Copies at most `n` bytes of `src` to `dst`, then pads `dst` with NULs to `n`
/// bytes. `dst` is not NUL-terminated when `src` is `n` bytes or longer.
/// Returns `dst`.
///
/// # Safety
///
/// `src` is NUL-terminated or readable for `n` bytes, `dst` is writable for `n`
/// bytes, and the two do not overlap.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        n == 0 || (!dst.is_null() && !src.is_null()),
        "strncpy: dst and src are non-null when n is not 0"
    );
    // SAFETY: `src` is NUL-terminated or readable for `n` bytes, per the caller.
    let mut src = unsafe { CStrCursor::bounded(src, n) };
    // SAFETY: `src` yields at most `n` bytes, and `dst` is writable for `n`
    // bytes it does not overlap, per the caller.
    let copied = unsafe { cstr::copy(dst, &mut src) };
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(copied < n, "strncpy: pads a short source with NULs");
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(n > 0 && copied == n, "strncpy: leaves dst unterminated");
    for i in copied..n {
        // SAFETY: `i < n`, and `dst` is writable for `n` bytes.
        unsafe { dst.add(i).write(0) };
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strncpy_pads_short_sources_and_truncates_long_ones() {
        let mut dst = [0x55_u8; 6];
        // SAFETY: `dst` holds 5 bytes, and the source is NUL-terminated.
        unsafe { strncpy(dst.as_mut_ptr().cast(), c"ab".as_ptr(), 5) };
        assert_eq!(&dst, b"ab\0\0\0\x55");
        let mut dst = [0x55_u8; 4];
        // SAFETY: `dst` holds 3 bytes, and the source is NUL-terminated.
        unsafe { strncpy(dst.as_mut_ptr().cast(), c"abcdef".as_ptr(), 3) };
        assert_eq!(&dst, b"abc\x55");
    }
}
