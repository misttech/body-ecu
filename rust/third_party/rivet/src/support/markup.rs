// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.
//
// Ported from upstream, under this notice:
//
// Copyright 2022 The Fuchsia Authors. All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are
// met:
//
//    * Redistributions of source code must retain the above copyright
// notice, this list of conditions and the following disclaimer.
//    * Redistributions in binary form must reproduce the above
// copyright notice, this list of conditions and the following disclaimer
// in the documentation and/or other materials provided with the
// distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
// OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
// LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
// THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

//! Symbolizer markup: the `{{{bt:...}}}`, `{{{module:...}}}`, and other
//! elements a host symbolizer, such as `llvm-symbolizer --filter-markup`,
//! turns back into function names and source lines.
//!
//! Ported from Fuchsia's `src/lib/symbolizer-markup`: the writer
//! (`include/lib/symbolizer-markup/writer.h`), its line-buffered sink
//! (`line-buffered-sink.h`), and their tests (`unittests.cc`), at
//! https://github.com/misttech/fuchsia revision
//! `0703a5c2d237c2c45afddbf4deb6e1cfc2b571ac`, with these changes:
//!
//! - A sink is anything that implements [`Sink`], which closures taking a
//!   `&str` do, rather than any callable.
//! - The writer takes `&self` and keeps its sink in a `RefCell`, so a color
//!   guard can hold the writer while the caller goes on writing through it,
//!   as upstream's reference does.
//! - The line-buffered sink is one type, [`LineBuffered`], with the buffer
//!   size as a const parameter, rather than a nested template.
//! - The tests below are upstream's, each one Rust test, collecting markup
//!   in a fixed buffer rather than a `std::string`.

use core::cell::RefCell;

/// Where markup goes: each call is part of one element, or a newline.
pub(crate) trait Sink {
    fn write(&mut self, s: &str);
}

impl<F: FnMut(&str)> Sink for F {
    fn write(&mut self, s: &str) {
        self(s);
    }
}

/// A supported output color, whose value derives from the corresponding SGR
/// control sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[allow(dead_code, reason = "upstream's full set of colors")]
pub(crate) enum Color {
    Default = 0,
    Black = 30,
    Red = 31,
    Green = 32,
    Yellow = 33,
    Blue = 34,
    Magenta = 35,
    Cyan = 36,
    White = 37,
}

/// Permissions attached to a region of memory.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MemoryPermissions {
    pub(crate) read: bool,
    pub(crate) write: bool,
    pub(crate) execute: bool,
}

const DECIMAL_DIGITS: &[u8; 10] = b"0123456789";
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

const BT: &str = "bt";
const DATA: &str = "data";
const DUMPFILE: &str = "dumpfile";
const ELF: &str = "elf";
const LOAD: &str = "load";
const MMAP: &str = "mmap";
const MODULE: &str = "module";
const PC: &str = "pc";
const RA: &str = "ra";
const RESET: &str = "reset";
const SYMBOL: &str = "symbol";
const BEGIN_ELEMENT: &str = "{{{";
const END_ELEMENT: &str = "}}}";
const HEX_PREFIX: &str = "0x";

/// Emits symbolizer markup. Each call represents a single markup element.
pub(crate) struct Writer<S: Sink> {
    sink: RefCell<S>,
}

/// Changes the color back to the default when it drops.
pub(crate) struct ResetColor<'a, S: Sink> {
    writer: &'a Writer<S>,
}

impl<S: Sink> Drop for ResetColor<'_, S> {
    fn drop(&mut self) {
        self.writer.literal("\x1b[0m");
    }
}

#[allow(dead_code, reason = "upstream's full set of elements; rivet prints a backtrace's")]
impl<S: Sink> Writer<S> {
    pub(crate) fn new(sink: S) -> Self {
        Self { sink: RefCell::new(sink) }
    }

    /// The sink back, once writing is done.
    pub(crate) fn into_sink(self) -> S {
        self.sink.into_inner()
    }

    //
    // Colorization.
    //

