// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `stdio.h`: streams, and the `printf` family over them.
//!
//! A stream, `FILE`, is a write callback and its context (`support::file`).
//! `stdout` and `stderr` are two streams whose callbacks call the firmware's
//! `write`, and firmware may give either another callback. Every output
//! function ends in a stream's callback, under the standard I/O lock, except
//! `snprintf` and its siblings, which write to the caller's buffer.
//!
//! Stable Rust can neither define a C variadic function nor read a `va_list`,
//! so `printf`, `fprintf`, `snprintf`, `sprintf`, and their `v` forms are C, in
//! `variadic.c`, which `build.rs` compiles into the archive for a bare-metal
//! target. They call [`rust_vfprintf`] and [`rust_vsnprintf`], which run the
//! formatter in `support::format` over the arguments `support::va_list` reads
//! back from the C side. A host build has neither the C file nor those two
//! functions; the formatter's tests cover it there.

use core::ffi::c_int;

mod fputc;
mod fputs;
mod fwrite;
mod print;
mod putc;
mod putchar;
mod puts;
mod streams;
#[cfg(target_os = "none")]
mod vfprintf;
#[cfg(target_os = "none")]
mod vsnprintf;

pub use fputc::fputc;
pub use fputs::fputs;
pub use fwrite::fwrite;
pub use print::{eprint, print};
pub use putc::putc;
pub use putchar::putchar;
pub use puts::puts;
pub use streams::{__rivet_stderr, __rivet_stdout, stderr, stdout};
#[cfg(target_os = "none")]
pub use vfprintf::rust_vfprintf;
#[cfg(target_os = "none")]
pub use vsnprintf::rust_vsnprintf;

/// C `EOF`.
pub(crate) const EOF: c_int = -1;

/// The `printf` result of a formatter's: the count, or -1 with `errno` set.
#[cfg(target_os = "none")]
fn result(result: Result<usize, crate::support::format::Error>) -> c_int {
    use crate::support::errno::{self, EINVAL, EIO, EOVERFLOW};
    use crate::support::format::Error;
    match result {
        // The formatter keeps the count within `int`.
        Ok(count) => count as c_int,
        Err(error) => {
            errno::set(match error {
                Error::Invalid => EINVAL,
                Error::Overflow => EOVERFLOW,
                Error::Output => EIO,
            });
            EOF
        }
    }
}
