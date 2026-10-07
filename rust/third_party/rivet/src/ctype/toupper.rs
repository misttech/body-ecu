// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_int;

use crate::support::ctype_utils;

/// `c` in uppercase if it is a lowercase letter, else `c` unchanged.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub extern "C" fn toupper(c: c_int) -> c_int {
    match ctype_utils::byte(c) {
        Some(b) => c_int::from(b.to_ascii_uppercase()),
        None => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ctype::tests::EOF;

    #[test]
    fn toupper_changes_only_lowercase_letters() {
        assert_eq!(toupper(c_int::from(b'q')), c_int::from(b'Q'));
        assert_eq!(toupper(c_int::from(b'Q')), c_int::from(b'Q'));
        for c in [EOF, 0, c_int::from(b'5'), c_int::from(b'@'), c_int::from(b'['), 0xc4] {
            assert_eq!(toupper(c), c);
        }
    }
}
