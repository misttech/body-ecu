// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_int;

/// The absolute value of `n`. `INT_MIN` has none in `int`, and C leaves that
/// case undefined; this returns `INT_MIN` rather than trapping.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn abs(n: c_int) -> c_int {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(n != c_int::MIN, "abs: n is not INT_MIN");
    n.wrapping_abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abs_drops_the_sign() {
        assert_eq!(abs(0), 0);
        assert_eq!(abs(7), 7);
        assert_eq!(abs(-7), 7);
        assert_eq!(abs(c_int::MIN + 1), c_int::MAX);
        assert_eq!(abs(c_int::MIN), c_int::MIN);
    }
}
