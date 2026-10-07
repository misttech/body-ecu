// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.
//
// Ported from upstream, under this notice:
//
// Copyright (C) 2011 by Valentin Ochs
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to
// deal in the Software without restriction, including without limitation the
// rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
// sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.

//! Smoothsort, an adaptive variant of heapsort, for `qsort`. Memory usage:
//! O(1). Run time: worst case O(n log n), close to O(n) in the mostly-sorted
//! case.
//!
//! Ported from musl 1.2.5's `src/stdlib/qsort.c` (https://musl.libc.org/),
//! with these changes:
//!
//! - The comparison takes no context argument: upstream is the reentrant
//!   variant that `qsort_r` and `qsort` share, and rivet has only `qsort`.
//! - [`sort`] takes an element count and width whose product `qsort` has
//!   checked, so the size arithmetic here cannot overflow.
//! - `ntz` of 0 is 0, as upstream's generic bit-scan gives, on every target.
//! - The two-word bit vector `p` shifts through `checked_shr`, where upstream
//!   shifts by the word width on a boundary, which C leaves undefined.
//! - Pointers into the array move through `wrapping_add` and `wrapping_sub`,
//!   so only the comparisons and the moves are `unsafe`.

use core::ffi::{c_int, c_void};
use core::mem::size_of;
use core::ptr;

/// `qsort`'s comparison: negative, zero, or positive as the first element
/// orders before, equal to, or after the second.
pub(crate) type CompareFn = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;

/// Bits in a word of the bit vector `p`.
const BITS: u32 = usize::BITS;

/// The most elements `cycle` rotates, one per level of a tree: upstream's
/// `14 * sizeof(size_t) + 1`.
const CYCLE_MAX: usize = 14 * size_of::<usize>() + 1;

/// The most Leonardo numbers below any array size: upstream's `12 * sizeof(size_t)`.
const LEONARDO_MAX: usize = 12 * size_of::<usize>();

/// Bytes `cycle` moves at a time.
const CYCLE_CHUNK: usize = 256;

/// Trailing zeros of `x`, with 0 for 0, as upstream's generic bit scan gives.
fn ntz(x: usize) -> u32 {
    if x == 0 { 0 } else { x.trailing_zeros() }
}

/// Trailing zeros of the bit vector `p` with its lowest bit cleared.
fn pntz(p: [usize; 2]) -> u32 {
    let r = ntz(p[0].wrapping_sub(1));
    if r != 0 {
        return r;
    }
    let r = BITS + ntz(p[1]);
    if r != BITS {
        return r;
    }
    0
}

/// Rotates the `n` elements at `ar[..n]` one place down: the element at
/// `ar[0]` ends at `ar[n - 1]`, through a chunk of stack.
///
/// # Safety
///
/// `ar[..n]` point at distinct elements of `width` writable bytes.
unsafe fn cycle(width: usize, ar: &mut [*mut u8; CYCLE_MAX], n: usize) {
    let mut tmp = [0u8; CYCLE_CHUNK];
    if n < 2 {
        return;
    }
    ar[n] = tmp.as_mut_ptr();
    let mut width = width;
    while width != 0 {
        let l = CYCLE_CHUNK.min(width);
        // SAFETY: `ar[0]` has `l` of its `width` bytes left to move, and `tmp`
        // holds `l`.
        unsafe { ptr::copy_nonoverlapping(ar[0], ar[n], l) };
        for i in 0..n {
            // SAFETY: `ar[i]` and `ar[i + 1]` are distinct elements, or `tmp`,
            // each with `l` bytes left to move.
            unsafe { ptr::copy_nonoverlapping(ar[i + 1], ar[i], l) };
            ar[i] = ar[i].wrapping_add(l);
        }
        width -= l;
    }
}

/// Shifts the bit vector `p` left by `n > 0`.
fn shl(p: &mut [usize; 2], mut n: u32) {
    if n >= BITS {
        n -= BITS;
        p[1] = p[0];
        p[0] = 0;
    }
    p[1] <<= n;
    p[1] |= p[0].checked_shr(BITS - n).unwrap_or(0);
    p[0] <<= n;
}

/// Shifts the bit vector `p` right by `n > 0`.
fn shr(p: &mut [usize; 2], mut n: u32) {
    if n >= BITS {
        n -= BITS;
        p[0] = p[1];
        p[1] = 0;
    }
    p[0] >>= n;
    p[0] |= p[1].checked_shl(BITS - n).unwrap_or(0);
    p[1] >>= n;
}

