// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The formatter behind `printf`, `snprintf`, and their `v` forms.
//!
//! [`format`] walks a `printf` format string, takes each conversion's
//! argument from an [`Args`], and writes the result to a [`Sink`]. The
//! arguments come through a trait because stable Rust cannot read a C
//! `va_list`: on a firmware target `support::va_list` answers from the C
//! entry points in `src/stdio/variadic.c`, and the tests here answer from a
//! slice.
//!
//! It takes the flags `-`, `+`, space, `#`, and `0`, a width and precision as
//! digits or `*`, the lengths `hh`, `h`, `l`, `ll`, `j`, `z`, and `t`, and the
//! conversions `d`, `i`, `u`, `o`, `x`, `X`, `c`, `s`, `p`, and `%`. The
//! floating-point conversions, `n`, the wide conversions, and `L` are not
//! supported: they stop the formatting as an invalid conversion does.

use core::ffi::{c_char, c_int, c_long, c_ulong, c_void};

use crate::support::cstr::CStrCursor;

/// A length modifier: the type of the conversion's argument.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Length {
    /// `hh`: `char`, passed as `int`.
    Char,
    /// `h`: `short`, passed as `int`.
    Short,
    #[default]
    Int,
    /// `l`.
    Long,
    /// `ll`.
    LongLong,
    /// `j`: `intmax_t`.
    IntMax,
    /// `z`: `size_t`.
    Size,
    /// `t`: `ptrdiff_t`.
    PtrDiff,
}

/// The arguments of a conversion, in the order the format names them.
pub(crate) trait Args {
    /// The next argument as a signed integer of `length`'s type.
    fn int(&mut self, length: Length) -> i64;
    /// The next argument as an unsigned integer of `length`'s type.
    fn uint(&mut self, length: Length) -> u64;
    /// The next argument as a pointer.
    fn pointer(&mut self) -> *const c_void;
}

/// Where the formatted bytes go.
pub(crate) trait Sink {
    /// Takes `bytes`. An error ends the formatting.
    fn write(&mut self, bytes: &[u8]) -> Result<(), ()>;
}

/// Why [`format`] stopped, and the `errno` each maps to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Error {
    /// The format holds a conversion that is not supported: `EINVAL`.
    Invalid,
    /// The result is longer than `INT_MAX` bytes: `EOVERFLOW`.
    Overflow,
    /// The sink refused bytes: `EIO`.
    Output,
}

/// A conversion's flags, width, precision, and length.
#[derive(Default)]
struct Spec {
    left: bool,
    plus: bool,
    space: bool,
    alt: bool,
    zero: bool,
    width: usize,
    precision: Option<usize>,
    length: Length,
}

/// Bytes `pad` writes at a time.
const PAD_CHUNK: usize = 256;

/// The sink with the count of bytes formatted so far, which is the result.
struct Out<'a, S: Sink> {
    sink: &'a mut S,
    count: usize,
}

impl<S: Sink> Out<'_, S> {
    fn put(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.count = self.count.checked_add(bytes.len()).ok_or(Error::Overflow)?;
        if self.count > c_int::MAX as usize {
            return Err(Error::Overflow);
        }
        self.sink.write(bytes).map_err(|()| Error::Output)
    }

    fn pad(&mut self, byte: u8, mut n: usize) -> Result<(), Error> {
        let chunk = [byte; PAD_CHUNK];
        while n > 0 {
            let k = n.min(PAD_CHUNK);
            self.put(&chunk[..k])?;
            n -= k;
        }
        Ok(())
    }

    /// Writes `body` in a field of `spec.width`, padded with spaces on the
    /// side `spec.left` says.
    fn padded(&mut self, spec: &Spec, body: &[u8]) -> Result<(), Error> {
        let pad = spec.width.saturating_sub(body.len());
        if !spec.left {
            self.pad(b' ', pad)?;
        }
        self.put(body)?;
        if spec.left {
            self.pad(b' ', pad)?;
        }
        Ok(())
    }

    /// Writes a number: `prefix` (a sign and a radix prefix), then `digits`
    /// with at least `zeros` zeros before them, in a field of `spec.width`.
    /// The `0` flag fills the field with zeros after the prefix, unless a
    /// precision was given or the conversion is left-adjusted.
    fn number(
        &mut self,
        spec: &Spec,
        prefix: &[u8],
        mut zeros: usize,
        digits: &[u8],
    ) -> Result<(), Error> {
        let body = prefix.len().saturating_add(zeros).saturating_add(digits.len());
        let pad = spec.width.saturating_sub(body);
        if spec.zero && !spec.left && spec.precision.is_none() {
            zeros = zeros.saturating_add(pad);
        } else if !spec.left {
            self.pad(b' ', pad)?;
        }
        self.put(prefix)?;
        self.pad(b'0', zeros)?;
        self.put(digits)?;
        if spec.left {
            self.pad(b' ', pad)?;
        }
        Ok(())
    }
}

