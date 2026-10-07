// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_char;

use crate::support::cstr::CStrCursor;

/// Returns the number of bytes before the NUL that ends `s`.
///
/// # Safety
///
/// `s` is a NUL-terminated string.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn strlen(s: *const c_char) -> usize {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(!s.is_null(), "strlen: s is non-null");
    // SAFETY: `s` is NUL-terminated, per the caller.
    let mut s = unsafe { CStrCursor::new(s) };
    while s.peek() != 0 {
        s.advance();
    }
    s.position()
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use super::*;

    fn len(s: &CStr) -> usize {
        // SAFETY: `s` is NUL-terminated.
        unsafe { strlen(s.as_ptr()) }
    }

    #[test]
    fn strlen_counts_bytes_before_the_nul() {
        assert_eq!(len(c""), 0);
        assert_eq!(len(c"rivet"), 5);
    }
}
