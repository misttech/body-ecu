// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_longlong;

/// The absolute value of `n`. `LLONG_MIN` has none in `long long`, and C
/// leaves that case undefined; this returns `LLONG_MIN` rather than trapping.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn llabs(n: c_longlong) -> c_longlong {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(n != c_longlong::MIN, "llabs: n is not LLONG_MIN");
    n.wrapping_abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llabs_drops_the_sign() {
        assert_eq!(llabs(0), 0);
        assert_eq!(llabs(-7), 7);
        assert_eq!(llabs(c_longlong::MIN + 1), c_longlong::MAX);
        assert_eq!(llabs(c_longlong::MIN), c_longlong::MIN);
    }
}
