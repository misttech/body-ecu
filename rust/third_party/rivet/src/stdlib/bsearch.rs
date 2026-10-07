// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

use core::ffi::c_void;

use crate::support::smoothsort::CompareFn;

/// Searches the sorted array of `nel` elements of `width` bytes at `base` for
/// an element that `cmp` orders equal to `key`: `cmp(key, element)` is
/// negative, zero, or positive as `key` orders before, equal to, or after the
/// element. Returns a pointer to one such element, or null.
///
/// # Safety
///
/// `base` holds `nel` elements of `width` bytes, sorted by `cmp`, and `cmp`
/// reads only its arguments. `cmp` is non-null.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn bsearch(
    key: *const c_void,
    base: *const c_void,
    nel: usize,
    width: usize,
    cmp: Option<CompareFn>,
) -> *mut c_void {
    #[cfg(feature = "forkpoint")]
    forkpoint::assert_always_or_unreachable!(
        cmp.is_some() && (nel == 0 || !base.is_null()),
        "bsearch: cmp is non-null, and base when nel is not 0"
    );
    let Some(cmp) = cmp else { return core::ptr::null_mut() };
    let base = base.cast::<u8>();
    let (mut low, mut high) = (0, nel);
    while low < high {
        let mid = low.midpoint(high);
        // `mid < nel`, so the element lies within the array, per the caller.
        let element = base.wrapping_add(mid.wrapping_mul(width));
        // SAFETY: `element` is an element of the array, which `cmp` may read,
        // per the caller.
        let order = unsafe { cmp(key, element.cast()) };
        if order == 0 {
            return element.cast_mut().cast();
        }
        if order < 0 {
            high = mid;
        } else {
            // `mid < high <= nel`, so this cannot wrap.
            low = mid.wrapping_add(1);
        }
    }
    #[cfg(feature = "forkpoint-coverage")]
    forkpoint::assert_sometimes!(nel > 0, "bsearch: finds no match in a non-empty array");
    core::ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use core::ffi::c_int;

    use super::*;

    unsafe extern "C" fn compare_i32(a: *const c_void, b: *const c_void) -> c_int {
        // SAFETY: both point at `i32`s of the arrays the tests pass.
        let (a, b) = unsafe { (a.cast::<i32>().read(), b.cast::<i32>().read()) };
        a.cmp(&b) as c_int
    }

    /// The index `bsearch` finds `key` at in `sorted`, if any.
    fn find(sorted: &[i32], key: i32) -> Option<usize> {
        let found = unsafe {
            // SAFETY: `sorted` is sorted, and `compare_i32` reads its `i32`s.
            bsearch(
                (&raw const key).cast(),
                sorted.as_ptr().cast(),
                sorted.len(),
                size_of::<i32>(),
                Some(compare_i32),
            )
        };
        (!found.is_null()).then(|| {
            found
                .addr()
                .wrapping_sub(sorted.as_ptr().addr())
                .checked_div(size_of::<i32>())
                .unwrap_or(0)
        })
    }

    #[test]
    fn bsearch_finds_every_element_and_misses_the_rest() {
        let sorted = [-7, -2, 0, 3, 3, 8, 12];
        for (i, &key) in sorted.iter().enumerate() {
            let found = find(&sorted, key).expect("found");
            assert_eq!(sorted[found], key, "key {key} at {i}");
        }
        for key in [-8, -1, 1, 5, 13] {
            assert_eq!(find(&sorted, key), None);
        }
        assert_eq!(find(&[], 0), None);
        assert_eq!(find(&[4], 4), Some(0));
        assert_eq!(find(&[4], 5), None);
    }

    #[test]
    fn bsearch_returns_null_without_a_comparison() {
        let sorted = [1, 2];
        // SAFETY: a null `cmp` is tolerated.
        let found = unsafe { bsearch(sorted.as_ptr().cast(), sorted.as_ptr().cast(), 2, 4, None) };
        assert!(found.is_null());
    }
}
