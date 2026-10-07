// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_int;

use crate::support::errno::{self, EIO};
use crate::support::file::{self, FILE};
use crate::support::stdout_lock;

/// Writes `c`, converted to `unsigned char`, to `stream`. Returns the byte
/// written, or `EOF` with `errno` set to `EIO` if the stream failed.
///
/// # Safety
///
/// `stream` is a stream, such as `stdout`, that lives for the call.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn fputc(c: c_int, stream: *mut FILE) -> c_int {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(!stream.is_null(), "fputc: stream is non-null");
    let byte = c as u8;
    let written = stdout_lock::locked(|| {
        // SAFETY: `stream` lives for the call, per the caller, and the lock
        // keeps rivet's other functions off it.
        unsafe { file::stream(stream) }.ok_or(()).and_then(|f| f.write_all(&[byte]))
    });
    match written {
        Ok(()) => c_int::from(byte),
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
    fn fputc_writes_the_low_byte_or_reports_eof() {
        let mut captured = Captured::new();
        let mut file = capturing(&mut captured);
        // SAFETY: `file` lives for each call.
        unsafe {
            assert_eq!(fputc(0x1_41, &raw mut file), 0x41);
            assert_eq!(fputc(c_int::from(b'\n'), &raw mut file), 0x0a);
        }
        assert_eq!(captured.text(), b"A\n");
        let mut closed = crate::support::file::CLOSED;
        let _errno = crate::support::errno::tests::take();
        // SAFETY: as above.
        assert_eq!(unsafe { fputc(1, &raw mut closed) }, super::super::EOF);
        assert_eq!(crate::support::errno::tests::get(), EIO);
    }
}
