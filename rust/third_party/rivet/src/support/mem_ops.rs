// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Access patterns shared by `memcpy`, `memmove`, `memset`, and `memcmp`.
//!
//! The block, overlap, and loop patterns, and dispatching on size smallest
//! first, follow "automemcpy: A Framework for Automatic Generation of
//! Fundamental Memory Operations" (ISMM '21):
//! <https://storage.googleapis.com/gweb-research2023-media/pubtools/6156.pdf>.
//!
//! Most calls to these functions are short, so each one dispatches on `n`,
//! smallest sizes first, to the cheapest pattern for this target:
//!
//! - Where an unaligned word access is one instruction ([`FAST_UNALIGNED`]),
//!   a size from `N` to `2N` is two `N`-byte blocks, one at each end of the
//!   range, that meet or overlap in the middle: no loop and no per-size
//!   branch. Above `2 * EDGE` bytes, an `EDGE`-byte block covers each end and
//!   words fill the middle, stored to aligned addresses.
//! - Elsewhere an unaligned word access is split into bytes, so blocks gain
//!   nothing and cost code size. Bytes are copied up to the first aligned
//!   address, whole aligned words follow when both pointers can be aligned
//!   together, and bytes finish the rest.
//!
//! Bytes in the overlapping part of two blocks are accessed twice, so these
//! patterns are not for memory with `volatile` semantics.
//!
//! [`copy_forward`] and [`copy_backward`] stay correct for overlapping ranges
//! that `memmove` passes them: every block is loaded before it is stored, and
//! each loop reads ahead of its writes in the direction it walks.
//!
//! Every load and store has a size fixed at compile time and lowers to plain
//! load and store instructions, never to a call to the functions defined here.
//!
//! Without the `automemcpy` feature every function is a plain byte loop: the
//! smallest code, at the cost of speed.

use core::ffi::c_int;
use core::mem::size_of;

/// Whether an unaligned word access is one instruction on this target: Arm
/// from ARMv7, which is told apart from ARMv6-M by its compare-and-swap, and
/// the 64-bit hosts. Only speed and code size depend on it; every pattern is
/// correct everywhere. The coverage macros at the end of this file repeat the
/// condition as a `#[cfg]`.
const FAST_UNALIGNED: bool = cfg!(any(
    all(target_arch = "arm", target_has_atomic = "32"),
    target_arch = "aarch64",
    target_arch = "x86_64",
));

/// Whether the size-dispatched patterns are built in; otherwise every function
/// moves one byte at a time. Both paths are compiled and linted either way.
const AUTOMEMCPY: bool = cfg!(feature = "automemcpy");

/// The machine word that the loops move.
type Word = usize;

/// Bytes in a [`Word`].
pub(crate) const WORD: usize = size_of::<Word>();

/// Bytes in the block at each end of a large fast-unaligned operation.
const EDGE: usize = 16;

/// Bytes below which a byte loop beats aligning to words, without
/// [`FAST_UNALIGNED`].
pub(crate) const WORDS_FROM: usize = 4 * WORD;

/// Whether `a` and `b` are equally misaligned, so aligning one aligns both.
#[inline(always)]
pub(crate) fn co_aligned(a: *const u8, b: *const u8) -> bool {
    (a.addr() ^ b.addr()) & (WORD - 1) == 0
}

/// Bytes from `p` to the next word boundary, below [`WORD`].
#[inline(always)]
fn to_aligned(p: *const u8) -> usize {
    p.addr().wrapping_neg() & (WORD - 1)
}

/// Loads a `T` from `p`, which need not be aligned.
///
/// # Safety
///
/// `p` is readable for `size_of::<T>()` bytes.
#[inline(always)]
unsafe fn load<T: Copy>(p: *const u8) -> T {
    // SAFETY: the caller guarantees `p` is readable for a `T`.
    unsafe { p.cast::<T>().read_unaligned() }
}

/// Stores `v` at `p`, which need not be aligned.
///
/// # Safety
///
/// `p` is writable for `size_of::<T>()` bytes.
#[inline(always)]
unsafe fn store<T: Copy>(p: *mut u8, v: T) {
    // SAFETY: the caller guarantees `p` is writable for a `T`.
    unsafe { p.cast::<T>().write_unaligned(v) }
}