    /// Changes the output color, optionally boldened. Once the return value
    /// goes out of scope, the color is changed back to the default. Among
    /// subsequent calls, 'last wins'.
    pub(crate) fn change_color(&self, color: Color, bold: bool) -> ResetColor<'_, S> {
        self.literal("\x1b[").decimal_digits(color as u64).literal_char('m');
        if bold {
            self.literal("\x1b[1m");
        }
        ResetColor { writer: self }
    }

    //
    // Presentation elements.
    //

    /// Emits the markup for a symbol or type, given its linkage name:
    /// `{{{symbol:$name}}}`.
    pub(crate) fn symbol(&self, name: &str) -> &Self {
        self.begin_element(SYMBOL).field(name).end_element()
    }

    /// Emits the markup for the memory address of a code location:
    /// `{{{pc:$addr}}}`.
    pub(crate) fn code(&self, pc: u64) -> &Self {
        self.begin_element(PC).hex_field(pc).end_element()
    }

    /// Emits the markup for the memory address of a data location:
    /// `{{{data:$addr}}}`.
    pub(crate) fn data(&self, addr: u64) -> &Self {
        self.begin_element(DATA).hex_field(addr).end_element()
    }

    /// Emits the markup for a backtrace frame off of the callstack:
    /// `{{{bt:$frame:$pc:ra}}}`.
    pub(crate) fn return_address_frame(&self, frame: u32, pc: u64) -> &Self {
        self.begin_element(BT).decimal_field(u64::from(frame)).hex_field(pc).field(RA).end_element()
    }

    /// Emits the markup for a backtrace frame leading into an interrupt:
    /// `{{{bt:$frame:$pc:pc}}}`.
    pub(crate) fn exact_pc_frame(&self, frame: u32, pc: u64) -> &Self {
        self.begin_element(BT).decimal_field(u64::from(frame)).hex_field(pc).field(PC).end_element()
    }

    //
    // Trigger elements.
    //

    /// Emits the markup for a dumpfile, given its type and name:
    /// `{{{dumpfile:$type:$name}}}`.
    pub(crate) fn dumpfile(&self, kind: &str, name: &str, name_suffix: &str) -> &Self {
        self.begin_element(DUMPFILE).field(kind).field(name).literal(name_suffix).end_element()
    }

    //
    // Contextual elements.
    //

    /// Emits the markup to reset the context: `{{{reset}}}`.
    pub(crate) fn reset(&self) -> &Self {
        self.begin_element(RESET).end_element()
    }

    /// Emits the markup for a given ELF module:
    /// `{{{module:$id:$name:elf:$build_id}}}`.
    pub(crate) fn elf_module(&self, id: u32, name: &str, build_id: &[u8]) -> &Self {
        self.begin_element(MODULE)
            .decimal_field(u64::from(id))
            .field(name)
            .field(ELF)
            .hex_bytes_field(build_id)
            .end_element()
    }

    /// Emits the markup for the load image of a module. The given permissions
    /// must admit at least one of reading, writing, or execution:
    /// `{{{mmap:$start:$size:load:$module_id:$perms:$static_start}}}`.
    pub(crate) fn load_image_mmap(
        &self,
        start: u64,
        size: u64,
        module_id: u32,
        perms: MemoryPermissions,
        static_start: u64,
    ) -> &Self {
        assert!(perms.read || perms.write || perms.execute);
        let mut perm_str = [0u8; 3];
        let mut perm_size = 0;
        for (allowed, letter) in [(perms.read, b'r'), (perms.write, b'w'), (perms.execute, b'x')] {
            if allowed {
                perm_str[perm_size] = letter;
                perm_size += 1;
            }
        }
        // The letters are ASCII.
        let perm_str = core::str::from_utf8(&perm_str[..perm_size]).unwrap_or_default();
        self.begin_element(MMAP)
            .hex_field(start)
            .hex_field(size)
            .field(LOAD)
            .decimal_field(u64::from(module_id))
            .field(perm_str)
            .hex_field(static_start)
            .end_element()
    }

    //
    // Helpers for writing markup fragments.
    //

    pub(crate) fn literal(&self, s: &str) -> &Self {
        if !s.is_empty() {
            self.sink.borrow_mut().write(s);
        }
        self
    }

    pub(crate) fn literals(&self, strs: &[&str]) -> &Self {
        for s in strs {
            self.literal(s);
        }
        self
    }

    pub(crate) fn literal_char(&self, c: char) -> &Self {
        let mut buf = [0u8; 4];
        self.literal(c.encode_utf8(&mut buf))
    }

    pub(crate) fn newline(&self) -> &Self {
        self.literal_char('\n')
    }

    /// Emits "$prefix: ", a conventional way of establishing the context of a
    /// line of emitted markup.
    pub(crate) fn prefix(&self, prefix: &str) -> &Self {
        if !prefix.is_empty() {
            self.literal(prefix).literal(": ");
        }
        self
    }

    /// Emits the decimal digits for a given unsigned integer. Leading zeroes
    /// are not emitted.
    pub(crate) fn decimal_digits(&self, n: u64) -> &Self {
        self.digits::<10>(n)
    }

    /// Emits the hexadecimal digits for a given unsigned integer. Leading
    /// zeroes are not emitted, but a leading "0x" is.
    pub(crate) fn hex_digits(&self, n: u64) -> &Self {
        self.literal(HEX_PREFIX).digits::<16>(n)
    }

    /// Emits plain hex digits for each byte with no separators.
    pub(crate) fn hex_string(&self, bytes: &[u8]) -> &Self {
        for &b in bytes {
            let hex = [HEX_DIGITS[usize::from(b >> 4)], HEX_DIGITS[usize::from(b & 0xf)]];
            // The digits are ASCII.
            self.literal(core::str::from_utf8(&hex).unwrap_or_default());
        }
        self
    }

    /// Emits the digits for a given unsigned integer, for a base of either 10
    /// or 16. Leading zeroes are not emitted.
    fn digits<const BASE: u64>(&self, mut n: u64) -> &Self {
        const { assert!(BASE == 10 || BASE == 16) };
        if n == 0 {
            return self.literal_char('0');
        }
        // 20 decimal digits hold `u64::MAX`, more than its 16 hex digits.
        let mut digits = [0u8; 20];
        let table: &[u8] = if BASE == 16 { HEX_DIGITS } else { DECIMAL_DIGITS };
        let mut i = digits.len();
        while n > 0 {
            i -= 1;
            digits[i] = table[(n % BASE) as usize];
            n /= BASE;
        }
        // The digits are ASCII.
        self.literal(core::str::from_utf8(&digits[i..]).unwrap_or_default())
    }

    fn separator(&self) -> &Self {
        self.literal_char(':')
    }

    fn begin_element(&self, name: &str) -> &Self {
        self.literal(BEGIN_ELEMENT).literal(name)
    }

    fn end_element(&self) -> &Self {
        self.literal(END_ELEMENT)
    }

    //
    // Helpers for writing markup fields.
    //

    fn field(&self, s: &str) -> &Self {
        self.separator().literal(s)
    }

    fn decimal_field(&self, n: u64) -> &Self {
        self.separator().decimal_digits(n)
    }

    fn hex_field(&self, n: u64) -> &Self {
        self.separator().hex_digits(n)
    }

    fn hex_bytes_field(&self, bytes: &[u8]) -> &Self {
        self.separator().hex_string(bytes)
    }
}

