// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;

use crate::support::mem_ops::{copy_forward, cover_copy};

/// Copies `n` bytes from `src` to `dst`. Returns `dst`.
///
/// # Safety
///
/// `src` is readable and `dst` writable for `n` bytes, and the two do not
/// overlap.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
// The run-time ABI's helpers in `crate::aeabi` forward here: one body for all of
// them, rather than a copy inlined into each.
#[cfg_attr(all(target_arch = "arm", target_os = "none"), inline(never))]
pub unsafe extern "C" fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    let d = dst.cast::<u8>();
    let s = src.cast::<u8>();
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        n == 0 || (!dst.is_null() && !src.is_null()),
        "memcpy: dst and src are non-null when n is not 0"
    );
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        !crate::support::property::overlaps(d.cast_const(), n, s, n),
        "memcpy: dst and src do not overlap"
    );
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(n == 0, "memcpy: copies zero bytes");
    cover_copy!("memcpy", "", d, s, n);
    // SAFETY: both ranges are valid for `n` bytes and do not overlap. `memmove`
    // also calls this when `dst` is at or below `src`, which `copy_forward`
    // allows.
    unsafe { copy_forward(d, s, n) };
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memcpy_copies_n_bytes_and_returns_dst() {
        let src = *b"abcdef";
        let mut dst = [0_u8; 6];
        // SAFETY: `src` holds 4 bytes, `dst` has room for them, and they do not overlap.
        let ret = unsafe { memcpy(dst.as_mut_ptr().cast(), src.as_ptr().cast(), 4) };
        assert_eq!(ret, dst.as_mut_ptr().cast());
        assert_eq!(&dst, b"abcd\0\0");
    }
}