/// Loads a [`Word`] from `p`, aligned when `ALIGNED`.
///
/// # Safety
///
/// `p` is readable for a [`Word`], and word-aligned when `ALIGNED`.
#[inline(always)]
#[expect(clippy::cast_ptr_alignment, reason = "`p` is word-aligned when `ALIGNED`, per the caller")]
unsafe fn load_word<const ALIGNED: bool>(p: *const u8) -> Word {
    if ALIGNED {
        // SAFETY: the caller guarantees `p` is aligned and readable.
        unsafe { p.cast::<Word>().read() }
    } else {
        // SAFETY: the caller guarantees `p` is readable.
        unsafe { load(p) }
    }
}

/// Stores `w` at `p`, which is word-aligned.
///
/// # Safety
///
/// `p` is word-aligned and writable for a [`Word`].
#[inline(always)]
#[expect(clippy::cast_ptr_alignment, reason = "`p` is word-aligned, per the caller")]
unsafe fn store_word(p: *mut u8, w: Word) {
    // SAFETY: the caller guarantees `p` is aligned and writable.
    unsafe { p.cast::<Word>().write(w) }
}

/// The [`EDGE`] bytes at `p`.
///
/// # Safety
///
/// `p` is readable for [`EDGE`] bytes.
#[inline(always)]
unsafe fn load_edge(p: *const u8) -> [u64; 2] {
    // SAFETY: the caller guarantees `EDGE` readable bytes.
    unsafe { [load(p), load(p.add(8))] }
}

/// Stores `v` as the [`EDGE`] bytes at `p`.
///
/// # Safety
///
/// `p` is writable for [`EDGE`] bytes.
#[inline(always)]
unsafe fn store_edge(p: *mut u8, v: [u64; 2]) {
    // SAFETY: the caller guarantees `EDGE` writable bytes.
    unsafe {
        store(p, v[0]);
        store(p.add(8), v[1]);
    }
}

/// Copies `n` bytes, `size_of::<T>() <= n <= 2 * size_of::<T>()`, as one `T`
/// at each end. Both are loaded before either is stored.
///
/// # Safety
///
/// `src` is readable and `dst` writable for `n` bytes, and `n` is in range.
#[inline(always)]
unsafe fn copy_overlap<T: Copy>(dst: *mut u8, src: *const u8, n: usize) {
    let last = n - size_of::<T>();
    // SAFETY: both blocks lie within the `n` bytes the caller guarantees.
    unsafe {
        let head: T = load(src);
        let tail: T = load(src.add(last));
        store(dst, head);
        store(dst.add(last), tail);
    }
}

/// Copies `n <= 2 * EDGE` bytes with blocks, loading every byte before storing
/// any.
///
/// # Safety
///
/// `src` is readable and `dst` writable for `n` bytes, and `n <= 2 * EDGE`.
#[inline(always)]
unsafe fn copy_blocks(dst: *mut u8, src: *const u8, n: usize) {
    // SAFETY: each arm covers exactly the `n` bytes the caller guarantees.
    unsafe {
        match n {
            0 => {}
            1 => store(dst, load::<u8>(src)),
            2..4 => copy_overlap::<u16>(dst, src, n),
            4..8 => copy_overlap::<u32>(dst, src, n),
            8..=16 => copy_overlap::<u64>(dst, src, n),
            _ => {
                let head = load_edge(src);
                let tail = load_edge(src.add(n - EDGE));
                store_edge(dst, head);
                store_edge(dst.add(n - EDGE), tail);
            }
        }
    }
}

/// Copies `n` bytes from `src` to `dst`, walking up.
///
/// # Safety
///
/// `src` is readable and `dst` writable for `n` bytes, and `dst` is not above
/// `src` if the ranges overlap.
#[inline(always)]
pub(crate) unsafe fn copy_forward(dst: *mut u8, src: *const u8, n: usize) {
    // SAFETY: forwarded from the caller.
    unsafe {
        if !AUTOMEMCPY {
            copy_bytes_up(dst, src, 0, n);
        } else if FAST_UNALIGNED {
            if n <= 2 * EDGE {
                copy_blocks(dst, src, n);
            } else {
                // The edges are loaded first and stored last.
                let (head, tail) = (load_edge(src), load_edge(src.add(n - EDGE)));
                let end = n - EDGE;
                // Start where `dst` is aligned, below `EDGE`, and stop once the
                // tail edge covers the rest.
                let start = to_aligned(dst);
                copy_words_up::<false>(dst, src, start, end);
                store_edge(dst, head);
                store_edge(dst.add(end), tail);
            }
        } else if n >= WORDS_FROM && co_aligned(dst, src) {
            let start = to_aligned(dst);
            let end = start + (n - start) / WORD * WORD;
            copy_bytes_up(dst, src, 0, start);
            copy_words_up::<true>(dst, src, start, end);
            copy_bytes_up(dst, src, end, n);
        } else {
            copy_bytes_up(dst, src, 0, n);
        }
    }
}

