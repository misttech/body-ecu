// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use crate::support::file;
use crate::support::stdout_lock;
use crate::support::symbolize::{self, Capture};

/// Prints the backtrace from the caller's call site to `stderr`, as
/// symbolizer markup, under the standard I/O lock.
fn print(capture: Capture) {
    stdout_lock::locked(|| {
        // SAFETY: `stderr` lives for the program, and the lock keeps rivet's
        // other functions off it.
        if let Some(stream) = unsafe { file::stream(crate::stdio::stderr()) } {
            symbolize::print(capture, |s: &str| {
                let _ = stream.write_all(s.as_bytes());
            });
        }
    });
}

#[cfg(all(target_os = "none", any(target_arch = "arm", target_arch = "riscv32")))]
extern "C" fn print_from(pc: usize, fp: usize, sp: usize) {
    print(Capture { pc, fp, sp });
}

#[cfg(all(target_os = "none", any(target_arch = "arm", target_arch = "riscv32")))]
symbolize::capture_entry! {
    /// Prints the backtrace from the call site to `stderr`, as symbolizer
    /// markup: the image's context, the caller, the frames its frame records
    /// lead to, and, if they stop early, stack words that point into code.
    pub fn rivet_backtrace() => print_from
}

/// Prints the backtrace from here to `stderr`, as symbolizer markup. On a
/// target without the naked entry point, the backtrace starts inside rivet.
#[cfg(not(all(target_os = "none", any(target_arch = "arm", target_arch = "riscv32"))))]
pub extern "C" fn rivet_backtrace() {
    print(symbolize::here());
}
