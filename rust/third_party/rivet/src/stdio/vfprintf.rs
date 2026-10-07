// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_char, c_int, c_void};

use crate::support::file::{self, Buffered, FILE};
use crate::support::format;
use crate::support::stdout_lock;
use crate::support::va_list::VaList;

/// `vfprintf` for the C entry points in `variadic.c`: formats `format` with
/// the arguments in `va` to `stream`, and returns how many bytes that is.
/// Returns -1 with `errno` set for a conversion that is not supported
/// (`EINVAL`), a result past `INT_MAX` bytes (`EOVERFLOW`), or a stream that
/// failed (`EIO`), after writing what came before.
///
/// # Safety
///
/// `stream` is a stream that lives for the call, `format` is a NUL-terminated
/// string, and `va` is the `struct rivet_va_list` of the running entry point,
/// whose arguments match the format's conversions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_vfprintf(
    stream: *mut FILE,
    format: *const c_char,
    va: *mut c_void,
) -> c_int {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        !stream.is_null() && !format.is_null() && !va.is_null(),
        "vfprintf: stream, format, and va are non-null"
    );
    // SAFETY: `va` is the running entry point's, per the caller.
    let mut args = unsafe { VaList::new(va) };
    // The whole result comes out together, under the standard I/O lock,
    // across the several writes its buffer takes.
    let result = stdout_lock::locked(|| {
        // SAFETY: `stream` lives for the call, per the caller, and the lock
        // keeps rivet's other functions off it.
        let Some(file) = (unsafe { file::stream(stream) }) else {
            return Err(format::Error::Output);
        };
        let mut out = Buffered::new(file);
        // SAFETY: `format` is NUL-terminated and `va` matches it, per the caller.
        let result = unsafe { format::format(&mut out, format, &mut args) };
        match (result, out.flush()) {
            (Ok(_), Err(())) => Err(format::Error::Output),
            (result, _) => result,
        }
    });
    super::result(result)
}
