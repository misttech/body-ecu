// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_char;

use crate::support::cstr::CStrCursor;

/// Finds the first occurrence of `needle` in `haystack`. Returns a pointer to it,
/// `haystack` itself when `needle` is empty, or null when there is none.
///
/// # Safety
///
/// `haystack` and `needle` are NUL-terminated strings.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        !haystack.is_null() && !needle.is_null(),
        "strstr: haystack and needle are non-null"
    );
    // SAFETY: both are NUL-terminated, per the caller.
    let (mut start, needle) = unsafe { (CStrCursor::new(haystack), CStrCursor::new(needle)) };
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(
        needle.peek() == 0,
        "strstr: returns haystack for an empty needle"
    );
    loop {
        // Match `needle` against `haystack` from `start`.
        let (mut h, mut n) = (start.clone(), needle.clone());
        loop {
            let wanted = n.peek();
            if wanted == 0 {
                return start.as_ptr().cast_mut();
            }
            // At the end of `haystack`, `h` yields 0, which is not `wanted`.
            if h.peek() != wanted {
                break;
            }
            h.advance();
            n.advance();
        }
        if start.peek() == 0 {
            return core::ptr::null_mut();
        }
        start.advance();
    }
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use super::*;

    /// The offset of the first `needle` in `haystack`, if any.
    fn find(haystack: &CStr, needle: &CStr) -> Option<usize> {
        // SAFETY: both are NUL-terminated.
        let found = unsafe { strstr(haystack.as_ptr(), needle.as_ptr()) };
        (!found.is_null()).then(|| found.addr() - haystack.as_ptr().addr())
    }

    #[test]
    fn strstr_finds_the_first_occurrence() {
        let s = c"host.local.local";
        assert_eq!(find(s, c".local"), Some(4));
        assert_eq!(find(s, c"host"), Some(0));
        assert_eq!(find(s, c"l.local"), Some(9));
        // A partial match that fails restarts one byte on.
        assert_eq!(find(c"aab", c"ab"), Some(1));
    }

    #[test]
    fn strstr_misses_and_an_empty_needle() {
        let s = c"example.com";
        assert_eq!(find(s, c".local"), None);
        assert_eq!(find(s, c"example.commerce"), None);
        assert_eq!(find(c"", c"a"), None);
        assert_eq!(find(s, c""), Some(0));
        assert_eq!(find(c"", c""), Some(0));
    }
}