/// Copies `n` bytes from `src` to `dst`, walking down.
///
/// # Safety
///
/// `src` is readable and `dst` writable for `n` bytes, and `dst` is not below
/// `src` if the ranges overlap.
#[inline(always)]
pub(crate) unsafe fn copy_backward(dst: *mut u8, src: *const u8, n: usize) {
    // SAFETY: forwarded from the caller.
    unsafe {
        if !AUTOMEMCPY {
            copy_bytes_down(dst, src, 0, n);
        } else if FAST_UNALIGNED {
            if n <= 2 * EDGE {
                copy_blocks(dst, src, n);
            } else {
                let (head, tail) = (load_edge(src), load_edge(src.add(n - EDGE)));
                // Stop where a word ends at an aligned `dst` address; the tail
                // edge covers what lies above, and the head edge what lies
                // below `EDGE`.
                let end = n - dst.addr().wrapping_add(n) % WORD;
                copy_words_down::<false>(dst, src, EDGE, end);
                store_edge(dst, head);
                store_edge(dst.add(n - EDGE), tail);
            }
        } else if n >= WORDS_FROM && co_aligned(dst, src) {
            let start = to_aligned(dst);
            let end = start + (n - start) / WORD * WORD;
            copy_bytes_down(dst, src, end, n);
            copy_words_down::<true>(dst, src, start, end);
            copy_bytes_down(dst, src, 0, start);
        } else {
            copy_bytes_down(dst, src, 0, n);
        }
    }
}

/// Copies the bytes at offsets `[from, to)`, walking up.
///
/// # Safety
///
/// `src` is readable and `dst` writable over the offsets, and any overlap is
/// as for [`copy_forward`].
#[inline(always)]
unsafe fn copy_bytes_up(dst: *mut u8, src: *const u8, from: usize, to: usize) {
    for i in from..to {
        // SAFETY: forwarded from the caller.
        unsafe { *dst.add(i) = *src.add(i) };
    }
}

/// Copies the bytes at offsets `[from, to)`, walking down.
///
/// # Safety
///
/// As for [`copy_bytes_up`], with overlap as for [`copy_backward`].
#[inline(always)]
unsafe fn copy_bytes_down(dst: *mut u8, src: *const u8, from: usize, to: usize) {
    for i in (from..to).rev() {
        // SAFETY: forwarded from the caller.
        unsafe { *dst.add(i) = *src.add(i) };
    }
}

/// Copies words at offsets from `start` while they start below `end`, walking
/// up. The last word may end past `end` by less than a word.
///
/// # Safety
///
/// `dst + start` is word-aligned, and `src + start` too when `SRC_ALIGNED`.
/// Every word copied lies within both ranges, and any overlap is as for
/// [`copy_forward`].
#[inline(always)]
unsafe fn copy_words_up<const SRC_ALIGNED: bool>(
    dst: *mut u8,
    src: *const u8,
    start: usize,
    end: usize,
) {
    let mut off = start;
    // SAFETY: forwarded from the caller. Each group of words is loaded before
    // any is stored.
    unsafe {
        while off + 4 * WORD <= end {
            let w = [
                load_word::<SRC_ALIGNED>(src.add(off)),
                load_word::<SRC_ALIGNED>(src.add(off + WORD)),
                load_word::<SRC_ALIGNED>(src.add(off + 2 * WORD)),
                load_word::<SRC_ALIGNED>(src.add(off + 3 * WORD)),
            ];
            for (i, w) in w.into_iter().enumerate() {
                store_word(dst.add(off + i * WORD), w);
            }
            off += 4 * WORD;
        }
        while off < end {
            store_word(dst.add(off), load_word::<SRC_ALIGNED>(src.add(off)));
            off += WORD;
        }
    }
}

