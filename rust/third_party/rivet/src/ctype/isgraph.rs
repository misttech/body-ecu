// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_int;

use crate::support::ctype_utils;

/// Whether `c` is a printing character other than space: 0x21 to 0x7e.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn isgraph(c: c_int) -> c_int {
    c_int::from(ctype_utils::byte(c).is_some_and(|b| b.is_ascii_graphic()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctype::tests::{accepted, every};

    #[test]
    fn isgraph_matches_the_c_locale_for_every_value() {
        assert_eq!(accepted(isgraph), every(|b| (0x21..=0x7e).contains(&b)));
    }
}