/// Wraps another sink and buffers: the inner sink is called with whole lines
/// including '\n' at the end, or with a full buffer that's a partial line
/// because the writer produced a line longer than the buffer size (or didn't
/// finish the line before the sink was dropped).
pub(crate) struct LineBuffered<const N: usize, S: Sink> {
    line_sink: S,
    buffer: [u8; N],
    used: usize,
}

impl<const N: usize, S: Sink> LineBuffered<N, S> {
    pub(crate) fn new(line_sink: S) -> Self {
        Self { line_sink, buffer: [0; N], used: 0 }
    }

    /// Sends what the buffer holds to the inner sink.
    fn flush(&mut self) {
        let used = core::mem::take(&mut self.used);
        if used > 0 {
            // Only whole `&str`s are copied in, and a split falls between
            // them or at a newline, which leaves the bytes valid UTF-8 unless
            // a full buffer cut a character, which the lossy form covers.
            let chunk = self.buffer.get(..used).unwrap_or_default();
            match core::str::from_utf8(chunk) {
                Ok(s) => self.line_sink.write(s),
                Err(e) => {
                    let (valid, _) = chunk.split_at(e.valid_up_to());
                    self.line_sink.write(core::str::from_utf8(valid).unwrap_or_default());
                }
            }
        }
    }
}

