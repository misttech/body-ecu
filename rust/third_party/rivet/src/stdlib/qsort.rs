// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;

use crate::support::smoothsort::{self, CompareFn};

/// Sorts the array of `nel` elements of `width` bytes at `base` in the order
/// `cmp` gives: `cmp(a, b)` is negative, zero, or positive as `a` orders
/// before, equal to, or after `b`. Elements that order equal may end in any
/// order. The sort is in place with no allocation, and takes O(n log n)
/// comparisons in the worst case and close to O(n) for a nearly sorted array.
///
/// # Safety
///
/// `base` holds `nel` writable elements of `width` bytes, and `cmp` reads only
/// its arguments. `cmp` is non-null.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn qsort(
    base: *mut c_void,
    nel: usize,
    width: usize,
    cmp: Option<CompareFn>,
) {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        cmp.is_some() && (nel == 0 || (!base.is_null() && width > 0)),
        "qsort: cmp is non-null, and base and width when nel is not 0"
    );
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(nel < 2, "qsort: sorts fewer than two elements");
    let Some(cmp) = cmp else { return };
    // An array of `nel` elements of `width` bytes exists, so its size fits.
    let Some(size) = nel.checked_mul(width) else { return };
    if size == 0 {
        return;
    }
    // SAFETY: the caller's contract is `sort`'s, and the size is checked.
    unsafe { smoothsort::sort(base.cast(), nel, width, cmp) }
}

#[cfg(test)]
mod tests {
    use core::ffi::c_int;

    use super::*;

    unsafe extern "C" fn compare_u16(a: *const c_void, b: *const c_void) -> c_int {
        // SAFETY: both point at `u16`s of the array under sort.
        let (a, b) = unsafe { (a.cast::<u16>().read(), b.cast::<u16>().read()) };
        a.cmp(&b) as c_int
    }

    #[test]
    fn qsort_orders_the_array_by_the_comparison() {
        let mut values = [5u16, 3, 9, 3, 0, 65535, 1];
        // SAFETY: `values` holds 7 writable `u16`s, which `compare_u16` reads.
        unsafe { qsort(values.as_mut_ptr().cast(), 7, 2, Some(compare_u16)) };
        assert_eq!(values, [0, 1, 3, 3, 5, 9, 65535]);
    }

    #[test]
    fn qsort_leaves_a_degenerate_array_alone() {
        let mut values = [2u16, 1];
        // SAFETY: a null `cmp`, no elements, and a zero width each sort nothing.
        unsafe {
            qsort(values.as_mut_ptr().cast(), 2, 2, None);
            qsort(values.as_mut_ptr().cast(), 0, 2, Some(compare_u16));
            qsort(values.as_mut_ptr().cast(), 2, 0, Some(compare_u16));
            qsort(values.as_mut_ptr().cast(), usize::MAX, 2, Some(compare_u16));
        }
        assert_eq!(values, [2, 1]);
    }
}
