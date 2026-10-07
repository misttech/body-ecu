// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_char;

use crate::support::cstr::{self, CStrCursor};

/// Copies `src`, with its NUL, to `dst`. Returns `dst`.
///
/// # Safety
///
/// `src` is a NUL-terminated string, `dst` is writable for `strlen(src) + 1`
/// bytes, and the two do not overlap.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        !dst.is_null() && !src.is_null(),
        "strcpy: dst and src are non-null"
    );
    // SAFETY: `src` is NUL-terminated, per the caller.
    let mut cursor = unsafe { CStrCursor::new(src) };
    // SAFETY: `dst` has room for `src`'s bytes and its NUL, and does not
    // overlap them, per the caller.
    unsafe { cstr::copy(dst, &mut cursor) };
    // The cursor now sits at the NUL: both strings are `position() + 1` bytes.
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        !crate::support::property::overlaps(
            dst.cast_const().cast(),
            cursor.position() + 1,
            src.cast(),
            cursor.position() + 1
        ),
        "strcpy: dst and src do not overlap"
    );
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strcpy_copies_through_the_nul() {
        let mut dst = [0x55_u8; 6];
        // SAFETY: `dst` holds the 4 bytes copied, and the source is NUL-terminated.
        unsafe { strcpy(dst.as_mut_ptr().cast(), c"abc".as_ptr()) };
        assert_eq!(&dst, b"abc\0\x55\x55");
    }
}
