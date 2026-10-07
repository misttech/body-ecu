// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;

use super::memcpy;
use crate::support::mem_ops::{copy_backward, copy_forward, cover_copy};

/// Copies `n` bytes from `src` to `dst`, which may overlap. Returns `dst`.
///
/// # Safety
///
/// `src` is readable and `dst` writable for `n` bytes.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
// The run-time ABI's helpers in `crate::aeabi` forward here: one body for all of
// them, rather than a copy inlined into each.
#[cfg_attr(all(target_arch = "arm", target_os = "none"), inline(never))]
pub unsafe extern "C" fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    let d = dst.cast::<u8>();
    let s = src.cast::<u8>();
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        n == 0 || (!dst.is_null() && !src.is_null()),
        "memmove: dst and src are non-null when n is not 0"
    );
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(
        d.cast_const() > s && crate::support::property::overlaps(d.cast_const(), n, s, n),
        "memmove: copies backward over an overlap"
    );
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(
        d.cast_const() < s && crate::support::property::overlaps(d.cast_const(), n, s, n),
        "memmove: copies forward over an overlap"
    );
    // SAFETY: both ranges are valid for `n` bytes. The copy walks away from any
    // overlap, so each byte is read before a write reaches it: up, as `memcpy`
    // does, when `dst` is at or below `src`, and down otherwise.
    unsafe {
        if d.cast_const() <= s {
            if cfg!(feature = "forkpoint") {
                // `memcpy` asserts that its ranges do not overlap, which a
                // forward `memmove` may; copy the same way without it.
                cover_copy!("memmove", " forward", d, s, n);
                copy_forward(d, s, n);
                return dst;
            }
            // Returning its result lets the call be a jump.
            return memcpy(dst, src, n);
        }
        cover_copy!("memmove", " backward", d, s, n);
        copy_backward(d, s, n);
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memmove_handles_overlap_in_both_directions() {
        let mut buf = *b"123456";
        let base = buf.as_mut_ptr();
        // SAFETY: both ranges lie in the 6 bytes of `buf`.
        unsafe { memmove(base.add(2).cast(), base.cast(), 4) };
        assert_eq!(&buf, b"121234");
        let mut buf = *b"123456";
        let base = buf.as_mut_ptr();
        // SAFETY: both ranges lie in the 6 bytes of `buf`.
        unsafe { memmove(base.cast(), base.add(2).cast(), 4) };
        assert_eq!(&buf, b"345656");
    }
}
