// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_int;

use crate::support::os_util::{self, Console};

/// The status `abort` exits with: 128 plus `SIGABRT`, as a shell reports it.
const ABORT_STATUS: c_int = 134;

/// Writes `abort()` to standard error and ends the program.
#[cfg(not(feature = "backtrace"))]
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn abort() -> ! {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_unreachable!("abort: called");
    // The program is ending: a failed write has nowhere to be reported.
    let _ = Console::STDERR.write_bytes(b"abort()\n");
    os_util::exit(ABORT_STATUS)
}

/// With the `backtrace` feature, `abort` also prints the backtrace from its
/// call site. Without it, none of this is compiled, and `abort` above is the
/// whole of it.
#[cfg(feature = "backtrace")]
mod with_backtrace {
    use super::{ABORT_STATUS, Console, os_util};
    use crate::support::symbolize::{self, Capture};

    /// Writes `abort()` and the backtrace from `capture` to standard error, and
    /// ends the program.
    pub(crate) fn abort_from(capture: Capture) -> ! {
        #[cfg(feature = "forkpoint")]
        forkpoint::assert_unreachable!("abort: called");
        // The program is ending: a failed write has nowhere to be reported.
        let _ = Console::STDERR.write_bytes(b"abort()\n");
        symbolize::print(capture, |s: &str| {
            let _ = Console::STDERR.write_bytes(s.as_bytes());
        });
        os_util::exit(ABORT_STATUS)
    }

    #[cfg(all(target_os = "none", any(target_arch = "arm", target_arch = "riscv32")))]
    extern "C" fn abort_captured(pc: usize, fp: usize, sp: usize) -> ! {
        abort_from(Capture { pc, fp, sp })
    }

    #[cfg(all(target_os = "none", any(target_arch = "arm", target_arch = "riscv32")))]
    symbolize::capture_entry! {
        /// Writes `abort()` and the backtrace from the call site to standard
        /// error, and ends the program.
        pub fn abort() -> ! => abort_captured
    }

    /// Writes `abort()` and the backtrace from here to standard error, and
    /// ends the program. On a target without the naked entry point, the
    /// backtrace starts inside rivet.
    #[cfg(not(all(target_os = "none", any(target_arch = "arm", target_arch = "riscv32"))))]
    #[cfg_attr(target_os = "none", unsafe(no_mangle))]
    pub extern "C" fn abort() -> ! {
        abort_from(symbolize::here())
    }
}

#[cfg(feature = "backtrace")]
pub use with_backtrace::abort;
#[cfg(feature = "backtrace")]
pub(crate) use with_backtrace::abort_from;
