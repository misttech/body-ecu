// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_char, c_int, c_long};

use crate::ctype::isspace;
use crate::support::cstr::CStrCursor;

/// Converts the initial decimal number in `s` to an `int`, as `(int)strtol(s, NULL, 10)`
/// does: leading white space is skipped, one `+` or `-` sign is taken, and the digits run
/// to the first non-digit. A number too large for `long` is clamped to `LONG_MAX` or
/// `LONG_MIN` before the conversion to `int`; no digits give 0.
///
/// # Safety
///
/// `s` is a NUL-terminated string.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn atoi(s: *const c_char) -> c_int {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(!s.is_null(), "atoi: s is non-null");
    // SAFETY: `s` is NUL-terminated, per the caller.
    let mut s = unsafe { CStrCursor::new(s) };
    while s.next_if(|b| isspace(c_int::from(b)) != 0).is_some() {}
    let negative = s.next_if(|b| b == b'-' || b == b'+') == Some(b'-');
    let mut value: c_long = 0;
    while let Some(b) = s.next_if(|b| b.is_ascii_digit()) {
        // A digit is `b'0'` or above, so this cannot wrap.
        let digit = c_long::from(b.wrapping_sub(b'0'));
        value = if negative {
            value.saturating_mul(10).saturating_sub(digit)
        } else {
            value.saturating_mul(10).saturating_add(digit)
        };
    }
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(
        value == c_long::MAX || value == c_long::MIN,
        "atoi: reaches LONG_MAX or LONG_MIN"
    );
    value as c_int
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use super::*;

    fn a(s: &CStr) -> c_int {
        // SAFETY: NUL-terminated.
        unsafe { atoi(s.as_ptr()) }
    }

    #[test]
    fn atoi_reads_the_leading_decimal_number() {
        assert_eq!(a(c"0"), 0);
        assert_eq!(a(c"42"), 42);
        assert_eq!(a(c"  \t\n\x0b\x0c\r-17abc"), -17);
        assert_eq!(a(c"+8"), 8);
        assert_eq!(a(c"007"), 7);
        assert_eq!(a(c""), 0);
        assert_eq!(a(c"x1"), 0);
        assert_eq!(a(c"- 1"), 0);
        assert_eq!(a(c"+-1"), 0);
        assert_eq!(a(c"2147483647"), 2_147_483_647);
        assert_eq!(a(c"-2147483648"), -2_147_483_648);
    }

    #[test]
    fn atoi_clamps_to_long_before_converting_to_int() {
        // strtol clamps to LONG_MAX; the conversion to int keeps the low bits, as C's does.
        assert_eq!(a(c"99999999999999999999999"), c_long::MAX as c_int);
        assert_eq!(a(c"-99999999999999999999999"), c_long::MIN as c_int);
    }
}
