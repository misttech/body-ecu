// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{CStr, c_char, c_int};
use core::fmt::{self, Write};

#[cfg(not(feature = "backtrace"))]
use crate::stdlib::abort;
use crate::support::os_util::Console;

/// Reports a failed `assert` on standard error and aborts.
///
/// # Safety
///
/// `file`, `function`, and `expression` are each null or a NUL-terminated
/// string, as the `assert` macro in `include/assert.h` passes them.
#[cfg(not(feature = "backtrace"))]
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __assert_func(
    file: *const c_char,
    line: c_int,
    function: *const c_char,
    expression: *const c_char,
) -> ! {
    // SAFETY: the caller passes null or NUL-terminated strings.
    let (file, function, expression) =
        unsafe { (c_bytes(file), c_bytes(function), c_bytes(expression)) };
    // The program is ending: a failed write has nowhere to be reported.
    let mut stderr = Console::STDERR;
    let _ = write_failure(&mut stderr, file, line, function, expression);
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_unreachable!("assert: an assertion failed");
    abort()
}

/// With the `backtrace` feature, a failed assertion also prints the backtrace
/// from its call site. Without it, none of this is compiled, and
/// `__assert_func` above is the whole of it.
#[cfg(feature = "backtrace")]
mod with_backtrace {
    use core::ffi::{c_char, c_int};

    use super::{Console, c_bytes, write_failure};
    use crate::stdlib::abort_from;
    use crate::support::symbolize::Capture;

    /// Writes the failed assertion to standard error, and aborts with the
    /// backtrace from `capture`.
    ///
    /// # Safety
    ///
    /// `file`, `function`, and `expression` are each null or a NUL-terminated
    /// string, as the `assert` macro in `include/assert.h` passes them.
    unsafe fn assert_failed(
        file: *const c_char,
        line: c_int,
        function: *const c_char,
        expression: *const c_char,
        capture: Capture,
    ) -> ! {
        // SAFETY: the caller passes null or NUL-terminated strings.
        let (file, function, expression) =
            unsafe { (c_bytes(file), c_bytes(function), c_bytes(expression)) };
        // The program is ending: a failed write has nowhere to be reported.
        let mut stderr = Console::STDERR;
        let _ = write_failure(&mut stderr, file, line, function, expression);
        #[cfg(feature = "forkpoint")]
        forkpoint::assert_unreachable!("assert: an assertion failed");
        abort_from(capture)
    }

    /// Reports a failed `assert` and the backtrace from here on standard
    /// error, and aborts. On a target without the naked entry point, the
    /// backtrace starts inside rivet.
    ///
    /// # Safety
    ///
    /// As for `assert_failed`.
    #[cfg(not(all(target_os = "none", any(target_arch = "arm", target_arch = "riscv32"))))]
    #[cfg_attr(target_os = "none", unsafe(no_mangle))]
    pub unsafe extern "C" fn __assert_func(
        file: *const c_char,
        line: c_int,
        function: *const c_char,
        expression: *const c_char,
    ) -> ! {
        // SAFETY: the caller's contract is `assert_failed`'s.
        unsafe {
            assert_failed(file, line, function, expression, crate::support::symbolize::here())
        }
    }

    #[cfg(all(target_os = "none", any(target_arch = "arm", target_arch = "riscv32")))]
    mod captured {
        use core::cell::UnsafeCell;
        use core::ffi::{c_char, c_int};

        use crate::support::symbolize::Capture;

        /// Where the naked `__assert_func` stores its caller's state, since its
        /// four arguments leave no register to pass it in.
        #[repr(transparent)]
        pub(super) struct Slot(UnsafeCell<Capture>);

        // SAFETY: only the naked entry writes the slot, right before the body
        // that reads it; two assertions failing at once garble only the
        // backtrace.
        unsafe impl Sync for Slot {}

        pub(super) static SLOT: Slot = Slot(UnsafeCell::new(Capture { pc: 0, fp: 0, sp: 0 }));

        /// The body of the naked `__assert_func`.
        ///
        /// # Safety
        ///
        /// As for `__assert_func`.
        unsafe extern "C" fn body(
            file: *const c_char,
            line: c_int,
            function: *const c_char,
            expression: *const c_char,
        ) -> ! {
            // SAFETY: the naked entry wrote the slot before jumping here.
            let capture = unsafe { SLOT.0.get().read_volatile() };
            // SAFETY: the caller's contract is `assert_failed`'s.
            unsafe { super::assert_failed(file, line, function, expression, capture) }
        }

        /// Reports a failed `assert` and the backtrace from its call site on
        /// standard error, and aborts.
        ///
        /// # Safety
        ///
        /// `file`, `function`, and `expression` are each null or a
        /// NUL-terminated string, as the `assert` macro in `include/assert.h`
        /// passes them.
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn __assert_func(
            file: *const c_char,
            line: c_int,
            function: *const c_char,
            expression: *const c_char,
        ) -> ! {
            // The caller's return address, frame pointer, and stack pointer go
            // to the slot through a scratch register, leaving the arguments in
            // theirs.
            #[cfg(target_arch = "arm")]
            core::arch::naked_asm!(
                "movw r12, :lower16:{slot}",
                "movt r12, :upper16:{slot}",
                "str lr, [r12]",
                "str r7, [r12, #4]",
                "str sp, [r12, #8]",
                "b {body}",
                slot = sym SLOT,
                body = sym body,
            );
            #[cfg(target_arch = "riscv32")]
            core::arch::naked_asm!(
                "la t0, {slot}",
                "sw ra, 0(t0)",
                "sw s0, 4(t0)",
                "sw sp, 8(t0)",
                "tail {body}",
                slot = sym SLOT,
                body = sym body,
            );
        }
    }

    #[cfg(all(target_os = "none", any(target_arch = "arm", target_arch = "riscv32")))]
    pub use captured::__assert_func;
}

#[cfg(feature = "backtrace")]
pub use with_backtrace::__assert_func;

/// # Safety
///
/// `s` is null or a NUL-terminated string that outlives the returned slice.
unsafe fn c_bytes<'a>(s: *const c_char) -> &'a [u8] {
    if s.is_null() {
        return b"?";
    }
    // SAFETY: `s` is non-null and NUL-terminated, per the caller.
    unsafe { CStr::from_ptr(s) }.to_bytes()
}

fn write_failure(
    out: &mut impl Write,
    file: &[u8],
    line: c_int,
    function: &[u8],
    expression: &[u8],
) -> fmt::Result {
    writeln!(
        out,
        "assert failed: {} {}:{} ({})",
        zr::from_utf8_lossy(function),
        zr::from_utf8_lossy(file),
        line,
        zr::from_utf8_lossy(expression)
    )
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::string::String;

    use super::*;

    #[test]
    fn a_failure_names_function_file_line_and_expression() {
        let mut out = String::new();
        write_failure(&mut out, b"main/main.c", 94, b"guiTask", b"buf1 != NULL").unwrap();
        assert_eq!(out, "assert failed: guiTask main/main.c:94 (buf1 != NULL)\n");
    }

    #[test]
    fn a_null_string_prints_as_a_question_mark() {
        // SAFETY: null is allowed.
        assert_eq!(unsafe { c_bytes(core::ptr::null()) }, b"?");
        // SAFETY: the literal is NUL-terminated and static.
        assert_eq!(unsafe { c_bytes(c"expr".as_ptr()) }, b"expr");
    }
}
