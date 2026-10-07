// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_char, c_int};

use crate::support::cstr::CStrCursor;

/// Returns a pointer to the first `c`, converted to `char`, in `s`, or null. The
/// NUL that ends `s` is part of the string, so `c == 0` finds it.
///
/// # Safety
///
/// `s` is a NUL-terminated string.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strchr(s: *const c_char, c: c_int) -> *mut c_char {
    let wanted = c as u8;
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(!s.is_null(), "strchr: s is non-null");
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(wanted == 0, "strchr: searches for the terminator");
    // SAFETY: `s` is NUL-terminated, per the caller.
    let mut s = unsafe { CStrCursor::new(s) };
    loop {
        let byte = s.peek();
        if byte == wanted {
            return s.as_ptr().cast_mut();
        }
        if byte == 0 {
            return core::ptr::null_mut();
        }
        s.advance();
    }
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use super::*;

    /// The offset of the first `c` in `s`, its NUL included, if any.
    fn find(s: &CStr, c: u8) -> Option<usize> {
        // SAFETY: `s` is NUL-terminated.
        let found = unsafe { strchr(s.as_ptr(), c_int::from(c)) };
        (!found.is_null()).then(|| found.addr() - s.as_ptr().addr())
    }

    #[test]
    fn strchr_finds_the_first_match_or_the_terminator() {
        let s = c"a\nb\n";
        assert_eq!(find(s, b'\n'), Some(1));
        assert_eq!(find(s, 0), Some(4));
        assert_eq!(find(s, b'z'), None);
    }
}
