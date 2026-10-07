// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_char, c_int, c_long};

use crate::support::strto;

/// Converts the initial number in `s` to a `long`: white space, then an
/// optional sign, then digits in `base`, which is 0 or 2 to 36. Base 16 allows
/// a `0x` prefix; base 0 takes the base from the prefix: `0x` is 16, `0` is 8,
/// and anything else is 10. If `endptr` is not null, it receives the address
/// just past the number, or `s` when there is none. A number outside `long`
/// gives `LONG_MAX` or `LONG_MIN` with `errno` set to `ERANGE`; an invalid base
/// gives 0 with `EINVAL`.
///
/// # Safety
///
/// `s` is a NUL-terminated string, and `endptr` is null or writable.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strtol(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(!s.is_null(), "strtol: s is non-null");
    let mut clamped = false;
    // SAFETY: the caller's contract is `signed`'s.
    let value = unsafe { strto::signed(s, endptr, base, c_long::MIN, c_long::MAX, &mut clamped) };
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(clamped, "strtol: clamps a number outside long");
    // `signed` keeps the value within `long`.
    value as c_long
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use super::*;

    /// `strtol` of `s`, and the offset of the end it stores.
    fn l(s: &CStr, base: c_int) -> (c_long, usize) {
        let mut end = core::ptr::null_mut();
        // SAFETY: `s` is NUL-terminated and `end` is writable.
        let value = unsafe { strtol(s.as_ptr(), &raw mut end, base) };
        (value, end.addr().wrapping_sub(s.as_ptr().addr()))
    }

    #[test]
    fn strtol_reads_the_number_and_stores_its_end() {
        assert_eq!(l(c"  -42xyz", 10), (-42, 5));
        assert_eq!(l(c"0x1f", 0), (0x1f, 4));
        assert_eq!(l(c"z", 36), (35, 1));
        assert_eq!(l(c"abc", 10), (0, 0));
        // SAFETY: `s` is NUL-terminated and a null `endptr` is allowed.
        assert_eq!(unsafe { strtol(c"7".as_ptr(), core::ptr::null_mut(), 10) }, 7);
    }

    #[test]
    fn strtol_clamps_to_long() {
        let _errno = crate::support::errno::tests::take();
        assert_eq!(l(c"99999999999999999999999", 10), (c_long::MAX, 23));
        assert_eq!(l(c"-99999999999999999999999", 10), (c_long::MIN, 24));
    }
}
