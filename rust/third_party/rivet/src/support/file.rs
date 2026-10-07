// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `FILE`: a stream as a write callback and the context it takes.
//!
//! rivet has no file system, so a stream is only where output goes. Every
//! output function, `printf` through `fwrite`, ends in [`FILE::write_all`],
//! which calls the stream's callback. `stdout` and `stderr` are two such
//! streams whose callbacks call the firmware's `write`, and firmware points a
//! stream anywhere else, a log buffer or a second UART, by giving it another
//! callback.

use core::cell::UnsafeCell;
use core::ffi::{c_char, c_int, c_void};
use core::fmt;
#[cfg(test)]
use core::ptr;

#[cfg(any(target_os = "none", test))]
use crate::support::format::Sink;

/// The callback of a stream: writes the `n` bytes at `s` and returns `n`, or
/// returns a negative value when it failed. rivet never passes more than
/// `INT_MAX` bytes in one call.
pub type WriteFn = unsafe extern "C" fn(ctx: *mut c_void, s: *const c_char, n: usize) -> c_int;

/// A stream, as `<stdio.h>` declares `struct __rivet_file`.
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct FILE {
    /// Writes to the stream; null for a stream that takes no output.
    pub write: Option<WriteFn>,
    /// What `write` is called with.
    pub ctx: *mut c_void,
}

zr::static_assert!(size_of::<FILE>() == 2 * size_of::<usize>());
zr::static_assert!(align_of::<FILE>() == align_of::<usize>());

impl FILE {
    /// A stream that writes through `write` with `ctx`.
    pub const fn new(write: WriteFn, ctx: *mut c_void) -> Self {
        Self { write: Some(write), ctx }
    }

    /// Writes every byte of `bytes` to the stream, in calls of at most
    /// `INT_MAX` bytes. Fails if the stream has no callback, or the callback
    /// reports anything but the length it was given.
    pub(crate) fn write_all(&mut self, mut bytes: &[u8]) -> Result<(), ()> {
        let Some(write) = self.write else { return Err(()) };
        while !bytes.is_empty() {
            let n = bytes.len().min(c_int::MAX as usize);
            // SAFETY: `bytes` is readable for `n` bytes, and `ctx` is the
            // context the stream's owner paired with `write`.
            let written = unsafe { write(self.ctx, bytes.as_ptr().cast(), n) };
            if usize::try_from(written) != Ok(n) {
                #[cfg(feature = "forkpoint-coverage")]
                forkpoint::assert_sometimes!(true, "stdio: a stream's write fails");
                return Err(());
            }
            bytes = bytes.get(n..).unwrap_or_default();
        }
        Ok(())
    }
}

impl fmt::Write for FILE {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_all(s.as_bytes()).map_err(|()| fmt::Error)
    }
}

/// A `FILE` in a static, which C reads and assigns through `stdout` and
/// `stderr`.
#[repr(transparent)]
pub struct Stream(UnsafeCell<FILE>);

// SAFETY: C and the kernel reach the stream through a raw pointer; rivet's own
// writes happen under the standard I/O lock, and firmware assigns a stream
// before threads that print start, as `<stdio.h>` says.
unsafe impl Sync for Stream {}

impl Stream {
    pub(crate) const fn new(file: FILE) -> Self {
        Self(UnsafeCell::new(file))
    }

    /// The stream, as C's `stdout` or `stderr` names it.
    pub fn as_ptr(&self) -> *mut FILE {
        self.0.get()
    }
}

/// A formatter sink over a stream, through a small buffer, for `vfprintf`, so a `printf`
/// costs a few writes rather than one per piece. [`flush`] writes what is
/// left.
///
/// [`flush`]: Self::flush
#[cfg(any(target_os = "none", test))]
pub(crate) struct Buffered<'a> {
    file: &'a mut FILE,
    buf: [u8; 64],
    len: usize,
}

#[cfg(any(target_os = "none", test))]
impl<'a> Buffered<'a> {
    pub(crate) fn new(file: &'a mut FILE) -> Self {
        Self { file, buf: [0; 64], len: 0 }
    }

