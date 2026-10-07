// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_char, c_int, c_longlong};

use crate::support::strto;

/// Converts the initial number in `s` to a `long long`, reading it as `strtol`
/// does. A number outside `long long` gives `LLONG_MAX` or `LLONG_MIN` with
/// `errno` set to `ERANGE`; an invalid base gives 0 with `EINVAL`.
///
/// # Safety
///
/// `s` is a NUL-terminated string, and `endptr` is null or writable.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strtoll(
    s: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
) -> c_longlong {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(!s.is_null(), "strtoll: s is non-null");
    let mut clamped = false;
    // SAFETY: the caller's contract is `signed`'s.
    let value =
        unsafe { strto::signed(s, endptr, base, c_longlong::MIN, c_longlong::MAX, &mut clamped) };
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(clamped, "strtoll: clamps a number outside long long");
    value
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use super::*;

    /// `strtoll` of `s`, and the offset of the end it stores.
    fn ll(s: &CStr, base: c_int) -> (c_longlong, usize) {
        let mut end = core::ptr::null_mut();
        // SAFETY: `s` is NUL-terminated and `end` is writable.
        let value = unsafe { strtoll(s.as_ptr(), &raw mut end, base) };
        (value, end.addr().wrapping_sub(s.as_ptr().addr()))
    }

    #[test]
    fn strtoll_reads_64_bit_numbers_on_every_target() {
        assert_eq!(ll(c"9223372036854775807", 10), (c_longlong::MAX, 19));
        assert_eq!(ll(c"-9223372036854775808", 10), (c_longlong::MIN, 20));
        assert_eq!(ll(c"-0x8000000000000000", 0), (c_longlong::MIN, 19));
    }

    #[test]
    fn strtoll_clamps_to_long_long() {
        let _errno = crate::support::errno::tests::take();
        assert_eq!(ll(c"9223372036854775808", 10), (c_longlong::MAX, 19));
        assert_eq!(ll(c"-9223372036854775809", 10), (c_longlong::MIN, 20));
    }
}
