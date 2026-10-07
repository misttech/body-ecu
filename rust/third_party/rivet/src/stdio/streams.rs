// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::support::file::{FILE, Stream};
use crate::support::os_util::Console;

/// Writes the `n` bytes at `s` to the console `ctx` names: the firmware's
/// `write` to standard output or standard error.
unsafe extern "C" fn console_write(ctx: *mut c_void, s: *const c_char, n: usize) -> c_int {
    let console = if ctx.is_null() { Console::STDOUT } else { Console::STDERR };
    // SAFETY: rivet passes `n` readable bytes at `s`.
    let bytes = unsafe { core::slice::from_raw_parts(s.cast::<u8>(), n) };
    match console.write_bytes(bytes) {
        // rivet passes at most `INT_MAX` bytes.
        Ok(()) => n as c_int,
        Err(()) => -1,
    }
}

/// The marker `console_write` takes for standard error; null is standard
/// output. It is never dereferenced.
const STDERR_MARK: *mut c_void = ptr::dangling_mut();

/// `stdout`: the firmware's `write` to standard output, until firmware
/// assigns it another stream.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
#[allow(non_upper_case_globals)]
pub static __rivet_stdout: Stream = Stream::new(FILE::new(console_write, ptr::null_mut()));

/// `stderr`: the firmware's `write` to standard error, until firmware assigns
/// it another stream.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
#[allow(non_upper_case_globals)]
pub static __rivet_stderr: Stream = Stream::new(FILE::new(console_write, STDERR_MARK));

/// `stdout`, as C's macro names it.
pub fn stdout() -> *mut FILE {
    __rivet_stdout.as_ptr()
}

/// `stderr`, as C's macro names it.
pub fn stderr() -> *mut FILE {
    __rivet_stderr.as_ptr()
}
