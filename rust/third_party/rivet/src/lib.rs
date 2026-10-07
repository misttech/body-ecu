// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Rivet: a minimal freestanding C library for embedded firmware.
//!
//! It provides what firmware, the C code it links, and compiler-generated code
//! call: the `string.h` and `ctype.h` functions, the `printf` family,
//! `puts`, `putchar`, `abort`, the `stdlib.h` number conversions and
//! sorted-array functions, `errno`, the `assert` failure path, and
//! `__libc_init_array` for startup code. The firmware supplies the two calls
//! a C library makes into its platform, POSIX `write` and `_exit` (declared
//! in `include/unistd.h`), and its own `free`; a kernel with threads also
//! names the running thread's `errno` through `rivet_errno.h`. The C
//! declarations are in `include/`.
//!
//! The C symbols are exported only for a bare-metal target (`target_os =
//! "none"`). A host build, such as `cargo test`, keeps them as ordinary Rust
//! functions so they do not replace the host's C library.

#![no_std]
// memcpy, memmove, memset, and memcmp are defined here: the compiler must not lower
// their loops back into calls to themselves.
#![no_builtins]

// The run-time ABI's memory helpers exist only for bare-metal Arm.
#[cfg(all(target_arch = "arm", target_os = "none"))]
mod aeabi;
mod assert;
mod ctype;
mod errno;
#[cfg(feature = "cmpctmalloc")]
pub mod heap;
#[cfg(feature = "backtrace")]
mod rivet_backtrace;
mod rivet_errno;
#[cfg(feature = "cmpctmalloc")]
mod rivet_heap;
mod rivet_init;
mod rivet_stdio;
mod stdio;
mod stdlib;
mod string;
mod support;

#[cfg(all(target_arch = "arm", target_os = "none"))]
pub use aeabi::{
    __aeabi_memclr, __aeabi_memclr4, __aeabi_memclr8, __aeabi_memcpy, __aeabi_memcpy4,
    __aeabi_memcpy8, __aeabi_memmove, __aeabi_memmove4, __aeabi_memmove8, __aeabi_memset,
    __aeabi_memset4, __aeabi_memset8,
};
pub use assert::__assert_func;
pub use ctype::{
    isalnum, isalpha, isblank, iscntrl, isdigit, isgraph, islower, isprint, ispunct, isspace,
    isupper, isxdigit, tolower, toupper,
};
pub use errno::__errno_location;
#[cfg(feature = "backtrace")]
pub use rivet_backtrace::{rivet_backtrace, rivet_backtrace_set_hooks};
pub use rivet_errno::rivet_errno_set_hooks;
#[cfg(feature = "cmpctmalloc")]
pub use rivet_heap::{
    rivet_heap_add, rivet_heap_alloc, rivet_heap_free, rivet_heap_init, rivet_heap_set_hooks,
    rivet_heap_stats,
};
#[cfg(target_os = "none")]
pub use rivet_init::__libc_init_array;
pub use rivet_stdio::rivet_stdio_set_hooks;
pub use stdio::{
    __rivet_stderr, __rivet_stdout, eprint, fputc, fputs, fwrite, print, putc, putchar, puts,
    stderr, stdout,
};
#[cfg(target_os = "none")]
pub use stdio::{rust_vfprintf, rust_vsnprintf};
pub use stdlib::{
    abort, abs, atoi, bsearch, labs, llabs, qsort, strtol, strtoll, strtoul, strtoull,
};
#[cfg(feature = "cmpctmalloc")]
pub use stdlib::{aligned_alloc, calloc, free, free_sized, malloc, realloc};
pub use string::{
    memchr, memcmp, memcpy, memmove, memset, stpcpy, strcat, strchr, strcmp, strcpy, strlen,
    strncat, strncmp, strncpy, strnlen, strrchr, strstr,
};
pub use support::errno::rivet_errno_hooks;
pub use support::file::{FILE, Stream, WriteFn};
pub use support::stdout_lock::rivet_stdio_hooks;
#[cfg(feature = "backtrace")]
pub use support::symbolize::rivet_backtrace_hooks;

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_unreachable!("rivet: panicked");
    use core::fmt::Write;

    let mut stderr = support::os_util::Console::STDERR;
    // The program is ending: a failed write has nowhere to be reported.
    let _ = writeln!(stderr, "libc panic: {info}");
    #[cfg(feature = "backtrace")]
    support::symbolize::print(support::symbolize::here(), |s: &str| {
        let _ = support::os_util::Console::STDERR.write_bytes(s.as_bytes());
    });
    support::os_util::exit(134)
}
