// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_int, c_void};

use crate::support::mem_ops::{compare, cover_compare};

/// Compares `n` bytes as `unsigned char`. Returns the difference of the first
/// pair that differs, or 0.
///
/// # Safety
///
/// `a` and `b` are readable for `n` bytes.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int {
    let x = a.cast::<u8>();
    let y = b.cast::<u8>();
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        n == 0 || (!a.is_null() && !b.is_null()),
        "memcmp: a and b are non-null when n is not 0"
    );
    cover_compare!("memcmp", x, y, n);
    // SAFETY: both ranges are readable for `n` bytes.
    unsafe { compare(x, y, n) }
}

#[cfg(test)]
mod tests {
    use core::ffi::c_int;

    use super::*;

    fn cmp(a: &[u8], b: &[u8], n: usize) -> c_int {
        assert!(n <= a.len() && n <= b.len());
        // SAFETY: `n` bytes of each slice are readable.
        unsafe { memcmp(a.as_ptr().cast(), b.as_ptr().cast(), n) }
    }

    #[test]
    fn memcmp_compares_as_unsigned_char() {
        let a = [0x01_u8, 0x80];
        let b = [0x01_u8, 0x7f];
        assert!(cmp(&a, &b, 2) > 0);
        assert_eq!(cmp(&a, &b, 1), 0);
        assert_eq!(cmp(&a, &b, 0), 0);
    }
}
