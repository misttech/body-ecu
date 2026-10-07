// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_char, c_int};

use crate::support::cstr::CStrCursor;
use crate::support::errno::{self, EIO};
use crate::support::file;
use crate::support::stdout_lock;

/// Writes `s`, then a newline, to `stdout`. Returns 0, or `EOF` with `errno`
/// set to `EIO` if the write failed.
///
/// # Safety
///
/// `s` is a NUL-terminated string.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn puts(s: *const c_char) -> c_int {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(!s.is_null(), "puts: s is non-null");
    // SAFETY: `s` is NUL-terminated, per the caller.
    let len = unsafe { CStrCursor::new(s) }.count();
    // SAFETY: `len` bytes of `s` were just read.
    let bytes = unsafe { core::slice::from_raw_parts(s.cast::<u8>(), len) };
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(len == 0, "puts: writes only the newline");
    // The line and its newline come out together, under the standard I/O lock.
    let written = stdout_lock::locked(|| {
        // SAFETY: `stdout` lives for the program, and the lock keeps rivet's
        // other functions off it.
        let file = unsafe { file::stream(super::stdout()) }.ok_or(())?;
        file.write_all(bytes).and_then(|()| file.write_all(b"\n"))
    });
    match written {
        Ok(()) => 0,
        Err(()) => {
            errno::set(EIO);
            super::EOF
        }
    }
}