/// The decimal number at the cursor, saturated, or 0 if there is none.
fn decimal(f: &mut CStrCursor<'_>) -> usize {
    let mut value: usize = 0;
    while let Some(d) = f.next_if(|b| b.is_ascii_digit()) {
        value = value.saturating_mul(10).saturating_add(usize::from(d - b'0'));
    }
    value
}

/// The low `bits` of `v`, sign-extended.
fn sign_extend(v: i64, bits: u32) -> i64 {
    let shift = u64::BITS - bits;
    (v << shift) >> shift
}

/// The low `bits` of `v`.
fn truncate(v: u64, bits: u32) -> u64 {
    v & (u64::MAX >> (u64::BITS - bits))
}

/// `v` as the signed type `length` names: the argument came as `intmax_t`,
/// and `hh` and `h` were promoted to `int`, so the type's width is restored.
fn narrow_signed(v: i64, length: Length) -> i64 {
    match length {
        Length::Char => sign_extend(v, i8::BITS),
        Length::Short => sign_extend(v, i16::BITS),
        Length::Int => sign_extend(v, c_int::BITS),
        Length::Long => sign_extend(v, c_long::BITS),
        Length::LongLong | Length::IntMax => v,
        Length::Size | Length::PtrDiff => sign_extend(v, isize::BITS),
    }
}

/// `v` as the unsigned type `length` names.
fn narrow_unsigned(v: u64, length: Length) -> u64 {
    match length {
        Length::Char => truncate(v, u8::BITS),
        Length::Short => truncate(v, u16::BITS),
        Length::Int => truncate(v, c_int::BITS),
        Length::Long => truncate(v, c_ulong::BITS),
        Length::LongLong | Length::IntMax => v,
        Length::Size | Length::PtrDiff => truncate(v, usize::BITS),
    }
}

/// The digits of `v` in `base`, written at the end of `buf`; the slice holding
/// them. No digits for 0 with a precision of 0, as C says.
fn digits<'a>(buf: &'a mut [u8; 24], mut v: u64, base: u64, upper: bool, spec: &Spec) -> &'a [u8] {
    const LOWER: &[u8; 16] = b"0123456789abcdef";
    const UPPER: &[u8; 16] = b"0123456789ABCDEF";
    let table = if upper { UPPER } else { LOWER };
    let mut i = buf.len();
    if !(v == 0 && spec.precision == Some(0)) {
        loop {
            i -= 1;
            buf[i] = table[(v % base) as usize];
            v /= base;
            if v == 0 {
                break;
            }
        }
    }
    &buf[i..]
}

