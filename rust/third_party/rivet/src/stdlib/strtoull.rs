// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_char, c_int, c_ulonglong};

use crate::support::strto;

/// Converts the initial number in `s` to an `unsigned long long`, reading it
/// as `strtol` does. A `-` sign negates the value in `unsigned long long`. A
/// magnitude above `ULLONG_MAX` gives `ULLONG_MAX` with `errno` set to
/// `ERANGE`; an invalid base gives 0 with `EINVAL`.
///
/// # Safety
///
/// `s` is a NUL-terminated string, and `endptr` is null or writable.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strtoull(
    s: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
) -> c_ulonglong {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(!s.is_null(), "strtoull: s is non-null");
    let mut clamped = false;
    // SAFETY: the caller's contract is `unsigned`'s.
    let value = unsafe { strto::unsigned(s, endptr, base, c_ulonglong::MAX, &mut clamped) };
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(clamped, "strtoull: clamps a number outside unsigned long long");
    value
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use super::*;

    /// `strtoull` of `s`, and the offset of the end it stores.
    fn ull(s: &CStr, base: c_int) -> (c_ulonglong, usize) {
        let mut end = core::ptr::null_mut();
        // SAFETY: `s` is NUL-terminated and `end` is writable.
        let value = unsafe { strtoull(s.as_ptr(), &raw mut end, base) };
        (value, end.addr().wrapping_sub(s.as_ptr().addr()))
    }

    #[test]
    fn strtoull_reads_64_bit_numbers_on_every_target() {
        assert_eq!(ull(c"18446744073709551615", 10), (c_ulonglong::MAX, 20));
        assert_eq!(ull(c"0xffffffffffffffff", 16), (c_ulonglong::MAX, 18));
        assert_eq!(ull(c"-1", 10), (c_ulonglong::MAX, 2));
    }

    #[test]
    fn strtoull_clamps_to_unsigned_long_long() {
        let _errno = crate::support::errno::tests::take();
        assert_eq!(ull(c"18446744073709551616", 10), (c_ulonglong::MAX, 20));
    }
}
