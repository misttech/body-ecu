// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Cursors over C strings, for the `string.h` functions and `atoi`.
//!
//! The C contract of those functions is a raw pointer to a NUL-terminated
//! string, or to one readable for `n` bytes. A [`CStrCursor`] takes that
//! contract once, in its constructor, and walks the string in safe code: it
//! moves only past a byte it has read and found to be no NUL, so it never
//! reads past the NUL or the limit, and the logic of the function has nothing
//! left to justify. A [`CStrPair`] does the same for two strings walked in
//! lockstep, as the comparisons do. Neither builds a slice or a `CStr`, which
//! would compare through `memcmp` or measure through `strlen`, the functions
//! being defined here.
//!
//! The shapes here are chosen for the code they compile to, which is the byte
//! loop each function was before: see the notes on `BOUNDED`, [`CStrPair`],
//! and [`copy`].

use core::ffi::{c_char, c_int};
use core::marker::PhantomData;
use core::num::NonZeroU8;

/// A position in a C string.
///
/// `BOUNDED` is whether the string may also end after `n` bytes, as the `n` of
/// `strncmp` and `strncpy` says. It is a type parameter rather than a field so
/// that a cursor over a string bounded only by its NUL compiles to the plain
/// byte loop: a limit of `usize::MAX` would give that loop a trip count the
/// compiler can compute, and it would unroll the loop many times over.
#[derive(Clone)]
pub(crate) struct CStrCursor<'a, const BOUNDED: bool = false> {
    start: *const u8,
    /// Bytes from `start` to the cursor.
    pos: usize,
    /// Bytes from `start` that may be read, when `BOUNDED`.
    limit: usize,
    _string: PhantomData<&'a [u8]>,
}

impl CStrCursor<'_> {
    /// A cursor at the start of `s`.
    ///
    /// # Safety
    ///
    /// `s` is a NUL-terminated string that lives for `'a`.
    #[inline(always)]
    pub(crate) unsafe fn new(s: *const c_char) -> Self {
        Self { start: s.cast(), pos: 0, limit: 0, _string: PhantomData }
    }
}

impl CStrCursor<'_, true> {
    /// A cursor at the start of `s` that ends after `n` bytes, or at the NUL if
    /// it comes first.
    ///
    /// # Safety
    ///
    /// `s` is NUL-terminated or readable for `n` bytes, and lives for `'a`.
    #[inline(always)]
    pub(crate) unsafe fn bounded(s: *const c_char, n: usize) -> Self {
        Self { start: s.cast(), pos: 0, limit: n, _string: PhantomData }
    }
}

impl<const BOUNDED: bool> CStrCursor<'_, BOUNDED> {
    /// The byte at the cursor: 0 at the end of the string or of its `n` bytes.
    #[inline(always)]
    pub(crate) fn peek(&self) -> u8 {
        if BOUNDED && self.pos >= self.limit {
            return 0;
        }
        // SAFETY: the cursor only moves past a byte that is not the NUL, so
        // `pos` is at or before the NUL of a NUL-terminated string, and here
        // below `limit` for one readable for `limit` bytes.
        unsafe { self.start.add(self.pos).read() }
    }

    /// Moves past the byte at the cursor, unless the cursor is at the end.
    #[inline(always)]
    pub(crate) fn advance(&mut self) {
        if self.peek() != 0 {
            self.pos += 1;
        }
    }

    /// Moves past the byte at the cursor and returns it when `accept` holds
    /// for it. The end never qualifies.
    #[inline(always)]
    pub(crate) fn next_if(&mut self, accept: impl FnOnce(u8) -> bool) -> Option<u8> {
        let byte = self.peek();
        if byte == 0 || !accept(byte) {
            return None;
        }
        self.pos += 1;
        Some(byte)
    }

    /// Bytes from the start of the string to the cursor.
    #[inline(always)]
    pub(crate) fn position(&self) -> usize {
        self.pos
    }

    /// The cursor, as a pointer into the string.
    #[inline(always)]
    pub(crate) fn as_ptr(&self) -> *const c_char {
        self.start.wrapping_add(self.pos).cast()
    }
}

impl<const BOUNDED: bool> Iterator for CStrCursor<'_, BOUNDED> {
    type Item = NonZeroU8;

    /// The byte at the cursor, moving past it; `None` at the end.
    #[inline(always)]
    fn next(&mut self) -> Option<NonZeroU8> {
        let byte = NonZeroU8::new(self.peek())?;
        self.pos += 1;
        Some(byte)
    }
}