/// Writes an integer conversion: `negative` with the magnitude `v`, or `v`
/// unsigned, in `base`. A signed conversion carries the `+` and space flags.
fn integer<S: Sink>(
    out: &mut Out<'_, S>,
    spec: &Spec,
    signed: bool,
    negative: bool,
    v: u64,
    base: u64,
    upper: bool,
) -> Result<(), Error> {
    let mut buf = [0u8; 24];
    let digits = digits(&mut buf, v, base, upper, spec);
    let mut zeros = spec.precision.map_or(0, |p| p.saturating_sub(digits.len()));
    // `#` gives octal a leading 0, including when the value and the precision
    // are both 0 and there are no digits, and non-zero hex a radix prefix.
    if spec.alt && base == 8 && zeros == 0 && digits.first() != Some(&b'0') {
        zeros = 1;
    }
    let prefix: &[u8] = match (signed, negative, spec.plus, spec.space) {
        (true, true, _, _) => b"-",
        (true, false, true, _) => b"+",
        (true, false, false, true) => b" ",
        _ if spec.alt && base == 16 && v != 0 => {
            if upper {
                b"0X"
            } else {
                b"0x"
            }
        }
        _ => b"",
    };
    out.number(spec, prefix, zeros, digits)
}

/// Writes a `%s`: the bytes of `s` to its NUL or `spec.precision`, or
/// `(null)` for a null pointer.
///
/// # Safety
///
/// `s` is null, NUL-terminated, or readable for `spec.precision` bytes.
unsafe fn string<S: Sink>(
    out: &mut Out<'_, S>,
    spec: &Spec,
    s: *const c_char,
) -> Result<(), Error> {
    let s = if s.is_null() { c"(null)".as_ptr() } else { s };
    let len = match spec.precision {
        // SAFETY: `s` is readable for `precision` bytes or to its NUL, per the caller.
        Some(precision) => unsafe { CStrCursor::bounded(s, precision) }.count(),
        // SAFETY: `s` is NUL-terminated, per the caller.
        None => unsafe { CStrCursor::new(s) }.count(),
    };
    // SAFETY: `len` bytes of `s` were just read.
    let bytes = unsafe { core::slice::from_raw_parts(s.cast::<u8>(), len) };
    out.padded(spec, bytes)
}

