// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_char, c_int};

use crate::support::cstr::CStrCursor;
use crate::support::errno::{self, EIO};
use crate::support::file::{self, FILE};
use crate::support::stdout_lock;

/// Writes `s`, without its NUL or a newline, to `stream`. Returns 0, or `EOF`
/// with `errno` set to `EIO` if the stream failed.
///
/// # Safety
///
/// `s` is a NUL-terminated string, and `stream` a stream that lives for the
/// call.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn fputs(s: *const c_char, stream: *mut FILE) -> c_int {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        !s.is_null() && !stream.is_null(),
        "fputs: s and stream are non-null"
    );
    // SAFETY: `s` is NUL-terminated, per the caller.
    let len = unsafe { CStrCursor::new(s) }.count();
    // SAFETY: `len` bytes of `s` were just read.
    let bytes = unsafe { core::slice::from_raw_parts(s.cast::<u8>(), len) };
    let written = stdout_lock::locked(|| {
        // SAFETY: `stream` lives for the call, per the caller, and the lock
        // keeps rivet's other functions off it.
        unsafe { file::stream(stream) }.ok_or(()).and_then(|f| f.write_all(bytes))
    });
    match written {
        Ok(()) => 0,
        Err(()) => {
            errno::set(EIO);
            super::EOF
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::file::tests::{Captured, capturing};

    #[test]
    fn fputs_writes_the_string_without_a_newline() {
        let mut captured = Captured::new();
        let mut file = capturing(&mut captured);
        // SAFETY: NUL-terminated, and `file` lives for each call.
        unsafe {
            assert_eq!(fputs(c"line".as_ptr(), &raw mut file), 0);
            assert_eq!(fputs(c"".as_ptr(), &raw mut file), 0);
        }
        assert_eq!(captured.text(), b"line");
        let mut closed = crate::support::file::CLOSED;
        let _errno = crate::support::errno::tests::take();
        // SAFETY: as above.
        assert_eq!(unsafe { fputs(c"x".as_ptr(), &raw mut closed) }, super::super::EOF);
        assert_eq!(crate::support::errno::tests::get(), EIO);
    }
}
