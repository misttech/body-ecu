// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_char;

use crate::support::cstr::{self, CStrCursor};

/// Copies `src`, with its NUL, to `dst`, as `strcpy` does, and returns a
/// pointer to the NUL written to `dst` rather than to `dst` itself, so that
/// the caller can go on appending there.
///
/// # Safety
///
/// `src` is a NUL-terminated string, `dst` is writable for `strlen(src) + 1`
/// bytes, and the two do not overlap.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn stpcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        !dst.is_null() && !src.is_null(),
        "stpcpy: dst and src are non-null"
    );
    // SAFETY: `src` is NUL-terminated, per the caller.
    let mut cursor = unsafe { CStrCursor::new(src) };
    // SAFETY: `dst` has room for `src`'s bytes and its NUL, and does not
    // overlap them, per the caller.
    unsafe { cstr::copy(dst, &mut cursor) };
    // The cursor now sits at the NUL: both strings are `position() + 1` bytes.
    let len = cursor.position();
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        !crate::support::property::overlaps(dst.cast_const().cast(), len + 1, src.cast(), len + 1),
        "stpcpy: dst and src do not overlap"
    );
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(len == 0, "stpcpy: copies an empty string");
    dst.wrapping_add(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stpcpy_copies_through_the_nul_and_returns_its_address() {
        let mut dst = [0x55_u8; 6];
        let base = dst.as_mut_ptr().cast::<c_char>();
        // SAFETY: `dst` holds the 4 bytes copied, and the source is NUL-terminated.
        let end = unsafe { stpcpy(base, c"abc".as_ptr()) };
        assert_eq!(end, base.wrapping_add(3));
        assert_eq!(&dst, b"abc\0\x55\x55");
        // SAFETY: `dst` holds the 1 byte copied.
        assert_eq!(unsafe { stpcpy(base, c"".as_ptr()) }, base);
        assert_eq!(dst[0], 0);
    }
}
