// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_int;

/// Writes `c`, converted to `unsigned char`, to `stdout`.
///
/// Returns the byte written, or `EOF` if the write failed.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn putchar(c: c_int) -> c_int {
    // SAFETY: `stdout` lives for the program.
    unsafe { super::fputc(c, super::stdout()) }
}
