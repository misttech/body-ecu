// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `ctype.h`: character classification and case mapping in the "C" locale.
//!
//! Each function takes a character as an `int` holding an `unsigned char` value or
//! `EOF`, as C does. Only ASCII classifies: letters, digits, punctuation, the
//! six white-space characters (`' '`, `\t`, `\n`, `\v`, `\f`, `\r`), and the
//! control characters, 0 to 0x1f and 0x7f. Every other value, including `EOF`
//! and bytes above 0x7f, falls in no class. The predicates return 1 or 0.

mod isalnum;
mod isalpha;
mod isblank;
mod iscntrl;
mod isdigit;
mod isgraph;
mod islower;
mod isprint;
mod ispunct;
mod isspace;
mod isupper;
mod isxdigit;
mod tolower;
mod toupper;

pub use isalnum::isalnum;
pub use isalpha::isalpha;
pub use isblank::isblank;
pub use iscntrl::iscntrl;
pub use isdigit::isdigit;
pub use isgraph::isgraph;
pub use islower::islower;
pub use isprint::isprint;
pub use ispunct::ispunct;
pub use isspace::isspace;
pub use isupper::isupper;
pub use isxdigit::isxdigit;
pub use tolower::tolower;
pub use toupper::toupper;

#[cfg(test)]
pub(crate) mod tests {
    use core::ffi::c_int;

    pub(crate) const EOF: c_int = -1;

    /// Every value a predicate accepts, from EOF through 255.
    pub(crate) fn accepted(predicate: extern "C" fn(c_int) -> c_int) -> [bool; 257] {
        core::array::from_fn(|i| {
            let result = predicate(i as c_int - 1);
            assert!(result == 0 || result == 1, "predicates return 0 or 1");
            result == 1
        })
    }

    /// The acceptance table of a predicate that accepts exactly the bytes
    /// `accept` holds for.
    pub(crate) fn every(accept: impl Fn(u8) -> bool) -> [bool; 257] {
        core::array::from_fn(|i| i > 0 && accept((i - 1) as u8))
    }

    /// The acceptance table of a predicate that accepts exactly `chars`.
    pub(crate) fn only(chars: &[u8]) -> [bool; 257] {
        every(|b| chars.contains(&b))
    }
}
