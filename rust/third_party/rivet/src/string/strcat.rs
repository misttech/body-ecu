// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_char;

use super::{strcpy, strlen};

/// Appends `src`, with its NUL, to the string in `dst`. Returns `dst`.
///
/// # Safety
///
/// `dst` and `src` are NUL-terminated strings, `dst` is writable for
/// `strlen(dst) + strlen(src) + 1` bytes, and the two do not overlap.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strcat(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        !dst.is_null() && !src.is_null(),
        "strcat: dst and src are non-null"
    );
    // SAFETY: the caller's contract covers both calls.
    unsafe { strcpy(dst.add(strlen(dst)), src) };
    dst
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use super::*;

    #[test]
    fn strcat_appends_after_the_existing_string() {
        let mut dst = *b"ab\0\0\0\0";
        // SAFETY: `dst` is NUL-terminated with room for the 2 bytes appended.
        unsafe { strcat(dst.as_mut_ptr().cast(), c"\n".as_ptr()) };
        // SAFETY: `strcat` left `dst` NUL-terminated.
        let joined = unsafe { CStr::from_ptr(dst.as_ptr().cast()) };
        assert_eq!(joined, c"ab\n");
    }
}
