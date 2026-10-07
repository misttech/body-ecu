// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Fuchsia's `zr` crate, vendored in full. Module sources match upstream,
//! except the test edits recorded in `README.md`. This root matches upstream's
//! exports and also exports `LossyUtf8`, which upstream leaves private.

#![no_std]

mod defer;
mod lossy_utf8;
mod opaque;
mod opaque_bytes;
mod pin_init;
mod ptr;
mod static_assert;
mod string;

pub use defer::{Deferred, defer};
pub use lossy_utf8::{LossyUtf8, from_utf8_lossy};
pub use opaque::{Opaque, OpaqueFacade};
pub use opaque_bytes::OpaqueBytes;
pub use ptr::{AtomicConstPtr, ToMutPtr, slice_from_raw_parts, slice_from_raw_parts_mut};
pub use string::{parse_usize, to_array};

#[cfg(test)]
mod tests {
    extern crate std;

    use std::format;

    use super::from_utf8_lossy;

    crate::static_assert!(core::mem::size_of::<u32>() == 4);
    crate::static_assert_size_and_align!(u64, 8, core::mem::align_of::<u64>());

    #[test]
    fn valid_utf8_formats_unchanged() {
        assert_eq!(format!("{}", from_utf8_lossy(b"GT1151")), "GT1151");
    }

    #[test]
    fn each_invalid_sequence_becomes_one_replacement_character() {
        assert_eq!(format!("{}", from_utf8_lossy(b"a\xffb\xc3")), "a\u{FFFD}b\u{FFFD}");
    }

    #[test]
    fn parse_usize_reads_a_decimal() {
        assert_eq!(crate::parse_usize("1151"), Some(1151));
        assert_eq!(crate::parse_usize(""), None);
        assert_eq!(crate::parse_usize("12a"), None);
    }

    #[test]
    fn to_array_copies_bytes_and_leaves_a_nul() {
        assert_eq!(crate::to_array::<8>("GT"), *b"GT\0\0\0\0\0\0");
    }

    #[test]
    fn opaque_get_points_at_the_stored_value() {
        let value = crate::Opaque::new(7u32);
        // SAFETY: `value` owns the `u32`, and this test does not alias it.
        assert_eq!(unsafe { *value.get() }, 7);
    }
}
