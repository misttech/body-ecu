// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_int;

use crate::support::ctype_utils;

/// Whether `c` is a control character: 0 to 0x1f, or 0x7f (`DEL`).
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn iscntrl(c: c_int) -> c_int {
    c_int::from(ctype_utils::byte(c).is_some_and(|b| b.is_ascii_control()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctype::tests::{accepted, every};

    #[test]
    fn iscntrl_matches_the_c_locale_for_every_value() {
        assert_eq!(accepted(iscntrl), every(|b| b < 0x20 || b == 0x7f));
    }
}