    /// Writes what the buffer holds.
    pub(crate) fn flush(&mut self) -> Result<(), ()> {
        let len = core::mem::take(&mut self.len);
        self.file.write_all(self.buf.get(..len).unwrap_or_default())
    }
}

#[cfg(any(target_os = "none", test))]
impl Sink for Buffered<'_> {
    fn write(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if self.len + bytes.len() > self.buf.len() {
            self.flush()?;
        }
        if bytes.len() > self.buf.len() {
            return self.file.write_all(bytes);
        }
        self.buf[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        Ok(())
    }
}

/// The stream `file` points at, for the length of one call.
///
/// # Safety
///
/// `file` is null or points at a `FILE` that no one else uses for the call,
/// which the standard I/O lock ensures among rivet's functions.
pub(crate) unsafe fn stream<'a>(file: *mut FILE) -> Option<&'a mut FILE> {
    // SAFETY: as the caller guarantees.
    unsafe { file.as_mut() }
}

/// A stream with no callback: every write to it fails.
#[cfg(test)]
pub(crate) const CLOSED: FILE = FILE { write: None, ctx: ptr::null_mut() };

#[cfg(test)]
pub(crate) mod tests {
    use core::fmt::Write as _;

    use super::*;

    /// What a capturing stream has taken.
    pub(crate) struct Captured {
        pub(crate) bytes: [u8; 512],
        pub(crate) len: usize,
        pub(crate) calls: usize,
        /// Fails every call after this many.
        pub(crate) fail_after: usize,
    }

    impl Captured {
        pub(crate) const fn new() -> Self {
            Self { bytes: [0; 512], len: 0, calls: 0, fail_after: usize::MAX }
        }

        pub(crate) fn text(&self) -> &[u8] {
            &self.bytes[..self.len]
        }
    }

    /// A callback that appends to the `Captured` its context points at.
    pub(crate) unsafe extern "C" fn capture(ctx: *mut c_void, s: *const c_char, n: usize) -> c_int {
        // SAFETY: the tests pass a live, unshared `Captured` as the context.
        let captured = unsafe { &mut *ctx.cast::<Captured>() };
        if captured.calls >= captured.fail_after {
            return -1;
        }
        captured.calls += 1;
        // SAFETY: rivet passes `n` readable bytes at `s`.
        let bytes = unsafe { core::slice::from_raw_parts(s.cast::<u8>(), n) };
        captured.bytes[captured.len..captured.len + n].copy_from_slice(bytes);
        captured.len += n;
        n as c_int
    }

    pub(crate) fn capturing(captured: &mut Captured) -> FILE {
        FILE::new(capture, ptr::from_mut(captured).cast())
    }

    #[test]
    fn write_all_calls_the_callback_and_checks_its_count() {
        let mut captured = Captured::new();
        let mut file = capturing(&mut captured);
        assert_eq!(file.write_all(b"abc"), Ok(()));
        assert_eq!(file.write_all(b""), Ok(()));
        assert_eq!(write!(file, "{}-{}", 4, "five"), Ok(()));
        assert_eq!(captured.text(), b"abc4-five");
        captured.fail_after = captured.calls;
        let mut file = capturing(&mut captured);
        assert_eq!(file.write_all(b"x"), Err(()));
        assert!(CLOSED.write.is_none());
        let mut closed = CLOSED;
        assert_eq!(closed.write_all(b"x"), Err(()));
    }

    #[test]
    fn buffered_writes_in_few_calls_and_flushes_the_rest() {
        let mut captured = Captured::new();
        let mut file = capturing(&mut captured);
        let mut out = Buffered::new(&mut file);
        for _ in 0..10 {
            assert_eq!(out.write(b"0123456789"), Ok(()));
        }
        assert_eq!(out.write(&[b'y'; 100]), Ok(()));
        assert_eq!(out.flush(), Ok(()));
        assert_eq!(captured.len, 200);
        assert!(captured.calls <= 4, "{} calls", captured.calls);
        assert_eq!(&captured.text()[..10], b"0123456789");
        assert_eq!(captured.text()[199], b'y');
    }
}
