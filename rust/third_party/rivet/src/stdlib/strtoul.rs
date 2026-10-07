// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_char, c_int, c_ulong};

use crate::support::strto;

/// Converts the initial number in `s` to an `unsigned long`, reading it as
/// `strtol` does. A `-` sign negates the value in `unsigned long`, so `-1`
/// gives `ULONG_MAX`. A magnitude above `ULONG_MAX` gives `ULONG_MAX` with
/// `errno` set to `ERANGE`; an invalid base gives 0 with `EINVAL`.
///
/// # Safety
///
/// `s` is a NUL-terminated string, and `endptr` is null or writable.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strtoul(
    s: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
) -> c_ulong {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(!s.is_null(), "strtoul: s is non-null");
    let mut clamped = false;
    // SAFETY: the caller's contract is `unsigned`'s.
    let value = unsafe { strto::unsigned(s, endptr, base, c_ulong::MAX, &mut clamped) };
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(clamped, "strtoul: clamps a number outside unsigned long");
    // `unsigned` keeps the value within `unsigned long`.
    value as c_ulong
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use super::*;

    /// `strtoul` of `s`, and the offset of the end it stores.
    fn ul(s: &CStr, base: c_int) -> (c_ulong, usize) {
        let mut end = core::ptr::null_mut();
        // SAFETY: `s` is NUL-terminated and `end` is writable.
        let value = unsafe { strtoul(s.as_ptr(), &raw mut end, base) };
        (value, end.addr().wrapping_sub(s.as_ptr().addr()))
    }

    #[test]
    fn strtoul_reads_the_number_and_negates_in_the_type() {
        assert_eq!(ul(c"42", 10), (42, 2));
        assert_eq!(ul(c"0x10", 0), (16, 4));
        assert_eq!(ul(c"-1", 10), (c_ulong::MAX, 2));
        assert_eq!(ul(c"-2", 10), (c_ulong::MAX - 1, 2));
    }

    #[test]
    fn strtoul_clamps_to_unsigned_long() {
        let _errno = crate::support::errno::tests::take();
        assert_eq!(ul(c"99999999999999999999999", 10), (c_ulong::MAX, 23));
    }
}