/// One position in two C strings walked together, for `strcmp` and `strncmp`.
///
/// Two [`CStrCursor`]s would do the same, but with a position each, and the
/// compiler unrolls a loop over two counters it cannot tell apart; one shared
/// position compiles to the plain loop.
pub(crate) struct CStrPair<'a, const BOUNDED: bool = false> {
    a: *const u8,
    b: *const u8,
    /// Bytes from the start of each string to the cursor.
    pos: usize,
    /// Bytes of each string that may be read, when `BOUNDED`.
    limit: usize,
    _strings: PhantomData<&'a [u8]>,
}

impl CStrPair<'_> {
    /// A pair at the start of `a` and `b`.
    ///
    /// # Safety
    ///
    /// `a` and `b` are NUL-terminated strings that live for `'a`.
    #[inline(always)]
    pub(crate) unsafe fn new(a: *const c_char, b: *const c_char) -> Self {
        Self { a: a.cast(), b: b.cast(), pos: 0, limit: 0, _strings: PhantomData }
    }
}

impl CStrPair<'_, true> {
    /// A pair at the start of `a` and `b` that ends after `n` bytes, or at the
    /// first NUL if it comes first.
    ///
    /// # Safety
    ///
    /// `a` and `b` are each NUL-terminated or readable for `n` bytes, and live
    /// for `'a`.
    #[inline(always)]
    pub(crate) unsafe fn bounded(a: *const c_char, b: *const c_char, n: usize) -> Self {
        Self { a: a.cast(), b: b.cast(), pos: 0, limit: n, _strings: PhantomData }
    }
}

impl<const BOUNDED: bool> CStrPair<'_, BOUNDED> {
    /// The bytes at the cursor: both 0 at the end of the `n` bytes.
    #[inline(always)]
    pub(crate) fn peek(&self) -> (u8, u8) {
        if BOUNDED && self.pos >= self.limit {
            return (0, 0);
        }
        // SAFETY: the pair only moves past bytes that are both not the NUL, so
        // `pos` is at or before the NUL of each string, and here below `limit`
        // for strings readable for `limit` bytes.
        unsafe { (self.a.add(self.pos).read(), self.b.add(self.pos).read()) }
    }

    /// Moves past the bytes at the cursor, unless either is the NUL.
    #[inline(always)]
    pub(crate) fn advance(&mut self) {
        let (p, q) = self.peek();
        if p != 0 && q != 0 {
            self.pos += 1;
        }
    }
}

/// Compares the two strings of `pair` as `unsigned char`, from the cursor to
/// the first difference or the end of either: negative, zero, or positive, as
/// `strcmp` returns.
#[inline(always)]
pub(crate) fn compare<const BOUNDED: bool>(mut pair: CStrPair<'_, BOUNDED>) -> c_int {
    loop {
        let (p, q) = pair.peek();
        if p != q || p == 0 {
            return c_int::from(p) - c_int::from(q);
        }
        pair.advance();
    }
}

/// Copies the string under `src` to `dst`, from the cursor to the end, and
/// returns how many bytes come before the end. A string bounded only by its
/// NUL is copied with the NUL; a bounded one is not, since it may end at the
/// limit instead.
///
/// This lives here so it can step the cursor's position after the store, the
/// shape the compiler keeps as the plain byte loop; stepping before it, as
/// `advance` after a store would, costs a second load or an unrolled loop.
///
/// # Safety
///
/// `dst` is writable for the bytes copied, and does not overlap them.
#[inline(always)]
pub(crate) unsafe fn copy<const BOUNDED: bool>(
    dst: *mut c_char,
    src: &mut CStrCursor<'_, BOUNDED>,
) -> usize {
    let dst = dst.cast::<u8>();
    loop {
        let byte = src.peek();
        if BOUNDED && byte == 0 {
            return src.pos;
        }
        // SAFETY: `dst` is writable for every byte copied, per the caller.
        unsafe { dst.add(src.pos).write(byte) };
        if byte == 0 {
            return src.pos;
        }
        // The byte at `pos` is not the NUL, so the cursor may move past it.
        src.pos += 1;
    }
}

#[cfg(test)]
mod tests {
    use core::ffi::CStr;

    use super::*;