/// Sifts the root at `head` down its tree of order `pshift`.
///
/// # Safety
///
/// `head` is the root of a tree of order `pshift` within the array, whose
/// elements `cmp` may read.
unsafe fn sift(
    mut head: *mut u8,
    width: usize,
    cmp: CompareFn,
    mut pshift: usize,
    lp: &[usize; LEONARDO_MAX],
) {
    let mut ar = [ptr::null_mut(); CYCLE_MAX];
    let mut i = 1;

    ar[0] = head;
    while pshift > 1 {
        let rt = head.wrapping_sub(width);
        let lf = head.wrapping_sub(width).wrapping_sub(lp[pshift - 2]);

        // SAFETY: `ar[0]`, `lf`, and `rt` are elements of the array.
        if unsafe { cmp(ar[0].cast(), lf.cast()) >= 0 && cmp(ar[0].cast(), rt.cast()) >= 0 } {
            break;
        }
        // SAFETY: as above.
        if unsafe { cmp(lf.cast(), rt.cast()) } >= 0 {
            ar[i] = lf;
            i += 1;
            head = lf;
            pshift -= 1;
        } else {
            ar[i] = rt;
            i += 1;
            head = rt;
            pshift -= 2;
        }
    }
    // SAFETY: `ar[..i]` are the distinct elements walked down the tree.
    unsafe { cycle(width, &mut ar, i) };
}

/// Restores the heap order along the roots ending at `head`, then sifts.
///
/// # Safety
///
/// `head` is the root of a tree of order `pshift` within the array, `pp` is
/// the heap's bit vector, and `cmp` may read the array's elements.
unsafe fn trinkle(
    mut head: *mut u8,
    width: usize,
    cmp: CompareFn,
    pp: [usize; 2],
    mut pshift: usize,
    mut trusty: bool,
    lp: &[usize; LEONARDO_MAX],
) {
    let mut p = pp;
    let mut ar = [ptr::null_mut(); CYCLE_MAX];
    let mut i = 1;

    ar[0] = head;
    while p[0] != 1 || p[1] != 0 {
        let stepson = head.wrapping_sub(lp[pshift]);
        // SAFETY: `stepson` and `ar[0]` are elements of the array.
        if unsafe { cmp(stepson.cast(), ar[0].cast()) } <= 0 {
            break;
        }
        if !trusty && pshift > 1 {
            let rt = head.wrapping_sub(width);
            let lf = head.wrapping_sub(width).wrapping_sub(lp[pshift - 2]);
            // SAFETY: as above.
            if unsafe { cmp(rt.cast(), stepson.cast()) >= 0 || cmp(lf.cast(), stepson.cast()) >= 0 }
            {
                break;
            }
        }

        ar[i] = stepson;
        i += 1;
        head = stepson;
        let trail = pntz(p);
        shr(&mut p, trail);
        pshift += trail as usize;
        trusty = false;
    }
    if !trusty {
        // SAFETY: `ar[..i]` are the distinct roots walked, and `head` is the
        // root of a tree of order `pshift`.
        unsafe {
            cycle(width, &mut ar, i);
            sift(head, width, cmp, pshift, lp);
        }
    }
}