/// Formats `format` with `args` into `sink`, and returns how many bytes that
/// is, as `printf` returns.
///
/// # Safety
///
/// `format` is a NUL-terminated string, and `args` yields what its
/// conversions name: a `%s` argument is null, NUL-terminated, or readable for
/// the conversion's precision.
pub(crate) unsafe fn format<S: Sink, A: Args>(
    sink: &mut S,
    format: *const c_char,
    args: &mut A,
) -> Result<usize, Error> {
    let mut out = Out { sink, count: 0 };
    // SAFETY: `format` is NUL-terminated, per the caller.
    let mut f = unsafe { CStrCursor::new(format) };
    loop {
        // The literal text up to the next conversion.
        let start = f.clone();
        while f.next_if(|b| b != b'%').is_some() {}
        let len = f.position() - start.position();
        // SAFETY: the cursor read these `len` bytes of the format.
        let text = unsafe { core::slice::from_raw_parts(start.as_ptr().cast::<u8>(), len) };
        out.put(text)?;
        if f.peek() == 0 {
            return Ok(out.count);
        }
        f.advance();

        let mut spec = Spec::default();
        while let Some(flag) = f.next_if(|b| matches!(b, b'-' | b'+' | b' ' | b'#' | b'0')) {
            match flag {
                b'-' => spec.left = true,
                b'+' => spec.plus = true,
                b' ' => spec.space = true,
                b'#' => spec.alt = true,
                _ => spec.zero = true,
            }
        }
        if f.next_if(|b| b == b'*').is_some() {
            // A negative width is a `-` flag with its magnitude.
            let width = args.int(Length::Int);
            spec.left |= width < 0;
            spec.width = width.unsigned_abs() as usize;
        } else {
            spec.width = decimal(&mut f);
        }
        if f.next_if(|b| b == b'.').is_some() {
            if f.next_if(|b| b == b'*').is_some() {
                // A negative precision is no precision.
                spec.precision = usize::try_from(args.int(Length::Int)).ok();
            } else {
                spec.precision = Some(decimal(&mut f));
            }
        }
        spec.length = match f.next_if(|b| matches!(b, b'h' | b'l' | b'j' | b'z' | b't')) {
            Some(b'h') if f.next_if(|b| b == b'h').is_some() => Length::Char,
            Some(b'h') => Length::Short,
            Some(b'l') if f.next_if(|b| b == b'l').is_some() => Length::LongLong,
            Some(b'l') => Length::Long,
            Some(b'j') => Length::IntMax,
            Some(b'z') => Length::Size,
            Some(b't') => Length::PtrDiff,
            _ => Length::Int,
        };

        let Some(conversion) = f.next_if(|_| true) else { return Err(Error::Invalid) };
        match conversion {
            b'd' | b'i' => {
                let v = narrow_signed(args.int(spec.length), spec.length);
                integer(&mut out, &spec, true, v < 0, v.unsigned_abs(), 10, false)?;
            }
            b'u' | b'o' | b'x' | b'X' => {
                let v = narrow_unsigned(args.uint(spec.length), spec.length);
                let base = match conversion {
                    b'u' => 10,
                    b'o' => 8,
                    _ => 16,
                };
                integer(&mut out, &spec, false, false, v, base, conversion == b'X')?;
            }
            // `%lc` and `%ls` take wide characters, which are not supported.
            b'c' | b's' if spec.length != Length::Int => return Err(Error::Invalid),
            b'c' => {
                // The argument is an `int`, converted to `unsigned char`.
                let c = args.int(Length::Int) as u8;
                out.padded(&spec, &[c])?;
            }
            // SAFETY: a `%s` argument is null, NUL-terminated, or readable for
            // the precision, per the caller.
            b's' => unsafe { string(&mut out, &spec, args.pointer().cast()) }?,
            b'p' => {
                // The address as `%#x` gives it: in hex with a `0x` prefix, or
                // `0` for null.
                let address = args.pointer().addr() as u64;
                let spec = Spec { alt: true, precision: None, length: Length::Size, ..spec };
                integer(&mut out, &spec, false, false, address, 16, false)?;
            }
            b'%' => out.put(b"%")?,
            _ => return Err(Error::Invalid),
        }
    }
}

/// `snprintf`'s buffer: keeps the first `cap - 1` bytes, and [`finish`]
/// writes the NUL after them. With a `cap` of 0 it keeps nothing and writes
/// nothing, so the buffer may be null.
///
/// [`finish`]: Self::finish
pub(crate) struct Buffer {
    buf: *mut u8,
    cap: usize,
    len: usize,
}

impl Buffer {
    /// A buffer over the `cap` bytes at `buf`.
    ///
    /// # Safety
    ///
    /// `buf` is writable for `cap` bytes, or `cap` is 0.
    pub(crate) unsafe fn new(buf: *mut c_char, cap: usize) -> Self {
        Self { buf: buf.cast(), cap, len: 0 }
    }

    /// Terminates the string kept.
    pub(crate) fn finish(self) {
        if self.cap > 0 {
            // SAFETY: `len < cap`, and `buf` is writable for `cap` bytes.
            unsafe { self.buf.add(self.len).write(0) };
        }
    }
}

