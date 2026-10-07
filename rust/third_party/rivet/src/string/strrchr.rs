// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_char, c_int};

use crate::support::cstr::CStrCursor;

/// Returns a pointer to the last `c`, converted to `char`, in `s`, or null. The
/// NUL that ends `s` is part of the string, so `c == 0` finds it.
///
/// # Safety
///
/// `s` is a NUL-terminated string.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strrchr(s: *const c_char, c: c_int) -> *mut c_char {
    let wanted = c as u8;
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(!s.is_null(), "strrchr: s is non-null");
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(wanted == 0, "strrchr: searches for the terminator");
    // SAFETY: `s` is NUL-terminated, per the caller.
    let mut s = unsafe { CStrCursor::new(s) };
    let mut last = core::ptr::null_mut();
    loop {
        let byte = s.peek();
        if byte == wanted {
            last = s.as_ptr().cast_mut();
        }
        if byte == 0 {
            return last;
        }
        s.advance();
    }
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use super::*;

    /// The offset of the last `c` in `s`, its NUL included, if any.
    fn find(s: &CStr, c: u8) -> Option<usize> {
        // SAFETY: `s` is NUL-terminated.
        let found = unsafe { strrchr(s.as_ptr(), c_int::from(c)) };
        (!found.is_null()).then(|| found.addr() - s.as_ptr().addr())
    }

    #[test]
    fn strrchr_finds_the_last_match_or_the_terminator() {
        let s = c"a\nb\n";
        assert_eq!(find(s, b'\n'), Some(3));
        assert_eq!(find(s, b'a'), Some(0));
        assert_eq!(find(s, 0), Some(4));
        assert_eq!(find(s, b'z'), None);
        assert_eq!(find(c"", b'a'), None);
    }
}
