// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_int;

use crate::support::ctype_utils;

/// Whether `c` is a letter, `A` to `Z` or `a` to `z`.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn isalpha(c: c_int) -> c_int {
    c_int::from(ctype_utils::byte(c).is_some_and(|b| b.is_ascii_alphabetic()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctype::tests::{accepted, only};

    #[test]
    fn isalpha_matches_the_c_locale_for_every_value() {
        assert_eq!(
            accepted(isalpha),
            only(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz")
        );
    }
}
