// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_char;

use super::strlen;
use crate::support::cstr::{self, CStrCursor};

/// Appends at most `n` bytes of `src`, then a NUL, to the string in `dst`.
/// Unlike `strncpy`, the result is always NUL-terminated, and nothing is
/// padded. Returns `dst`.
///
/// # Safety
///
/// `dst` is a NUL-terminated string, `src` is NUL-terminated or readable for
/// `n` bytes, `dst` is writable for `strlen(dst) + min(n, strlen(src)) + 1`
/// bytes, and the two do not overlap.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strncat(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        !dst.is_null() && (n == 0 || !src.is_null()),
        "strncat: dst is non-null, and src when n is not 0"
    );
    // SAFETY: `dst` is NUL-terminated, per the caller.
    let end = unsafe { dst.add(strlen(dst)) };
    // SAFETY: `src` is NUL-terminated or readable for `n` bytes, per the caller.
    let mut src = unsafe { CStrCursor::bounded(src, n) };
    // SAFETY: `src` yields at most `min(n, strlen(src))` bytes, which `dst`
    // has room for after its string, with no overlap, per the caller.
    let copied = unsafe { cstr::copy(end, &mut src) };
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(copied == n && n > 0, "strncat: stops at n before src ends");
    // SAFETY: `dst` has room for the NUL after the bytes copied, per the caller.
    unsafe { end.add(copied).write(0) };
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strncat_appends_at_most_n_bytes_and_terminates() {
        let mut dst = *b"ab\0\x55\x55\x55\x55\x55";
        // SAFETY: `dst` is NUL-terminated with room for 3 bytes and a NUL.
        unsafe { strncat(dst.as_mut_ptr().cast(), c"12345".as_ptr(), 3) };
        assert_eq!(&dst, b"ab123\0\x55\x55");
        let mut dst = *b"ab\0\x55\x55\x55\x55\x55";
        // SAFETY: `dst` is NUL-terminated with room for the 2 bytes and a NUL.
        unsafe { strncat(dst.as_mut_ptr().cast(), c"12".as_ptr(), 5) };
        assert_eq!(&dst, b"ab12\0\x55\x55\x55");
        let mut dst = *b"ab\0\x55";
        // SAFETY: `n` is 0, so only the NUL is rewritten.
        let ret = unsafe { strncat(dst.as_mut_ptr().cast(), b"1".as_ptr().cast(), 0) };
        assert_eq!(ret.cast_const().cast::<u8>(), dst.as_ptr());
        assert_eq!(&dst, b"ab\0\x55");
    }
}
