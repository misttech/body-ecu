// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Helpers shared by the `ctype.h` functions.

use core::ffi::c_int;

/// `c` as a byte, or `None` if it is `EOF` or outside `unsigned char`.
pub(crate) fn byte(c: c_int) -> Option<u8> {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        c == -1 || u8::try_from(c).is_ok(),
        "ctype: c is EOF or an unsigned char"
    );
    u8::try_from(c).ok()
}