impl<const N: usize, S: Sink> Sink for LineBuffered<N, S> {
    fn write(&mut self, s: &str) {
        assert!(self.used < N); // Previous call should have flushed if full.
        let mut bytes = s.as_bytes();
        while !bytes.is_empty() {
            let left = N - self.used;
            let n = bytes.len().min(left);
            assert!(n > 0);
            self.buffer[self.used..self.used + n].copy_from_slice(&bytes[..n]);
            self.used += n;
            let last = bytes[bytes.len() - 1];
            bytes = &bytes[n..];
            // The buffer is full, or the string ended with a newline. The
            // writer always makes a separate call for a newline, so the whole
            // string need not be searched.
            if self.used == N || (bytes.is_empty() && last == b'\n') {
                self.flush();
            }
        }
        assert!(self.used < N); // Last iteration should have flushed if empty.
    }
}

impl<const N: usize, S: Sink> Drop for LineBuffered<N, S> {
    fn drop(&mut self) {
        // The last use should have been a newline, but rather than enforce
        // that, just be sure to flush.
        self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Markup collected in a fixed buffer.
    struct Text {
        buf: [u8; 1024],
        len: usize,
    }

    impl Text {
        fn new() -> Self {
            Self { buf: [0; 1024], len: 0 }
        }

        fn as_str(&self) -> &str {
            core::str::from_utf8(&self.buf[..self.len]).unwrap()
        }
    }

    impl Sink for &mut Text {
        fn write(&mut self, s: &str) {
            self.buf[self.len..self.len + s.len()].copy_from_slice(s.as_bytes());
            self.len += s.len();
        }
    }

    fn markup(write: impl FnOnce(&Writer<&mut Text>)) -> Text {
        let mut text = Text::new();
        write(&Writer::new(&mut text));
        text
    }

    #[test]
    fn literals() {
        let text = markup(|w| {
            w.literal("ab").literal_char('c').newline().literal("123");
        });
        assert_eq!(text.as_str(), "abc\n123");
    }

    #[test]
    fn decimal_digits() {
        let text = markup(|w| {
            for n in [
                0,
                1,
                9,
                10,
                123,
                100_000_000,
                123_454_321,
                12_345_678_987_654_321,
                9_999_999_999_999_999_999,
            ] {
                w.decimal_digits(n).newline();
            }
        });
        assert_eq!(
            text.as_str(),
            "0\n1\n9\n10\n123\n100000000\n123454321\n12345678987654321\n9999999999999999999\n"
        );
    }

    #[test]
    fn hex_digits() {
        let text = markup(|w| {
            for n in [
                0x0,
                0x1,
                0xa,
                0xff,
                0xabc,
                0xffff_ffff,
                0xabc_dcba,
                0x1234_5678_90ab_cdef,
                0xffff_ffff_ffff_ffff,
            ] {
                w.hex_digits(n).newline();
            }
        });
        assert_eq!(
            text.as_str(),
            "0x0\n0x1\n0xa\n0xff\n0xabc\n0xffffffff\n0xabcdcba\n0x1234567890abcdef\n0xffffffffffffffff\n"
        );
    }

    #[test]
    fn colors() {
        const DEFAULT: &str = "\x1b[0m";
        const BOLD: &str = "\x1b[1m";
        const BLACK: &str = "\x1b[30m";
        const GREEN: &str = "\x1b[32m";
        const MAGENTA: &str = "\x1b[35m";
        let mut text = Text::new();
        {
            let w = Writer::new(&mut text);
            {
                let _magenta = w.change_color(Color::Magenta, false);
                let _black_bold = w.change_color(Color::Black, true);
                {
                    let _green = w.change_color(Color::Green, false);
                }
                // With `green` out of scope, the color changes back to the
                // default.
            }
            // With `magenta` and `black_bold` out of scope, the color changes
            // back to the default twice again.
        }
        let expected = [MAGENTA, BLACK, BOLD, GREEN, DEFAULT, DEFAULT, DEFAULT].concat_into();
        assert_eq!(text.as_str(), expected.as_str());
    }

    /// `[&str]::concat` for a test without `alloc`.
    trait ConcatInto {
        fn concat_into(&self) -> Text;
    }

    impl ConcatInto for [&str] {
        fn concat_into(&self) -> Text {
            let mut text = Text::new();
            let mut sink = &mut text;
            for s in self {
                sink.write(s);
            }
            text
        }
    }

    #[test]
    fn symbol() {
        let text = markup(|w| {
            w.symbol("_ZN7Mangled4NameEv").newline().symbol("foobar");
        });
        assert_eq!(text.as_str(), "{{{symbol:_ZN7Mangled4NameEv}}}\n{{{symbol:foobar}}}");
    }

    #[test]
    fn code() {
        let text = markup(|w| {
            w.code(0xffff_ffff_0000_abcd)
                .newline()
                .code(0x1234_5678_0000_0000)
                .newline()
                .code(0x123)
                .newline()
                .code(0x0);
        });
        assert_eq!(
            text.as_str(),
            "{{{pc:0xffffffff0000abcd}}}\n{{{pc:0x1234567800000000}}}\n{{{pc:0x123}}}\n{{{pc:0x0}}}"
        );
    }

    #[test]
    fn data() {
        let text = markup(|w| {
            w.data(0xffff_ffff_0000_abcd)
                .newline()
                .data(0x1234_5678_0000_0000)
                .newline()
                .data(0x123)
                .newline()
                .data(0x0);
        });
        assert_eq!(
            text.as_str(),
            "{{{data:0xffffffff0000abcd}}}\n{{{data:0x1234567800000000}}}\n{{{data:0x123}}}\n{{{data:0x0}}}"
        );
    }

    #[test]
    fn backtrace_frame() {
        let text = markup(|w| {
            w.exact_pc_frame(9, 0xffff_ffff_0000_abcd)
                .newline()
                .return_address_frame(10, 0x1234_5678)
                .newline()
                .return_address_frame(11, 0x5555_5555);
        });
        assert_eq!(
            text.as_str(),
            "{{{bt:9:0xffffffff0000abcd:pc}}}\n{{{bt:10:0x12345678:ra}}}\n{{{bt:11:0x55555555:ra}}}"
        );
    }

    #[test]
    fn dumpfile() {
        let text = markup(|w| {
            w.dumpfile("TYPE", "NAME", "").newline().dumpfile("sancov", "sancov.8675", "");
        });
        assert_eq!(text.as_str(), "{{{dumpfile:TYPE:NAME}}}\n{{{dumpfile:sancov:sancov.8675}}}");
    }

    #[test]
    fn reset() {
        let text = markup(|w| {
            w.reset();
        });
        assert_eq!(text.as_str(), "{{{reset}}}");
    }

    #[test]
    fn module() {
        const BUILD_ID_A: [u8; 8] = [0x54, 0x59, 0x75, 0x39, 0x4d, 0x10, 0xa0, 0x7d];
        const BUILD_ID_B: [u8; 8] = [0xba, 0x43, 0xd6, 0xf6, 0x91, 0x1e, 0x87, 0x23];
        let text = markup(|w| {
            w.elf_module(5, "moduleA", &BUILD_ID_A).newline().elf_module(
                10,
                "moduleB",
                &BUILD_ID_B,
            );
        });
        assert_eq!(
            text.as_str(),
            "{{{module:5:moduleA:elf:545975394d10a07d}}}\n{{{module:10:moduleB:elf:ba43d6f6911e8723}}}"
        );
    }

    #[test]
    fn load_image_mmap() {
        let perms = |read, write, execute| MemoryPermissions { read, write, execute };
        let text = markup(|w| {
            w.load_image_mmap(0x1000_0000, 0x1000, 0, perms(true, false, false), 0x400)
                .newline()
                .load_image_mmap(0x2000_0000, 0x2000, 1, perms(true, true, false), 0x800)
                .newline()
                .load_image_mmap(0x3000_0000, 0x3000, 2, perms(true, true, true), 0xc00)
                .newline()
                .load_image_mmap(0x4000_0000, 0x4000, 3, perms(true, false, true), 0x1000)
                .newline()
                .load_image_mmap(0x5000_0000, 0x5000, 4, perms(false, true, false), 0x1400)
                .newline()
                .load_image_mmap(0x6000_0000, 0x6000, 5, perms(false, true, true), 0x1800)
                .newline()
                .load_image_mmap(0x7000_0000, 0x7000, 6, perms(false, false, true), 0x1c00);
        });
        assert_eq!(
            text.as_str(),
            "{{{mmap:0x10000000:0x1000:load:0:r:0x400}}}\n\
             {{{mmap:0x20000000:0x2000:load:1:rw:0x800}}}\n\
             {{{mmap:0x30000000:0x3000:load:2:rwx:0xc00}}}\n\
             {{{mmap:0x40000000:0x4000:load:3:rx:0x1000}}}\n\
             {{{mmap:0x50000000:0x5000:load:4:w:0x1400}}}\n\
             {{{mmap:0x60000000:0x6000:load:5:wx:0x1800}}}\n\
             {{{mmap:0x70000000:0x7000:load:6:x:0x1c00}}}"
        );
    }

    /// The chunks a sink was called with, in order.
    struct Calls {
        chunks: [[u8; 128]; 4],
        lens: [usize; 4],
        n: usize,
    }

    impl Calls {
        fn new() -> Self {
            Self { chunks: [[0; 128]; 4], lens: [0; 4], n: 0 }
        }

        fn get(&self, i: usize) -> &str {
            core::str::from_utf8(&self.chunks[i][..self.lens[i]]).unwrap()
        }
    }

    impl Sink for &mut Calls {
        fn write(&mut self, s: &str) {
            self.chunks[self.n][..s.len()].copy_from_slice(s.as_bytes());
            self.lens[self.n] = s.len();
            self.n += 1;
        }
    }

    // Upstream's assertions: a buffer with no room takes no bytes, which would
    // otherwise loop forever.
    #[test]
    #[should_panic(expected = "self.used < N")]
    fn a_line_buffered_sink_without_room_fails_its_assertion() {
        let mut sink = LineBuffered::<0, _>::new(|_: &str| {});
        sink.write("x");
    }

    #[test]
    fn line_buffered_sink() {
        const BUFFER_SIZE: usize = 64;

        // Each Reset() sends three chunks to the line-buffered sink, which
        // sends each whole line on once.
        let mut calls = Calls::new();
        {
            let w = Writer::new(LineBuffered::<BUFFER_SIZE, _>::new(&mut calls));
            w.reset().newline().reset().newline();
        }
        assert_eq!(calls.n, 2);
        assert_eq!((calls.get(0), calls.get(1)), ("{{{reset}}}\n", "{{{reset}}}\n"));

        // An unfinished line gets flushed when the sink drops.
        let mut calls = Calls::new();
        {
            let w = Writer::new(LineBuffered::<BUFFER_SIZE, _>::new(&mut calls));
            w.literal("first line").newline().literal("unfinished line has no newline");
        }
        assert_eq!(calls.n, 2);
        assert_eq!(
            (calls.get(0), calls.get(1)),
            ("first line\n", "unfinished line has no newline")
        );

        // A whole buffer is flushed when exactly full.
        const LONG: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n";
        let mut calls = Calls::new();
        {
            let mut sink = LineBuffered::<BUFFER_SIZE, _>::new(&mut calls);
            sink.write(LONG);
        }
        assert_eq!(calls.n, 2);
        assert_eq!((calls.get(0), calls.get(1)), (&LONG[..BUFFER_SIZE], &LONG[BUFFER_SIZE..]));
    }
}
