// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_int;

use crate::support::ctype_utils;

/// Whether `c` is a letter or a decimal digit.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn isalnum(c: c_int) -> c_int {
    c_int::from(ctype_utils::byte(c).is_some_and(|b| b.is_ascii_alphanumeric()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctype::tests::{accepted, only};

    #[test]
    fn isalnum_matches_the_c_locale_for_every_value() {
        assert_eq!(
            accepted(isalnum),
            only(b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz")
        );
    }
}
