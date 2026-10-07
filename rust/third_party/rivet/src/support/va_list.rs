// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The arguments of a C variadic call, read through `src/stdio/variadic.c`.
//!
//! Stable Rust cannot read a `va_list`, so the C entry points wrap theirs in
//! a `struct rivet_va_list` and pass its address here, and [`VaList`] asks
//! them for each argument through the `cpp_va_*` calls. The integer kinds
//! are numbered as the C file numbers them.

use core::ffi::{c_int, c_void};

use crate::support::format::{Args, Length};

/// `RIVET_VA_INT` and the rest, in `variadic.c`.
const VA_INT: c_int = 0;
const VA_LONG: c_int = 1;
const VA_LONG_LONG: c_int = 2;
const VA_INTMAX: c_int = 3;
const VA_SIZE: c_int = 4;
const VA_PTRDIFF: c_int = 5;

unsafe extern "C" {
    fn cpp_va_int(va: *mut c_void, kind: c_int) -> i64;
    fn cpp_va_uint(va: *mut c_void, kind: c_int) -> u64;
    fn cpp_va_pointer(va: *mut c_void) -> *const c_void;
}

/// A `struct rivet_va_list` of a C entry point, as the formatter's arguments.
pub(crate) struct VaList(*mut c_void);

impl VaList {
    /// The arguments behind `va`.
    ///
    /// # Safety
    ///
    /// `va` is the `struct rivet_va_list` of a running C entry point, whose
    /// arguments match the conversions that will read them.
    pub(crate) unsafe fn new(va: *mut c_void) -> Self {
        Self(va)
    }

    /// The kind `variadic.c` reads for `length`: `hh` and `h` are promoted
    /// to `int`.
    fn kind(length: Length) -> c_int {
        match length {
            Length::Char | Length::Short | Length::Int => VA_INT,
            Length::Long => VA_LONG,
            Length::LongLong => VA_LONG_LONG,
            Length::IntMax => VA_INTMAX,
            Length::Size => VA_SIZE,
            Length::PtrDiff => VA_PTRDIFF,
        }
    }
}

impl Args for VaList {
    fn int(&mut self, length: Length) -> i64 {
        // SAFETY: `va` is a live `struct rivet_va_list` with this argument next.
        unsafe { cpp_va_int(self.0, Self::kind(length)) }
    }

    fn uint(&mut self, length: Length) -> u64 {
        // SAFETY: as in `int`.
        unsafe { cpp_va_uint(self.0, Self::kind(length)) }
    }

    fn pointer(&mut self) -> *const c_void {
        // SAFETY: as in `int`.
        unsafe { cpp_va_pointer(self.0) }
    }
}