/// Copies words ending at offsets from `end` while they end above `start`,
/// walking down. The last word may start before `start` by less than a word.
///
/// # Safety
///
/// `dst + end` is word-aligned, and `src + end` too when `SRC_ALIGNED`. Every
/// word copied lies within both ranges, and any overlap is as for
/// [`copy_backward`].
#[inline(always)]
unsafe fn copy_words_down<const SRC_ALIGNED: bool>(
    dst: *mut u8,
    src: *const u8,
    start: usize,
    end: usize,
) {
    let mut off = end;
    // SAFETY: as for `copy_words_up`.
    unsafe {
        while off >= start + 4 * WORD {
            off -= 4 * WORD;
            let w = [
                load_word::<SRC_ALIGNED>(src.add(off + 3 * WORD)),
                load_word::<SRC_ALIGNED>(src.add(off + 2 * WORD)),
                load_word::<SRC_ALIGNED>(src.add(off + WORD)),
                load_word::<SRC_ALIGNED>(src.add(off)),
            ];
            for (i, w) in w.into_iter().enumerate() {
                store_word(dst.add(off + (3 - i) * WORD), w);
            }
        }
        while off > start {
            off -= WORD;
            store_word(dst.add(off), load_word::<SRC_ALIGNED>(src.add(off)));
        }
    }
}

/// Fills `n` bytes, `size_of::<T>() <= n <= 2 * size_of::<T>()`, with `v`, as
/// one `T` at each end.
///
/// # Safety
///
/// `dst` is writable for `n` bytes, and `n` is in range.
#[inline(always)]
unsafe fn fill_overlap<T: Copy>(dst: *mut u8, v: T, n: usize) {
    // SAFETY: both blocks lie within the `n` bytes the caller guarantees.
    unsafe {
        store(dst, v);
        store(dst.add(n - size_of::<T>()), v);
    }
}

/// Fills `n` bytes at `dst` with `c`.
///
/// # Safety
///
/// `dst` is writable for `n` bytes.
#[inline(always)]
pub(crate) unsafe fn fill(dst: *mut u8, c: u8, n: usize) {
    // `c` in every byte; truncating it keeps that.
    let v = u64::from(c) * (u64::MAX / 0xff);
    // SAFETY: every store lies within the `n` bytes the caller guarantees.
    unsafe {
        if !AUTOMEMCPY {
            fill_bytes(dst, c, 0, n);
        } else if FAST_UNALIGNED {
            match n {
                0 => {}
                1 => store(dst, c),
                2..4 => fill_overlap(dst, v as u16, n),
                4..8 => fill_overlap(dst, v as u32, n),
                8..=16 => fill_overlap(dst, v, n),
                _ => {
                    store_edge(dst, [v, v]);
                    store_edge(dst.add(n - EDGE), [v, v]);
                    if n > 2 * EDGE {
                        fill_words(dst, v as Word, to_aligned(dst), n - EDGE);
                    }
                }
            }
        } else if n >= WORDS_FROM {
            let start = to_aligned(dst);
            let end = start + (n - start) / WORD * WORD;
            fill_bytes(dst, c, 0, start);
            fill_words(dst, v as Word, start, end);
            fill_bytes(dst, c, end, n);
        } else {
            fill_bytes(dst, c, 0, n);
        }
    }
}

/// Fills the bytes at offsets `[from, to)` with `c`.
///
/// # Safety
///
/// `dst` is writable over the offsets.
#[inline(always)]
unsafe fn fill_bytes(dst: *mut u8, c: u8, from: usize, to: usize) {
    for i in from..to {
        // SAFETY: forwarded from the caller.
        unsafe { *dst.add(i) = c };
    }
}

/// Fills words with `w` at offsets from `start` while they start below `end`.
/// The last word may end past `end` by less than a word.
///
/// # Safety
///
/// `dst + start` is word-aligned, and every word written lies within `dst`'s
/// range.
#[inline(always)]
unsafe fn fill_words(dst: *mut u8, w: Word, start: usize, end: usize) {
    let mut off = start;
    // SAFETY: forwarded from the caller.
    unsafe {
        while off + 4 * WORD <= end {
            for i in 0..4 {
                store_word(dst.add(off + i * WORD), w);
            }
            off += 4 * WORD;
        }
        while off < end {
            store_word(dst.add(off), w);
            off += WORD;
        }
    }
}

