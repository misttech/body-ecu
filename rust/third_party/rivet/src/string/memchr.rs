// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_int, c_void};

/// Returns a pointer to the first byte equal to `c`, converted to `unsigned
/// char`, among the `n` bytes at `s`, or null if none is. Unlike `strchr`, a
/// NUL is an ordinary byte here.
///
/// # Safety
///
/// `s` is readable for `n` bytes.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void {
    let wanted = c as u8;
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        n == 0 || !s.is_null(),
        "memchr: s is non-null when n is not 0"
    );
    let s = s.cast::<u8>();
    for i in 0..n {
        // SAFETY: `i < n`, and `s` is readable for `n` bytes, per the caller.
        if unsafe { s.add(i).read() } == wanted {
            return s.wrapping_add(i).cast_mut().cast();
        }
    }
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(n > 0, "memchr: finds no match in a non-empty range");
    core::ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The offset of the first `c` among the first `n` bytes of `s`, if any.
    fn find(s: &[u8], c: u8, n: usize) -> Option<usize> {
        assert!(n <= s.len());
        // SAFETY: `n` bytes of `s` are readable.
        let found = unsafe { memchr(s.as_ptr().cast(), c_int::from(c), n) };
        (!found.is_null()).then(|| found.addr() - s.as_ptr().addr())
    }

    #[test]
    fn memchr_finds_the_first_match_within_n() {
        let s = b"ab\0ab";
        assert_eq!(find(s, b'b', 5), Some(1));
        assert_eq!(find(s, b'b', 1), None);
        assert_eq!(find(s, 0, 5), Some(2));
        assert_eq!(find(s, b'z', 5), None);
        assert_eq!(find(s, b'a', 0), None);
    }

    #[test]
    fn memchr_compares_the_low_byte_of_c() {
        let s = b"\xff";
        // SAFETY: 1 byte of `s` is readable.
        let found = unsafe { memchr(s.as_ptr().cast(), -1, 1) };
        assert_eq!(found.cast_const().cast::<u8>(), s.as_ptr());
    }
}
