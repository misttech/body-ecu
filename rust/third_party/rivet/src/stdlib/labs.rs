// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_long;

/// The absolute value of `n`. `LONG_MIN` has none in `long`, and C leaves that
/// case undefined; this returns `LONG_MIN` rather than trapping.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn labs(n: c_long) -> c_long {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(n != c_long::MIN, "labs: n is not LONG_MIN");
    n.wrapping_abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labs_drops_the_sign() {
        assert_eq!(labs(0), 0);
        assert_eq!(labs(-7), 7);
        assert_eq!(labs(c_long::MIN + 1), c_long::MAX);
        assert_eq!(labs(c_long::MIN), c_long::MIN);
    }
}