impl Sink for Buffer {
    fn write(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let room = self.cap.saturating_sub(1).saturating_sub(self.len);
        let n = bytes.len().min(room);
        // SAFETY: `len + n < cap`, `buf` is writable for `cap` bytes, and a
        // caller's buffer never overlaps the formatter's bytes.
        unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), self.buf.add(self.len), n) };
        self.len += n;
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use core::ffi::CStr;

    use super::*;

    /// An argument a test passes, as C would.
    #[derive(Clone, Copy)]
    pub(crate) enum Arg {
        I(i64),
        U(u64),
        P(*const c_void),
        S(&'static CStr),
    }

    pub(crate) struct SliceArgs<'a>(pub(crate) &'a [Arg]);

    impl SliceArgs<'_> {
        fn next(&mut self) -> Arg {
            let (first, rest) = self.0.split_first().expect("an argument for each conversion");
            self.0 = rest;
            *first
        }
    }

    impl Args for SliceArgs<'_> {
        fn int(&mut self, _: Length) -> i64 {
            match self.next() {
                Arg::I(v) => v,
                Arg::U(v) => v as i64,
                _ => panic!("an integer argument"),
            }
        }

        fn uint(&mut self, _: Length) -> u64 {
            match self.next() {
                Arg::I(v) => v as u64,
                Arg::U(v) => v,
                _ => panic!("an integer argument"),
            }
        }

        fn pointer(&mut self) -> *const c_void {
            match self.next() {
                Arg::P(p) => p,
                Arg::S(s) => s.as_ptr().cast(),
                _ => panic!("a pointer argument"),
            }
        }
    }

    /// A sink that keeps everything.
    pub(crate) struct Keep(pub(crate) [u8; 4096], pub(crate) usize);

    impl Keep {
        pub(crate) fn new() -> Self {
            Self([0; 4096], 0)
        }

        pub(crate) fn bytes(&self) -> &[u8] {
            &self.0[..self.1]
        }
    }

    impl Sink for Keep {
        fn write(&mut self, bytes: &[u8]) -> Result<(), ()> {
            self.0[self.1..self.1 + bytes.len()].copy_from_slice(bytes);
            self.1 += bytes.len();
            Ok(())
        }
    }

    /// A sink that counts and drops.
    struct Count;

    impl Sink for Count {
        fn write(&mut self, _: &[u8]) -> Result<(), ()> {
            Ok(())
        }
    }

    /// A sink that refuses everything.
    struct Refuse;

    impl Sink for Refuse {
        fn write(&mut self, _: &[u8]) -> Result<(), ()> {
            Err(())
        }
    }

    /// Formats `f` with `args` and checks the result against `expected`.
    #[track_caller]
    pub(crate) fn check(f: &CStr, args: &[Arg], expected: &str) {
        let mut keep = Keep::new();
        // SAFETY: `f` is NUL-terminated, and `args` match its conversions.
        let n = unsafe { format(&mut keep, f.as_ptr(), &mut SliceArgs(args)) };
        let got = core::str::from_utf8(keep.bytes()).expect("ASCII");
        assert_eq!(got, expected, "format {f:?}");
        assert_eq!(n, Ok(expected.len()), "count of {f:?}");
    }

    #[test]
    fn format_copies_text_and_percent() {
        check(c"", &[], "");
        check(c"plain text\n", &[], "plain text\n");
        check(c"100%%", &[], "100%");
        check(c"%%%d%%", &[Arg::I(5)], "%5%");
    }

    #[test]
    fn format_integers_with_width_precision_and_flags() {
        for (f, v, expected) in [
            (c"%d", 0, "0"),
            (c"%d", -12, "-12"),
            (c"%04d", 12, "0012"),
            (c"%.3d", 12, "012"),
            (c"%3d", 12, " 12"),
            (c"%-3d", 12, "12 "),
            (c"%+3d", 12, "+12"),
            (c"%+-5d", 12, "+12  "),
            (c"%+- 5d", 12, "+12  "),
            (c"%- 5d", 12, " 12  "),
            (c"% d", 12, " 12"),
            (c"%0-5d", 12, "12   "),
            (c"%-05d", 12, "12   "),
            (c"%05d", -12, "-0012"),
            (c"%.0d", 0, ""),
            (c"%.0o", 0, ""),
            (c"%#.0d", 0, ""),
            (c"%#.0o", 0, "0"),
            (c"%#.0o", 15, "017"),
            (c"%#.0x", 0, ""),
            (c"%2.0u", 0, "  "),
            (c"%02.0u", 0, "  "),
            (c"%2.0d", 0, "  "),
            (c"%02.0d", 0, "  "),
            (c"% .0d", 0, " "),
            (c"%+.0d", 0, "+"),
            (c"%x", 63, "3f"),
            (c"%#x", 63, "0x3f"),
            (c"%X", 63, "3F"),
            (c"%#X", 63, "0X3F"),
            (c"%#x", 0, "0"),
            (c"%o", 15, "17"),
            (c"%#o", 15, "017"),
            (c"%#o", 0, "0"),
            (c"%#.3o", 15, "017"),
            (c"%#8x", 63, "    0x3f"),
            (c"%#08x", 63, "0x00003f"),
            (c"%-#8x|", 63, "0x3f    |"),
            (c"%i", -7, "-7"),
            (c"%u", -1, "4294967295"),
            (c"%d", i64::from(i32::MIN), "-2147483648"),
        ] {
            check(f, &[Arg::I(v)], expected);
        }
    }

    /// A `long` argument, whose width is the target's.
    fn long(v: impl Into<i64>) -> Arg {
        Arg::I(v.into())
    }

    /// An `unsigned long` argument, whose width is the target's.
    fn ulong(v: impl Into<u64>) -> Arg {
        Arg::U(v.into())
    }

    #[test]
    fn format_narrows_to_the_length_modifier() {
        check(c"%hhd", &[Arg::I(300)], "44");
        check(c"%hhu", &[Arg::I(-1)], "255");
        check(c"%hd", &[Arg::I(70000)], "4464");
        check(c"%hu", &[Arg::I(-1)], "65535");
        check(c"%hx", &[Arg::I(0x1_2345)], "2345");
        check(c"%lld", &[Arg::I(i64::MIN)], "-9223372036854775808");
        check(c"%llu", &[Arg::U(u64::MAX)], "18446744073709551615");
        check(c"%jd", &[Arg::I(-5)], "-5");
        check(c"%llx", &[Arg::U(0xdead_beef_cafe)], "deadbeefcafe");
        check(c"%zu", &[Arg::U(usize::MAX as u64)], "18446744073709551615");
        check(c"%zd", &[Arg::I(-1)], "-1");
        check(c"%td", &[Arg::I(-3)], "-3");
        check(c"%ld", &[long(c_long::MIN)], "-9223372036854775808");
        check(c"%lu", &[ulong(c_ulong::MAX)], "18446744073709551615");
    }

    #[test]
    fn format_takes_width_and_precision_from_arguments() {
        check(c"%*d", &[Arg::I(5), Arg::I(42)], "   42");
        check(c"%-*d|", &[Arg::I(5), Arg::I(42)], "42   |");
        check(c"%*d|", &[Arg::I(-5), Arg::I(42)], "42   |");
        check(c"%.*d", &[Arg::I(4), Arg::I(42)], "0042");
        check(c"%.*d", &[Arg::I(-4), Arg::I(42)], "42");
        check(c"%*.*d", &[Arg::I(7), Arg::I(4), Arg::I(42)], "   0042");
        check(c"%.*s", &[Arg::I(2), Arg::S(c"hello")], "he");
    }

    #[test]
    fn format_strings_and_characters() {
        check(c"%s", &[Arg::S(c"hello")], "hello");
        check(c"%s", &[Arg::S(c"")], "");
        check(c"%.2s", &[Arg::S(c"hello")], "he");
        check(c"%.10s", &[Arg::S(c"hello")], "hello");
        check(c"%.0s", &[Arg::S(c"hello")], "");
        check(c"%7s|", &[Arg::S(c"hello")], "  hello|");
        check(c"%-7s|", &[Arg::S(c"hello")], "hello  |");
        check(c"%4.2s|", &[Arg::S(c"hello")], "  he|");
        check(c"%s", &[Arg::P(core::ptr::null())], "(null)");
        check(c"%.3s", &[Arg::P(core::ptr::null())], "(nu");
        check(c"%c", &[Arg::I(i64::from(b'x'))], "x");
        check(c"%c", &[Arg::I(0x1_41)], "A");
        check(c"%3c|", &[Arg::I(i64::from(b'x'))], "  x|");
        check(c"%-3c|", &[Arg::I(i64::from(b'x'))], "x  |");
        check(c"[%s=%d]", &[Arg::S(c"n"), Arg::I(3)], "[n=3]");
    }

    #[test]
    fn format_precision_reads_no_further_than_it_needs() {
        // Not NUL-terminated: only the precision keeps the read in bounds.
        let bytes = *b"abc";
        let s = bytes.as_ptr().cast::<c_void>();
        check(c"%.3s", &[Arg::P(s)], "abc");
        check(c"%.2s", &[Arg::P(s)], "ab");
    }

    #[test]
    fn format_pointers_in_hex_with_a_prefix() {
        let p = core::ptr::without_provenance::<c_void>(0x1f_2e3d);
        check(c"%p", &[Arg::P(p)], "0x1f2e3d");
        check(c"%12p|", &[Arg::P(p)], "    0x1f2e3d|");
        check(c"%-12p|", &[Arg::P(p)], "0x1f2e3d    |");
        check(c"%p", &[Arg::P(core::ptr::null())], "0");
    }

    #[test]
    fn format_stops_at_an_invalid_conversion() {
        for f in [c"%n", c"%", c"%5", c"%ls", c"%lc", c"%hs", c"%Lf", c"%f", c"%a", c"%q", c"%h"] {
            let mut keep = Keep::new();
            // SAFETY: `f` is NUL-terminated; the conversion is rejected before any argument.
            let result = unsafe { format(&mut keep, f.as_ptr(), &mut SliceArgs(&[])) };
            assert_eq!(result, Err(Error::Invalid), "{f:?}");
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "pads INT_MAX bytes, which Miri interprets one by one")]
    fn format_reports_a_refusing_sink_and_a_result_past_int_max() {
        // SAFETY: NUL-terminated, no conversions.
        let result = unsafe { format(&mut Refuse, c"x".as_ptr(), &mut SliceArgs(&[])) };
        assert_eq!(result, Err(Error::Output));
        // Exactly INT_MAX bytes fit; one more does not.
        let max = i64::from(c_int::MAX);
        // SAFETY: NUL-terminated, with the precision and value as arguments.
        let result = unsafe {
            format(&mut Count, c"%.*u".as_ptr(), &mut SliceArgs(&[Arg::I(max), Arg::I(0)]))
        };
        assert_eq!(result, Ok(c_int::MAX as usize));
        // SAFETY: as above.
        let result = unsafe {
            format(&mut Count, c"%.*u ".as_ptr(), &mut SliceArgs(&[Arg::I(max), Arg::I(0)]))
        };
        assert_eq!(result, Err(Error::Overflow));
    }

    #[test]
    fn buffer_keeps_what_fits_and_terminates() {
        let mut bytes = [b'x'; 8];
        // SAFETY: `bytes` holds 4 bytes from its start.
        let mut buffer = unsafe { Buffer::new(bytes.as_mut_ptr().cast(), 4) };
        assert_eq!(buffer.write(b"123456"), Ok(()));
        assert_eq!(buffer.write(b"78"), Ok(()));
        buffer.finish();
        assert_eq!(&bytes, b"123\0xxxx");
        let mut bytes = [b'x'; 8];
        // SAFETY: `bytes` holds 8 bytes.
        let mut buffer = unsafe { Buffer::new(bytes.as_mut_ptr().cast(), 8) };
        assert_eq!(buffer.write(b"12"), Ok(()));
        buffer.finish();
        assert_eq!(&bytes, b"12\0xxxxx");
        // A capacity of 0 writes nothing, so the pointer may be anything.
        // SAFETY: `cap` is 0.
        let mut buffer = unsafe { Buffer::new(core::ptr::null_mut(), 0) };
        assert_eq!(buffer.write(b"12"), Ok(()));
        buffer.finish();
    }
}
