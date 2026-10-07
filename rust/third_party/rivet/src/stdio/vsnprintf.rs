// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_char, c_int, c_void};

use crate::support::format::{self, Buffer};
use crate::support::va_list::VaList;

/// `vsnprintf` for the C entry points in `variadic.c`: formats `format` with
/// the arguments in `va` into the first `n - 1` bytes of `buf`, then a NUL,
/// and returns how long the whole result is, as `snprintf` does. With an `n`
/// of 0 it writes nothing, and `buf` may be null. Returns -1 with `errno` set
/// for a conversion that is not supported (`EINVAL`) or a result past
/// `INT_MAX` bytes (`EOVERFLOW`).
///
/// # Safety
///
/// `buf` is writable for `n` bytes, or `n` is 0; `format` is a NUL-terminated
/// string; and `va` is the `struct rivet_va_list` of the running entry point,
/// whose arguments match the format's conversions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_vsnprintf(
    buf: *mut c_char,
    n: usize,
    format: *const c_char,
    va: *mut c_void,
) -> c_int {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        !format.is_null() && !va.is_null() && (n == 0 || !buf.is_null()),
        "vsnprintf: format and va are non-null, and buf when n is not 0"
    );
    // SAFETY: `buf` is writable for `n` bytes, or `n` is 0, per the caller.
    let mut buffer = unsafe { Buffer::new(buf, n) };
    // SAFETY: `va` is the running entry point's, per the caller.
    let mut args = unsafe { VaList::new(va) };
    // SAFETY: `format` is NUL-terminated and `va` matches it, per the caller.
    let result = unsafe { format::format(&mut buffer, format, &mut args) };
    buffer.finish();
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(
        result.is_ok_and(|len| len >= n),
        "vsnprintf: truncates a result longer than the buffer"
    );
    super::result(result)
}