/// Compares `n` bytes as `unsigned char`: the difference of the first pair
/// that differs, or 0.
///
/// # Safety
///
/// `a` and `b` are readable for `n` bytes.
#[inline(always)]
pub(crate) unsafe fn compare(a: *const u8, b: *const u8, n: usize) -> c_int {
    // SAFETY: every access lies in `[0, n)`.
    unsafe {
        // Below a word, setting up the word loop costs more than it saves.
        if !AUTOMEMCPY || n < WORD {
            return compare_bytes(a, b, n);
        }
        let off = if FAST_UNALIGNED {
            compare_words::<false>(a, b, n)
        } else if co_aligned(a, b) {
            // Compare bytes up to the first word boundary, then aligned words.
            let head = to_aligned(a);
            let d = compare_bytes(a, b, head);
            if d != 0 {
                return d;
            }
            head + compare_words::<true>(a.add(head), b.add(head), n - head)
        } else {
            0
        };
        // The rest is fewer than `WORD` bytes, or starts with the word that
        // differs.
        compare_bytes(a.add(off), b.add(off), n - off)
    }
}

/// The number of leading bytes of `n` that whole equal words cover. It stops
/// at the first word that differs, or before the fewer than `WORD` bytes left.
///
/// # Safety
///
/// `a` and `b` are readable for `n` bytes, and word-aligned when `ALIGNED`.
#[inline(always)]
unsafe fn compare_words<const ALIGNED: bool>(a: *const u8, b: *const u8, n: usize) -> usize {
    let mut off = 0;
    // SAFETY: every word read lies in `[0, n)`.
    unsafe {
        while off + WORD <= n
            && load_word::<ALIGNED>(a.add(off)) == load_word::<ALIGNED>(b.add(off))
        {
            off += WORD;
        }
    }
    off
}

/// Compares `n` bytes one at a time.
///
/// # Safety
///
/// `a` and `b` are readable for `n` bytes.
#[inline(always)]
unsafe fn compare_bytes(a: *const u8, b: *const u8, n: usize) -> c_int {
    for i in 0..n {
        // SAFETY: `i < n`.
        let (p, q) = unsafe { (*a.add(i), *b.add(i)) };
        if p != q {
            return c_int::from(p) - c_int::from(q);
        }
    }
    0
}

// Coverage properties: with `forkpoint-coverage`, each function marks the
// pattern a call takes, so a test run shows that every pattern ran. A property
// is cataloged even where it cannot be reached, so each target marks only the
// patterns it builds in: the `#[cfg]`s repeat `FAST_UNALIGNED` and
// `AUTOMEMCPY`, and the conditions repeat the dispatch above.

/// Marks the pattern that copying `n` bytes from `src` to `dst` takes, in
/// messages that start with `$f` and end with `$dir`.
macro_rules! cover_copy {
    ($f:literal, $dir:literal, $dst:expr, $src:expr, $n:expr) => {{
        let n: usize = $n;
        #[cfg(all(
            feature = "forkpoint-coverage",
            feature = "automemcpy",
            any(
                all(target_arch = "arm", target_has_atomic = "32"),
                target_arch = "aarch64",
                target_arch = "x86_64",
            ),
        ))]
        {
            forkpoint::assert_sometimes!(n == 1, concat!($f, ": copies 1 byte", $dir));
            forkpoint::assert_sometimes!(
                (2..4).contains(&n),
                concat!($f, ": copies 2 to 3 bytes as two 2-byte blocks", $dir)
            );
            forkpoint::assert_sometimes!(
                (4..8).contains(&n),
                concat!($f, ": copies 4 to 7 bytes as two 4-byte blocks", $dir)
            );
            forkpoint::assert_sometimes!(
                (8..=16).contains(&n),
                concat!($f, ": copies 8 to 16 bytes as two 8-byte blocks", $dir)
            );
            forkpoint::assert_sometimes!(
                (17..=32).contains(&n),
                concat!($f, ": copies 17 to 32 bytes as two 16-byte blocks", $dir)
            );
            forkpoint::assert_sometimes!(
                n > 32,
                concat!($f, ": copies over 32 bytes as words between two 16-byte blocks", $dir)
            );
        }
        #[cfg(all(
            feature = "forkpoint-coverage",
            feature = "automemcpy",
            not(any(
                all(target_arch = "arm", target_has_atomic = "32"),
                target_arch = "aarch64",
                target_arch = "x86_64",
            )),
        ))]
        {
            use $crate::support::mem_ops::{WORDS_FROM, co_aligned};
            let long = n >= WORDS_FROM;
            forkpoint::assert_sometimes!(
                n > 0 && !long,
                concat!($f, ": copies a short range one byte at a time", $dir)
            );
            forkpoint::assert_sometimes!(
                long && co_aligned($dst, $src),
                concat!($f, ": copies aligned words between equally aligned pointers", $dir)
            );
            forkpoint::assert_sometimes!(
                long && !co_aligned($dst, $src),
                concat!($f, ": copies bytes between differently aligned pointers", $dir)
            );
        }
        let _ = ($dst, $src, n);
    }};
}
pub(crate) use cover_copy;

