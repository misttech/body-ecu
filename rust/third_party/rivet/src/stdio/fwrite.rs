// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;

use crate::support::errno::{self, EIO};
use crate::support::file::{self, FILE};
use crate::support::stdout_lock;

/// Writes `count` items of `size` bytes from `buf` to `stream`. Returns
/// `count`, or 0 with `errno` set to `EIO` if the stream failed: a stream
/// writes all it is given or reports a failure, so no count in between is
/// known. Writes nothing and returns 0 when either is 0.
///
/// # Safety
///
/// `buf` is readable for `size * count` bytes, and `stream` is a stream that
/// lives for the call.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn fwrite(
    buf: *const c_void,
    size: usize,
    count: usize,
    stream: *mut FILE,
) -> usize {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        !stream.is_null() && (size == 0 || count == 0 || !buf.is_null()),
        "fwrite: stream is non-null, and buf when there is data"
    );
    // A buffer of `size * count` bytes exists, so the product fits.
    let Some(len) = size.checked_mul(count) else { return 0 };
    if len == 0 {
        return 0;
    }
    // SAFETY: `buf` is readable for `len` bytes, per the caller.
    let bytes = unsafe { core::slice::from_raw_parts(buf.cast::<u8>(), len) };
    let written = stdout_lock::locked(|| {
        // SAFETY: `stream` lives for the call, per the caller, and the lock
        // keeps rivet's other functions off it.
        unsafe { file::stream(stream) }.ok_or(()).and_then(|f| f.write_all(bytes))
    });
    match written {
        Ok(()) => count,
        Err(()) => {
            errno::set(EIO);
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::file::tests::{Captured, capturing};

    #[test]
    fn fwrite_writes_whole_items_or_none() {
        let mut captured = Captured::new();
        let mut file = capturing(&mut captured);
        let data = [1u16, 2, 3];
        // SAFETY: `data` holds 3 items of 2 bytes, and `file` lives for each call.
        unsafe {
            assert_eq!(fwrite(data.as_ptr().cast(), 2, 3, &raw mut file), 3);
            assert_eq!(fwrite(data.as_ptr().cast(), 0, 3, &raw mut file), 0);
            assert_eq!(fwrite(data.as_ptr().cast(), 2, 0, &raw mut file), 0);
        }
        assert_eq!(captured.len, 6);
        let mut closed = crate::support::file::CLOSED;
        let _errno = crate::support::errno::tests::take();
        // SAFETY: as above.
        assert_eq!(unsafe { fwrite(data.as_ptr().cast(), 2, 1, &raw mut closed) }, 0);
        assert_eq!(crate::support::errno::tests::get(), EIO);
    }
}