    fn cursor(s: &CStr) -> CStrCursor<'_> {
        // SAFETY: `s` is NUL-terminated and borrowed for the cursor's lifetime.
        unsafe { CStrCursor::new(s.as_ptr()) }
    }

    fn bounded(s: &[u8], n: usize) -> CStrCursor<'_, true> {
        assert!(n <= s.len() || s.contains(&0));
        // SAFETY: `s` is readable for `n` bytes, or holds a NUL before them.
        unsafe { CStrCursor::bounded(s.as_ptr().cast(), n) }
    }

    fn pair<'a>(a: &'a CStr, b: &'a CStr) -> CStrPair<'a> {
        // SAFETY: both are NUL-terminated and borrowed for the pair's lifetime.
        unsafe { CStrPair::new(a.as_ptr(), b.as_ptr()) }
    }

    fn bounded_pair<'a>(a: &'a [u8], b: &'a [u8], n: usize) -> CStrPair<'a, true> {
        assert!(n <= a.len() && n <= b.len());
        // SAFETY: both are readable for `n` bytes.
        unsafe { CStrPair::bounded(a.as_ptr().cast(), b.as_ptr().cast(), n) }
    }

    #[test]
    fn cursor_stops_at_the_nul() {
        let mut c = cursor(c"ab");
        assert_eq!((c.peek(), c.position()), (b'a', 0));
        c.advance();
        assert_eq!((c.peek(), c.position()), (b'b', 1));
        c.advance();
        assert_eq!((c.peek(), c.position()), (0, 2));
        c.advance();
        assert_eq!((c.peek(), c.position()), (0, 2));
        assert_eq!(cursor(c"").peek(), 0);
        assert_eq!(cursor(c"rivet").count(), 5);
    }

    #[test]
    fn bounded_cursor_stops_at_the_limit_or_the_nul() {
        // No NUL within the limit: the end is the limit, and nothing past it is read.
        let mut c = bounded(b"abc", 2);
        assert_eq!(c.next().map(NonZeroU8::get), Some(b'a'));
        assert_eq!(c.next().map(NonZeroU8::get), Some(b'b'));
        assert_eq!(c.next(), None);
        assert_eq!(c.position(), 2);
        // A NUL within the limit ends the string first.
        assert_eq!(bounded(b"a\0bc", 4).count(), 1);
        // A limit of 0 reads nothing, not even the first byte.
        assert_eq!(bounded(b"", 0).peek(), 0);
    }

    #[test]
    fn next_if_takes_only_accepted_bytes() {
        let mut c = cursor(c"12x");
        assert_eq!(c.next_if(|b| b.is_ascii_digit()), Some(b'1'));
        assert_eq!(c.next_if(|b| b.is_ascii_digit()), Some(b'2'));
        assert_eq!(c.next_if(|b| b.is_ascii_digit()), None);
        assert_eq!(c.peek(), b'x');
        c.advance();
        assert_eq!(c.next_if(|_| true), None);
    }

    #[test]
    fn as_ptr_follows_the_cursor() {
        let s = c"abc";
        let mut c = cursor(s);
        c.advance();
        c.advance();
        assert_eq!(c.as_ptr(), s.as_ptr().wrapping_add(2));
    }

    #[test]
    fn pair_stops_at_the_first_nul_or_the_limit() {
        let mut p = pair(c"ab", c"a");
        assert_eq!(p.peek(), (b'a', b'a'));
        p.advance();
        assert_eq!(p.peek(), (b'b', 0));
        p.advance();
        assert_eq!(p.peek(), (b'b', 0));
        let mut p = bounded_pair(b"ab", b"ab", 1);
        p.advance();
        assert_eq!(p.peek(), (0, 0));
    }

    #[test]
    fn compare_orders_as_unsigned_char() {
        assert_eq!(compare(pair(c"abc", c"abc")), 0);
        assert!(compare(pair(c"ab", c"abc")) < 0);
        assert!(compare(pair(c"\x80", c"\x7f")) > 0);
        assert_eq!(compare(bounded_pair(b"abX", b"abY", 2)), 0);
        assert!(compare(bounded_pair(b"abX", b"abY", 3)) < 0);
    }

    #[test]
    fn copy_writes_through_the_nul_unless_bounded() {
        let mut dst = [0xaa_u8; 4];
        let mut src = cursor(c"ab");
        // SAFETY: `dst` holds the 3 bytes copied, and does not overlap the literal.
        let n = unsafe { copy(dst.as_mut_ptr().cast(), &mut src) };
        assert_eq!((n, dst, src.peek()), (2, [b'a', b'b', 0, 0xaa], 0));
        let mut dst = [0xaa_u8; 4];
        let mut src = bounded(b"ab\0", 3);
        // SAFETY: `dst` holds the 2 bytes copied, and does not overlap the literal.
        let n = unsafe { copy(dst.as_mut_ptr().cast(), &mut src) };
        assert_eq!((n, dst), (2, [b'a', b'b', 0xaa, 0xaa]));
        let mut dst = [0xaa_u8; 4];
        let mut src = bounded(b"abc", 2);
        // SAFETY: as above.
        let n = unsafe { copy(dst.as_mut_ptr().cast(), &mut src) };
        assert_eq!((n, dst), (2, [b'a', b'b', 0xaa, 0xaa]));
    }
}
