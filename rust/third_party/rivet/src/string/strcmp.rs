// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_char, c_int};

use crate::support::cstr::{self, CStrPair};

/// Compares two strings as `unsigned char`.
///
/// # Safety
///
/// `a` and `b` are NUL-terminated strings.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strcmp(a: *const c_char, b: *const c_char) -> c_int {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        !a.is_null() && !b.is_null(),
        "strcmp: a and b are non-null"
    );
    // SAFETY: both are NUL-terminated, per the caller.
    cstr::compare(unsafe { CStrPair::new(a, b) })
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use super::*;

    fn cmp(a: &CStr, b: &CStr) -> c_int {
        // SAFETY: both are NUL-terminated.
        unsafe { strcmp(a.as_ptr(), b.as_ptr()) }
    }

    #[test]
    fn strcmp_orders_by_first_difference_as_unsigned_char() {
        assert_eq!(cmp(c"abc", c"abc"), 0);
        assert!(cmp(c"abc", c"abd") < 0);
        assert!(cmp(c"ab", c"abc") < 0);
        assert!(cmp(c"\x80", c"\x7f") > 0);
    }
}
