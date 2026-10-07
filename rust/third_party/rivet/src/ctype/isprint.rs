// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_int;

use crate::support::ctype_utils;

/// Whether `c` is a printing character, space included: 0x20 to 0x7e.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn isprint(c: c_int) -> c_int {
    c_int::from(ctype_utils::byte(c).is_some_and(|b| (b' '..=b'~').contains(&b)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctype::tests::{accepted, every};

    #[test]
    fn isprint_matches_the_c_locale_for_every_value() {
        assert_eq!(accepted(isprint), every(|b| (0x20..=0x7e).contains(&b)));
    }
}
