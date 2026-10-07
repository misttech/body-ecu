// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::{c_int, c_void};

use crate::support::mem_ops::{cover_fill, fill};

/// Fills `n` bytes at `dst` with `c` converted to `unsigned char`. Returns `dst`.
///
/// # Safety
///
/// `dst` is writable for `n` bytes.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
// The run-time ABI's helpers in `crate::aeabi` forward here: one body for all of
// them, rather than a copy inlined into each.
#[cfg_attr(all(target_arch = "arm", target_os = "none"), inline(never))]
pub unsafe extern "C" fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void {
    let d = dst.cast::<u8>();
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        n == 0 || !dst.is_null(),
        "memset: dst is non-null when n is not 0"
    );
    cover_fill!("memset", n);
    // C converts `c` to `unsigned char`, keeping its low byte.
    // SAFETY: `dst` is writable for `n` bytes.
    unsafe { fill(d, c as u8, n) };
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memset_stores_the_low_byte_of_c() {
        let mut buf = [0_u8; 4];
        // SAFETY: `buf` holds the 3 bytes written.
        unsafe { memset(buf.as_mut_ptr().cast(), 0x1ab, 3) };
        assert_eq!(buf, [0xab, 0xab, 0xab, 0]);
    }
}
