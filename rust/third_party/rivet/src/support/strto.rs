// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The number parser behind `strtol`, `strtoul`, `strtoll`, and `strtoull`.
//!
//! [`parse`] reads the subject sequence the C standard describes, white space,
//! sign, prefix, and digits, into a magnitude with a sign, and [`signed`] and
//! [`unsigned`] fit that into the function's type, as C's rules say: clamping
//! with `ERANGE`, and negating in the unsigned type.

use core::ffi::{c_char, c_int};

use crate::ctype::isspace;
use crate::support::cstr::CStrCursor;
use crate::support::errno::{self, EINVAL, ERANGE};

/// The largest base, where the digits are `0` to `9` and `a` to `z`.
const MAX_BASE: u8 = 36;

/// A number read by [`parse`].
struct Parsed {
    /// The digits' value, `u64::MAX` when `overflow`.
    magnitude: u64,
    negative: bool,
    /// Whether the digits exceed `u64`, and so every type here.
    overflow: bool,
    /// Just past the last byte of the number, or the start of the string when
    /// it holds no number.
    end: *const c_char,
}

/// The value of `b` as a digit in base up to [`MAX_BASE`], if it is one.
fn digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'z' => Some(b - b'a' + 10),
        b'A'..=b'Z' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Reads the number at the start of `s` in `base`, which is 0 or 2 to 36 (the
/// callers check): white space, then an optional sign, then for base 16 an
/// optional `0x`, then the digits. Base 0 takes the base from the prefix:
/// `0x` is 16, `0` is 8, and anything else is 10.
///
/// # Safety
///
/// `s` is a NUL-terminated string.
unsafe fn parse(s: *const c_char, base: u8) -> Parsed {
    let mut parsed = Parsed { magnitude: 0, negative: false, overflow: false, end: s };
    // SAFETY: `s` is NUL-terminated, per the caller.
    let mut s = unsafe { CStrCursor::new(s) };
    while s.next_if(|b| isspace(c_int::from(b)) != 0).is_some() {}
    parsed.negative = s.next_if(|b| b == b'-' || b == b'+') == Some(b'-');
    let mut base = base;
    if (base == 0 || base == 16) && s.peek() == b'0' {
        // The `0` is a digit on its own, if no hex digit follows the `x`.
        s.advance();
        parsed.end = s.as_ptr();
        let mut after_x = s.clone();
        after_x.advance();
        if (s.peek() == b'x' || s.peek() == b'X') && after_x.peek().is_ascii_hexdigit() {
            s.advance();
            base = 16;
        } else if base == 0 {
            base = 8;
        }
    } else if base == 0 {
        base = 10;
    }
    while let Some(d) = digit(s.peek()).filter(|&d| d < base) {
        s.advance();
        parsed.end = s.as_ptr();
        match parsed
            .magnitude
            .checked_mul(u64::from(base))
            .and_then(|m| m.checked_add(u64::from(d)))
        {
            Some(magnitude) => parsed.magnitude = magnitude,
            None => {
                parsed.overflow = true;
                parsed.magnitude = u64::MAX;
            }
        }
    }
    parsed
}

/// Parses the number in `s` and stores where it ends in `endptr`, if that is
/// not null; `None` with `EINVAL` set for a base that is not 0 or 2 to 36.
///
/// # Safety
///
/// `s` is a NUL-terminated string, and `endptr` is null or writable.
unsafe fn parse_with_end(
    s: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
) -> Option<Parsed> {
    let base = u8::try_from(base).ok().filter(|&b| b == 0 || (2..=MAX_BASE).contains(&b));
    let Some(base) = base else {
        errno::set(EINVAL);
        if !endptr.is_null() {
            // SAFETY: `endptr` is writable, per the caller.
            unsafe { endptr.write(s.cast_mut()) };
        }
        return None;
    };
    // SAFETY: `s` is NUL-terminated, per the caller.
    let parsed = unsafe { parse(s, base) };
    if !endptr.is_null() {
        // SAFETY: `endptr` is writable, per the caller.
        unsafe { endptr.write(parsed.end.cast_mut()) };
    }
    Some(parsed)
}

/// `strtol` for a signed type with the range `min` to `max`: the number in
/// `s`, clamped to that range with `ERANGE` set when it falls outside, and 0
/// with `EINVAL` set for a base that is not 0 or 2 to 36. The result of a
/// clamp is reported in `clamped`, for the callers' coverage properties.
///
/// # Safety
///
/// `s` is a NUL-terminated string, and `endptr` is null or writable.
pub(crate) unsafe fn signed(
    s: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
    min: impl Into<i64>,
    max: impl Into<i64>,
    clamped: &mut bool,
) -> i64 {
    // SAFETY: the caller's contract is this function's.
    let Some(parsed) = (unsafe { parse_with_end(s, endptr, base) }) else { return 0 };
    // The limits come as the C type's, which may be narrower than `i64`.
    let (min, max): (i64, i64) = (min.into(), max.into());
    let limit = if parsed.negative { min.unsigned_abs() } else { max.unsigned_abs() };
    if parsed.overflow || parsed.magnitude > limit {
        *clamped = true;
        errno::set(ERANGE);
        return if parsed.negative { min } else { max };
    }
    // The magnitude is at most `min.unsigned_abs()`, so the negation wraps
    // only for `min` itself, to `min`.
    let value = parsed.magnitude as i64;
    if parsed.negative { value.wrapping_neg() } else { value }
}

/// `strtoul` for an unsigned type with the range 0 to `max`, where `max + 1`
/// is a power of two: the number in `s`, negated in that type when it has a
/// `-` sign, `max` with `ERANGE` set when its magnitude exceeds `max`, and 0
/// with `EINVAL` set for a base that is not 0 or 2 to 36. The result of a
/// clamp is reported in `clamped`, for the callers' coverage properties.
///
/// # Safety
///
/// `s` is a NUL-terminated string, and `endptr` is null or writable.
pub(crate) unsafe fn unsigned(
    s: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
    max: impl Into<u64>,
    clamped: &mut bool,
) -> u64 {
    // SAFETY: the caller's contract is this function's.
    let Some(parsed) = (unsafe { parse_with_end(s, endptr, base) }) else { return 0 };
    // The limit comes as the C type's, which may be narrower than `u64`.
    let max: u64 = max.into();
    if parsed.overflow || parsed.magnitude > max {
        *clamped = true;
        errno::set(ERANGE);
        return max;
    }
    // Negation in the type, which is modulo `max + 1`.
    if parsed.negative { parsed.magnitude.wrapping_neg() & max } else { parsed.magnitude }
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use super::*;
    use crate::support::errno::tests as errno_tests;

    /// `parse` of `s` in `base`: the magnitude with its sign, whether the
    /// digits overflowed, and the offset of the end.
    fn p(s: &CStr, base: u8) -> (i128, bool, usize) {
        // SAFETY: `s` is NUL-terminated.
        let parsed = unsafe { parse(s.as_ptr(), base) };
        let magnitude = i128::from(parsed.magnitude);
        (
            if parsed.negative { -magnitude } else { magnitude },
            parsed.overflow,
            parsed.end.addr() - s.as_ptr().addr(),
        )
    }

    #[test]
    fn parse_reads_white_space_sign_prefix_and_digits() {
        assert_eq!(p(c"42", 10), (42, false, 2));
        assert_eq!(p(c" \t\n\x0b\x0c\r-17x", 10), (-17, false, 9));
        assert_eq!(p(c"+8", 10), (8, false, 2));
        assert_eq!(p(c"z", 36), (35, false, 1));
        assert_eq!(p(c"Zz", 36), (35 * 36 + 35, false, 2));
        assert_eq!(p(c"0F5F", 16), (0x0f5f, false, 4));
        assert_eq!(p(c"0x1234", 16), (0x1234, false, 6));
        assert_eq!(p(c"0X1234", 16), (0x1234, false, 6));
        assert_eq!(p(c"  15437", 8), (0o15437, false, 7));
        assert_eq!(p(c"19", 8), (1, false, 1));
        assert_eq!(p(c"101", 2), (5, false, 3));
    }

    #[test]
    fn parse_takes_the_base_from_the_prefix_for_base_0() {
        assert_eq!(p(c"  1", 0), (1, false, 3));
        assert_eq!(p(c"0x1f", 0), (0x1f, false, 4));
        assert_eq!(p(c"017", 0), (0o17, false, 3));
        assert_eq!(p(c"0", 0), (0, false, 1));
        assert_eq!(p(c"08", 0), (0, false, 1));
    }

    #[test]
    fn parse_ends_at_the_start_without_digits_and_after_a_lone_0x() {
        assert_eq!(p(c"", 10), (0, false, 0));
        assert_eq!(p(c"  ", 10), (0, false, 0));
        assert_eq!(p(c"-", 10), (0, false, 0));
        assert_eq!(p(c"- 1", 10), (0, false, 0));
        assert_eq!(p(c"abc", 10), (0, false, 0));
        // "0x" with no hex digit after it is the number 0, then an "x".
        assert_eq!(p(c"0xz", 16), (0, false, 1));
        assert_eq!(p(c"0x", 0), (0, false, 1));
        assert_eq!(p(c"-0x", 16), (0, false, 2));
    }

    #[test]
    fn parse_saturates_past_u64() {
        assert_eq!(p(c"18446744073709551615", 10), (i128::from(u64::MAX), false, 20));
        assert_eq!(p(c"18446744073709551616", 10), (i128::from(u64::MAX), true, 20));
        assert_eq!(p(c"-99999999999999999999999", 10), (-i128::from(u64::MAX), true, 24));
    }

    fn with_end(s: &CStr, convert: impl FnOnce(*mut *mut c_char) -> i128) -> (i128, usize) {
        let mut end = core::ptr::null_mut();
        let value = convert(&raw mut end);
        (value, end.addr() - s.as_ptr().addr())
    }

    fn sl(s: &CStr, base: c_int, min: i64, max: i64) -> (i128, usize, bool) {
        let mut clamped = false;
        let (value, end) = with_end(s, |end| {
            // SAFETY: `s` is NUL-terminated and `end` is writable.
            i128::from(unsafe { signed(s.as_ptr(), end, base, min, max, &mut clamped) })
        });
        (value, end, clamped)
    }

    fn ul(s: &CStr, base: c_int, max: u64) -> (i128, usize, bool) {
        let mut clamped = false;
        let (value, end) = with_end(s, |end| {
            // SAFETY: `s` is NUL-terminated and `end` is writable.
            i128::from(unsafe { unsigned(s.as_ptr(), end, base, max, &mut clamped) })
        });
        (value, end, clamped)
    }

    #[test]
    fn signed_clamps_to_the_range_with_erange() {
        let _errno = errno_tests::take();
        let (min, max) = (i64::from(i32::MIN), i64::from(i32::MAX));
        assert_eq!(sl(c"2147483647", 0, min, max), (2_147_483_647, 10, false));
        assert_eq!(sl(c"-2147483648", 0, min, max), (-2_147_483_648, 11, false));
        assert_eq!(errno_tests::get(), 0);
        assert_eq!(sl(c"2147483648", 0, min, max), (2_147_483_647, 10, true));
        assert_eq!(errno_tests::get(), ERANGE);
        errno::set(0);
        assert_eq!(sl(c"-2147483649", 0, min, max), (-2_147_483_648, 11, true));
        assert_eq!(errno_tests::get(), ERANGE);
        errno::set(0);
        assert_eq!(sl(c"-99999999999999999999999", 10, min, max), (-2_147_483_648, 24, true));
        assert_eq!(errno_tests::get(), ERANGE);
        assert_eq!(sl(c"-9223372036854775808", 10, i64::MIN, i64::MAX).0, i128::from(i64::MIN));
    }

    #[test]
    fn unsigned_negates_in_the_type_and_clamps_with_erange() {
        let _errno = errno_tests::take();
        let max = u64::from(u32::MAX);
        assert_eq!(ul(c"4294967295", 0, max), (4_294_967_295, 10, false));
        assert_eq!(ul(c"-1", 0, max), (4_294_967_295, 2, false));
        assert_eq!(ul(c"-2", 0, max), (4_294_967_294, 2, false));
        assert_eq!(ul(c"-2147483648", 0, max), (2_147_483_648, 11, false));
        assert_eq!(ul(c"-2147483649", 0, max), (2_147_483_647, 11, false));
        assert_eq!(errno_tests::get(), 0);
        assert_eq!(ul(c"4294967296", 0, max), (4_294_967_295, 10, true));
        assert_eq!(errno_tests::get(), ERANGE);
        errno::set(0);
        assert_eq!(ul(c"-4294967296", 0, max), (4_294_967_295, 11, true));
        assert_eq!(errno_tests::get(), ERANGE);
        assert_eq!(ul(c"18446744073709551615", 0, u64::MAX), (i128::from(u64::MAX), 20, false));
    }

    #[test]
    fn an_invalid_base_gives_0_at_the_start_with_einval() {
        let _errno = errno_tests::take();
        assert_eq!(sl(c"123", 37, i64::MIN, i64::MAX), (0, 0, false));
        assert_eq!(errno_tests::get(), EINVAL);
        errno::set(0);
        assert_eq!(ul(c"123", 1, u64::MAX), (0, 0, false));
        assert_eq!(errno_tests::get(), EINVAL);
        errno::set(0);
        assert_eq!(ul(c"123", -1, u64::MAX), (0, 0, false));
        assert_eq!(errno_tests::get(), EINVAL);
    }

    #[test]
    fn a_null_endptr_is_not_written() {
        let mut clamped = false;
        // SAFETY: NUL-terminated, and a null `endptr` is allowed.
        let value =
            unsafe { signed(c"12".as_ptr(), core::ptr::null_mut(), 10, 0, 100, &mut clamped) };
        assert_eq!(value, 12);
    }
}