/// Sorts the `nel` elements of `width` bytes at `base` by `cmp`.
///
/// # Safety
///
/// `base` holds `nel` writable elements of `width` bytes, `nel * width` does
/// not overflow and is not 0, and `cmp` reads only its arguments.
pub(crate) unsafe fn sort(base: *mut u8, nel: usize, width: usize, cmp: CompareFn) {
    let mut lp = [0usize; LEONARDO_MAX];
    let size = width * nel;
    let mut p = [1usize, 0];
    let mut pshift = 1usize;

    let mut head = base;
    let high = head.wrapping_add(size - width);

    // Precompute Leonardo numbers, scaled by element width. An array size
    // near `usize::MAX` saturates the sum, which also ends the loop.
    lp[0] = width;
    lp[1] = width;
    let mut i = 2;
    loop {
        lp[i] = lp[i - 2].saturating_add(lp[i - 1]).saturating_add(width);
        if lp[i] >= size {
            break;
        }
        i += 1;
    }

    while head < high {
        if p[0] & 3 == 3 {
            // SAFETY: `head` is the root of a tree of order `pshift`, by the
            // invariant the bit vector keeps.
            unsafe { sift(head, width, cmp, pshift, &lp) };
            shr(&mut p, 2);
            pshift += 2;
        } else {
            // SAFETY: as above.
            unsafe {
                if lp[pshift - 1] >= high.addr() - head.addr() {
                    trinkle(head, width, cmp, p, pshift, false, &lp);
                } else {
                    sift(head, width, cmp, pshift, &lp);
                }
            }

            if pshift == 1 {
                shl(&mut p, 1);
                pshift = 0;
            } else {
                shl(&mut p, (pshift - 1) as u32);
                pshift = 1;
            }
        }

        p[0] |= 1;
        head = head.wrapping_add(width);
    }

    // SAFETY: `head` is the last element, the root of the last tree.
    unsafe { trinkle(head, width, cmp, p, pshift, false, &lp) };

    while pshift != 1 || p[0] != 1 || p[1] != 0 {
        if pshift <= 1 {
            let trail = pntz(p);
            shr(&mut p, trail);
            pshift += trail as usize;
        } else {
            shl(&mut p, 2);
            pshift -= 2;
            p[0] ^= 7;
            shr(&mut p, 1);
            // SAFETY: the two roots the tree at `head` splits into are within
            // the array, by the invariant the bit vector keeps.
            unsafe {
                trinkle(
                    head.wrapping_sub(lp[pshift]).wrapping_sub(width),
                    width,
                    cmp,
                    p,
                    pshift + 1,
                    true,
                    &lp,
                );
            }
            shl(&mut p, 1);
            p[0] |= 1;
            // SAFETY: as above.
            unsafe { trinkle(head.wrapping_sub(width), width, cmp, p, pshift, true, &lp) };
        }
        head = head.wrapping_sub(width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn compare_u32(a: *const c_void, b: *const c_void) -> c_int {
        // SAFETY: both point at `u32`s of the array under sort.
        let (a, b) = unsafe { (a.cast::<u32>().read(), b.cast::<u32>().read()) };
        a.cmp(&b) as c_int
    }

    unsafe extern "C" fn compare_byte(a: *const c_void, b: *const c_void) -> c_int {
        // SAFETY: both point at bytes of the array under sort.
        let (a, b) = unsafe { (a.cast::<u8>().read(), b.cast::<u8>().read()) };
        a.cmp(&b) as c_int
    }

    /// A wide element, to make `cycle` move it in more than one chunk; it
    /// orders by its first byte, and its tail must travel with it.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    #[repr(C)]
    struct Wide([u8; 2 * CYCLE_CHUNK + 3]);

    impl Wide {
        fn new(key: u8) -> Self {
            let mut bytes = [0; 2 * CYCLE_CHUNK + 3];
            for (i, byte) in bytes.iter_mut().enumerate() {
                *byte = key.wrapping_add(i as u8);
            }
            Self(bytes)
        }
    }

    unsafe extern "C" fn compare_wide(a: *const c_void, b: *const c_void) -> c_int {
        // SAFETY: both point at `Wide`s of the array under sort.
        let (a, b) = unsafe { (a.cast::<Wide>().read(), b.cast::<Wide>().read()) };
        a.0[0].cmp(&b.0[0]) as c_int
    }

    /// Sorts `values` through `sort` and checks against `sort_unstable`.
    fn check<T: Copy + Ord + core::fmt::Debug>(values: &mut [T], cmp: CompareFn) {
        let mut expected = [None; 1024];
        for (slot, &value) in expected.iter_mut().zip(values.iter()) {
            *slot = Some(value);
        }
        let expected = &mut expected[..values.len()];
        expected.sort_unstable();
        if !values.is_empty() {
            // SAFETY: `values` holds its elements, which `cmp` reads.
            unsafe { sort(values.as_mut_ptr().cast(), values.len(), size_of::<T>(), cmp) };
        }
        for (value, expected) in values.iter().zip(expected.iter()) {
            assert_eq!(Some(*value), *expected);
        }
    }

    /// A deterministic sequence of `u32`s.
    fn pseudo_random(n: usize) -> [u32; 1024] {
        let mut state = 0x2545_f491_u32;
        let mut values = [0; 1024];
        for value in values.iter_mut().take(n) {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *value = state;
        }
        values
    }

    #[test]
    fn sort_orders_every_small_length() {
        for n in 1..=64 {
            let mut values = pseudo_random(n);
            check(&mut values[..n], compare_u32);
            let mut small = pseudo_random(n).map(|v| v % 4);
            check(&mut small[..n], compare_u32);
        }
    }

    #[test]
    fn sort_orders_a_long_array_in_every_initial_order() {
        let mut values = pseudo_random(1024);
        check(&mut values, compare_u32);
        check(&mut values, compare_u32);
        values.reverse();
        check(&mut values, compare_u32);
        let mut equal = [7u32; 1024];
        check(&mut equal, compare_u32);
        let mut nearly = pseudo_random(1024);
        nearly.sort_unstable();
        nearly.swap(10, 900);
        nearly.swap(5, 6);
        check(&mut nearly, compare_u32);
    }

    #[test]
    fn sort_moves_elements_of_any_width() {
        let mut bytes = pseudo_random(300).map(|v| v as u8);
        check(&mut bytes[..300], compare_byte);
        let mut wide = [Wide::new(9), Wide::new(200), Wide::new(3), Wide::new(3), Wide::new(77)];
        // SAFETY: `wide` holds 5 `Wide`s, which `compare_wide` reads.
        unsafe { sort(wide.as_mut_ptr().cast(), 5, size_of::<Wide>(), compare_wide) };
        assert_eq!(wide, [Wide::new(3), Wide::new(3), Wide::new(9), Wide::new(77), Wide::new(200)]);
    }

    #[test]
    fn bit_vector_shifts_carry_between_words() {
        let mut p = [1usize << (BITS - 1), 0];
        shl(&mut p, 1);
        assert_eq!(p, [0, 1]);
        shr(&mut p, 1);
        assert_eq!(p, [1 << (BITS - 1), 0]);
        let mut p = [0b1011, 0];
        shl(&mut p, BITS);
        assert_eq!(p, [0, 0b1011]);
        shr(&mut p, BITS);
        assert_eq!(p, [0b1011, 0]);
        assert_eq!(pntz([0b1001, 0]), 3);
        assert_eq!(pntz([1, 0b100]), BITS + 2);
        assert_eq!(pntz([1, 0]), 0);
    }
}
