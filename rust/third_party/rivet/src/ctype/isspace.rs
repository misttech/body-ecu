// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_int;

use crate::support::ctype_utils;

/// Whether `c` is white space: space, `\t`, `\n`, `\v`, `\f`, or `\r`. Unlike
/// `u8::is_ascii_whitespace`, this includes `\v`.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn isspace(c: c_int) -> c_int {
    c_int::from(ctype_utils::byte(c).is_some_and(|b| b == b' ' || (b'\t'..=b'\r').contains(&b)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctype::tests::{accepted, only};

    #[test]
    fn isspace_matches_the_c_locale_for_every_value() {
        assert_eq!(accepted(isspace), only(b" \t\n\x0b\x0c\r"));
    }
}
