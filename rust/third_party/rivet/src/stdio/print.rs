// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::fmt::{self, Write as _};

use crate::support::file;
use crate::support::stdout_lock;

/// Writes `args` to `stdout`, as one piece under the standard I/O lock, so
/// Rust output and C's `printf` share the stream and never interleave.
pub fn print(args: fmt::Arguments<'_>) -> fmt::Result {
    write_locked(super::stdout(), args)
}

/// Writes `args` to `stderr`, as [`print`] does to `stdout`.
pub fn eprint(args: fmt::Arguments<'_>) -> fmt::Result {
    write_locked(super::stderr(), args)
}

fn write_locked(stream: *mut file::FILE, args: fmt::Arguments<'_>) -> fmt::Result {
    stdout_lock::locked(|| {
        // SAFETY: `stdout` and `stderr` live for the program, and the lock
        // keeps rivet's other functions off them.
        let file = unsafe { file::stream(stream) }.ok_or(fmt::Error)?;
        file.write_fmt(args)
    })
}
