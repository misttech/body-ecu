// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_int;

use crate::support::file::FILE;

/// `fputc`, which C allows to be a macro; rivet's is a function.
///
/// # Safety
///
/// As for `fputc`.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn putc(c: c_int, stream: *mut FILE) -> c_int {
    // SAFETY: the caller's contract is `fputc`'s.
    unsafe { super::fputc(c, stream) }
}
