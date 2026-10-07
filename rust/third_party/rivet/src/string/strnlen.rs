// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_char;

use crate::support::cstr::CStrCursor;

/// Returns the number of bytes before the NUL that ends `s`, or `n` if no NUL
/// lies within the first `n` bytes.
///
/// # Safety
///
/// `s` is NUL-terminated or readable for `n` bytes.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strnlen(s: *const c_char, n: usize) -> usize {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        n == 0 || !s.is_null(),
        "strnlen: s is non-null when n is not 0"
    );
    // SAFETY: `s` is NUL-terminated or readable for `n` bytes, per the caller.
    let mut s = unsafe { CStrCursor::bounded(s, n) };
    while s.peek() != 0 {
        s.advance();
    }
    let len = s.position();
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(len == n, "strnlen: finds no NUL within n bytes");
    len
}

#[cfg(test)]
mod tests {
    use super::*;

    fn len(s: &[u8], n: usize) -> usize {
        assert!(n <= s.len() || s.contains(&0));
        // SAFETY: `s` is readable for `n` bytes, or holds a NUL before them.
        unsafe { strnlen(s.as_ptr().cast(), n) }
    }

    #[test]
    fn strnlen_stops_at_the_nul_or_at_n() {
        assert_eq!(len(b"rivet\0", 10), 5);
        assert_eq!(len(b"rivet\0", 5), 5);
        assert_eq!(len(b"rivet", 3), 3);
        assert_eq!(len(b"\0", 4), 0);
        assert_eq!(len(b"", 0), 0);
    }
}
