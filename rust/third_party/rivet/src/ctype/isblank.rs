// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_int;

use crate::support::ctype_utils;

/// Whether `c` is a blank, the white space within a line: space or `\t`.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn isblank(c: c_int) -> c_int {
    c_int::from(ctype_utils::byte(c).is_some_and(|b| b == b' ' || b == b'\t'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctype::tests::{accepted, only};

    #[test]
    fn isblank_matches_the_c_locale_for_every_value() {
        assert_eq!(accepted(isblank), only(b" \t"));
    }
}
