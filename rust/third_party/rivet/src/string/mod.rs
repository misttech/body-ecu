// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `string.h`.
//!
//! The memory functions share the block patterns in `support::mem_ops`; the
//! string functions walk raw pointers one byte at a time. None can use slices,
//! `CStr`, or `core::ptr::copy` and its relatives: those lower to calls to
//! `memcpy`, `memmove`, `memset`, `memcmp`, or `strlen`, which are the
//! functions being defined here. Comparisons read `c_char` through `u8`
//! pointers: C compares strings as `unsigned char`, and `c_char` is signed on
//! some targets.

mod memchr;
mod memcmp;
mod memcpy;
mod memmove;
mod memset;
mod stpcpy;
mod strcat;
mod strchr;
mod strcmp;
mod strcpy;
mod strlen;
mod strncat;
mod strncmp;
mod strncpy;
mod strnlen;
mod strrchr;
mod strstr;

pub use memchr::memchr;
pub use memcmp::memcmp;
pub use memcpy::memcpy;
pub use memmove::memmove;
pub use memset::memset;
pub use stpcpy::stpcpy;
pub use strcat::strcat;
pub use strchr::strchr;
pub use strcmp::strcmp;
pub use strcpy::strcpy;
pub use strlen::strlen;
pub use strncat::strncat;
pub use strncmp::strncmp;
pub use strncpy::strncpy;
pub use strnlen::strnlen;
pub use strrchr::strrchr;
pub use strstr::strstr;