/// Marks the pattern that filling `n` bytes at `dst` takes, in messages that
/// start with `$f`.
macro_rules! cover_fill {
    ($f:literal, $n:expr) => {{
        let n: usize = $n;
        #[cfg(all(
            feature = "forkpoint-coverage",
            feature = "automemcpy",
            any(
                all(target_arch = "arm", target_has_atomic = "32"),
                target_arch = "aarch64",
                target_arch = "x86_64",
            ),
        ))]
        {
            forkpoint::assert_sometimes!(n == 1, concat!($f, ": fills 1 byte"));
            forkpoint::assert_sometimes!(
                (2..4).contains(&n),
                concat!($f, ": fills 2 to 3 bytes as two 2-byte blocks")
            );
            forkpoint::assert_sometimes!(
                (4..8).contains(&n),
                concat!($f, ": fills 4 to 7 bytes as two 4-byte blocks")
            );
            forkpoint::assert_sometimes!(
                (8..=16).contains(&n),
                concat!($f, ": fills 8 to 16 bytes as two 8-byte blocks")
            );
            forkpoint::assert_sometimes!(
                (17..=32).contains(&n),
                concat!($f, ": fills 17 to 32 bytes as two 16-byte blocks")
            );
            forkpoint::assert_sometimes!(
                n > 32,
                concat!($f, ": fills over 32 bytes as words between two 16-byte blocks")
            );
        }
        #[cfg(all(
            feature = "forkpoint-coverage",
            feature = "automemcpy",
            not(any(
                all(target_arch = "arm", target_has_atomic = "32"),
                target_arch = "aarch64",
                target_arch = "x86_64",
            )),
        ))]
        {
            use $crate::support::mem_ops::WORDS_FROM;
            forkpoint::assert_sometimes!(
                n > 0 && n < WORDS_FROM,
                concat!($f, ": fills a short range one byte at a time")
            );
            forkpoint::assert_sometimes!(
                n >= WORDS_FROM,
                concat!($f, ": fills aligned words between unaligned ends")
            );
        }
        let _ = n;
    }};
}
pub(crate) use cover_fill;

/// Marks the pattern that comparing `n` bytes at `a` and `b` takes, in
/// messages that start with `$f`.
macro_rules! cover_compare {
    ($f:literal, $a:expr, $b:expr, $n:expr) => {{
        let n: usize = $n;
        #[cfg(all(feature = "forkpoint-coverage", feature = "automemcpy"))]
        {
            use $crate::support::mem_ops::WORD;
            forkpoint::assert_sometimes!(
                n > 0 && n < WORD,
                concat!($f, ": compares fewer bytes than a word one at a time")
            );
            #[cfg(any(
                all(target_arch = "arm", target_has_atomic = "32"),
                target_arch = "aarch64",
                target_arch = "x86_64",
            ))]
            forkpoint::assert_sometimes!(n >= WORD, concat!($f, ": compares words"));
            #[cfg(not(any(
                all(target_arch = "arm", target_has_atomic = "32"),
                target_arch = "aarch64",
                target_arch = "x86_64",
            )))]
            {
                use $crate::support::mem_ops::co_aligned;
                forkpoint::assert_sometimes!(
                    n >= WORD && co_aligned($a, $b),
                    concat!($f, ": compares aligned words between equally aligned pointers")
                );
                forkpoint::assert_sometimes!(
                    n >= WORD && !co_aligned($a, $b),
                    concat!($f, ": compares differently aligned bytes one at a time")
                );
            }
        }
        let _ = ($a, $b, n);
    }};
}
pub(crate) use cover_compare;
