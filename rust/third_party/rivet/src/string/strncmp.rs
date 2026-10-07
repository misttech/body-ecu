// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_char, c_int};

use crate::support::cstr::{self, CStrPair};

/// Compares at most `n` bytes of two strings as `unsigned char`.
///
/// # Safety
///
/// `a` and `b` are each NUL-terminated or readable for `n` bytes.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        n == 0 || (!a.is_null() && !b.is_null()),
        "strncmp: a and b are non-null when n is not 0"
    );
    // SAFETY: both are NUL-terminated or readable for `n` bytes, per the caller.
    cstr::compare(unsafe { CStrPair::bounded(a, b, n) })
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use super::*;

    fn cmp(a: &CStr, b: &CStr, n: usize) -> c_int {
        // SAFETY: both are NUL-terminated.
        unsafe { strncmp(a.as_ptr(), b.as_ptr(), n) }
    }

    #[test]
    fn strncmp_stops_after_n_bytes_or_at_the_nul() {
        assert_eq!(cmp(c"abcX", c"abcY", 3), 0);
        assert!(cmp(c"abcX", c"abcY", 4) < 0);
        assert_eq!(cmp(c"ab", c"ab", 10), 0);
        assert_eq!(cmp(c"a", c"b", 0), 0);
    }
}
