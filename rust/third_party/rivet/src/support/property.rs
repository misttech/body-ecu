// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Helpers for the Forkpoint properties the functions assert with the `forkpoint`
//! feature.

/// Whether the `a_len` bytes at `a` and the `b_len` bytes at `b` share an address.
pub(crate) fn overlaps(a: *const u8, a_len: usize, b: *const u8, b_len: usize) -> bool {
    let (a, b) = (a.addr(), b.addr());
    a_len > 0 && b_len > 0 && a < b.wrapping_add(b_len) && b < a.wrapping_add(a_len)
}
