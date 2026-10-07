// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::mem::size_of;

/// A function in `.preinit_array` or `.init_array`: the compiler puts C++
/// constructors there, and C functions marked `__attribute__((constructor))`.
type Constructor = unsafe extern "C" fn();

// The bounds the firmware's linker script gives the two arrays: see
// `include/rivet_init.h`. Each is an address to compute with, not an array to
// read through, so it is declared empty.
#[cfg(target_os = "none")]
unsafe extern "C" {
    static __preinit_array_start: [Constructor; 0];
    static __preinit_array_end: [Constructor; 0];
    static __init_array_start: [Constructor; 0];
    static __init_array_end: [Constructor; 0];
}

/// Calls the constructors the program was linked with: every function in
/// `.preinit_array`, then every function in `.init_array`, each array in
/// order. The startup code of a C runtime calls it once `.data` and `.bss`
/// are set up, before `main`. It does not call `_init`: that comes from
/// `crti.o`, which firmware linked without the C start files does not have.
///
/// # Safety
///
/// `.data` and `.bss` are initialized, the linker script bounds the two
/// arrays with the symbols `include/rivet_init.h` names, and this runs once.
#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __libc_init_array() {
    // SAFETY: the linker script bounds each array with these symbols, and the
    // caller runs this once, with memory initialized.
    unsafe {
        run((&raw const __preinit_array_start).cast(), (&raw const __preinit_array_end).cast());
        run((&raw const __init_array_start).cast(), (&raw const __init_array_end).cast());
    }
}

/// Calls each constructor from `start` up to `end`, in order. An `end` below
/// `start` holds nothing.
///
/// # Safety
///
/// `start` and `end` bound an array of constructors that may be called now.
#[cfg_attr(not(target_os = "none"), allow(dead_code))]
unsafe fn run(start: *const Constructor, end: *const Constructor) {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        start.addr() <= end.addr(),
        "__libc_init_array: an array's end is not below its start"
    );
    let count = end.addr().saturating_sub(start.addr()) / size_of::<Constructor>();
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(count > 0, "__libc_init_array: runs a constructor");
    for i in 0..count {
        // The bounds are addresses the linker computed, not an object `start`
        // points into, so the step is not an in-bounds offset of one.
        let entry = start.wrapping_add(i);
        // SAFETY: `i < count`, so the entry lies in the caller's array, and
        // the caller guarantees it may be called.
        unsafe { (*entry)() };
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// How far the constructors got: 1 after `first`, 2 after `second` ran
    /// after it.
    static STAGE: AtomicUsize = AtomicUsize::new(0);

    extern "C" fn first() {
        let _ = STAGE.compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed);
    }

    extern "C" fn second() {
        let _ = STAGE.compare_exchange(1, 2, Ordering::Relaxed, Ordering::Relaxed);
    }

    #[test]
    fn run_calls_each_constructor_in_order() {
        let array: [Constructor; 2] = [first, second];
        let start = array.as_ptr();
        // SAFETY: an empty range, an inverted one, then the two constructors,
        // which may be called.
        unsafe {
            run(start, start);
            run(start.wrapping_add(2), start);
            assert_eq!(STAGE.load(Ordering::Relaxed), 0);
            run(start, start.wrapping_add(2));
        }
        assert_eq!(STAGE.load(Ordering::Relaxed), 2);
    }
}
