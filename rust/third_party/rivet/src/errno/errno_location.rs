// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_int;

use crate::support::errno;

/// The address of `errno`, which `include/errno.h` defines `errno` as: the
/// running thread's cell when the kernel has named one through
/// `rivet_errno_set_hooks`, else the program's one cell.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn __errno_location() -> *mut c_int {
    errno::location()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errno_location_is_one_address_for_the_program() {
        let _guard = crate::support::errno::tests::take();
        assert_eq!(__errno_location(), __errno_location());
        assert!(!__errno_location().is_null());
    }
}
