// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Printing a backtrace as symbolizer markup, with the `backtrace` feature.
//!
//! A backtrace starts from the state of the function that asked for it: its
//! return address, its frame pointer, and its stack pointer, which the naked
//! entry points of `abort`, `__assert_func`, and `rivet_backtrace` read before
//! anything else runs ([`capture_entry`]). From there:
//!
//! 1. The return address is frame 0: the call site in the caller.
//! 2. Frame records are walked from the caller's frame pointer while each is
//!    plausible: on the stack, at a higher address than the one before, and
//!    returning into the code range. Code built without frame pointers, or by
//!    GCC for Thumb, whose frame pointer is not at its frame record, ends the
//!    walk early.
//! 3. If the walk ended early, the stack is scanned upward, from just above the
//!    last record the walk took (or the stack pointer), for words that point
//!    into the code range, listed as candidates: some are return addresses the
//!    walk missed, some are only data.
//!
//! The kernel supplies what rivet cannot know through `rivet_backtrace.h`: which
//! addresses are on the running thread's stack, the image's GNU build ID note,
//! and its code range. Without them, only frame 0 is printed.

use core::ffi::{c_char, c_int, c_void};
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use crate::support::backtrace::{CallFrame, FramePointerBacktrace};
use crate::support::markup::{LineBuffered, MemoryPermissions, Sink, Writer};

/// The calls and facts a kernel gives rivet's backtraces, registered with
/// `rivet_backtrace_set_hooks`. Each may be null or zero.
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct rivet_backtrace_hooks {
    /// Whether the word at `addr` lies in the running thread's stack. Without
    /// it, no frame record or stack word is read.
    pub is_on_stack: Option<unsafe extern "C" fn(addr: *const c_void) -> c_int>,
    /// The image's GNU build ID note, as the linker emits it in
    /// `.note.gnu.build-id`, and its size in bytes.
    pub build_id_note: *const c_void,
    pub build_id_note_size: usize,
    /// The image's code, as loaded, and its size in bytes.
    pub code_start: *const c_void,
    pub code_size: usize,
    /// The module's name in the markup; null for `firmware`.
    pub module_name: *const c_char,
}

zr::static_assert!(size_of::<rivet_backtrace_hooks>() == 6 * size_of::<usize>());
zr::static_assert!(align_of::<rivet_backtrace_hooks>() == align_of::<usize>());

static HOOKS: AtomicPtr<rivet_backtrace_hooks> = AtomicPtr::new(ptr::null_mut());

/// Registers the kernel's hooks, replacing any registered before. A backtrace
/// already printing may still use the hooks it started with, so hooks that
/// are replaced must stay valid too, as a `&'static` does.
pub(crate) fn set_hooks(hooks: &'static rivet_backtrace_hooks) {
    HOOKS.store(ptr::from_ref(hooks).cast_mut(), Ordering::Release);
}

fn hooks() -> Option<&'static rivet_backtrace_hooks> {
    // SAFETY: `HOOKS` is null or was stored from a `&'static`.
    unsafe { HOOKS.load(Ordering::Acquire).as_ref() }
}

/// The state of the function that asked for a backtrace, read on entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct Capture {
    /// Its return address: the call site in its caller. 0 if unknown.
    pub(crate) pc: usize,
    /// Its caller's frame pointer.
    pub(crate) fp: usize,
    /// Its stack pointer.
    pub(crate) sp: usize,
}

/// Defines `$name`, a naked `extern "C" fn() -> !` or `fn()`, whose first
/// instructions pass its return address, its caller's frame pointer, and its
/// stack pointer to `$body`, an `extern "C" fn(usize, usize, usize)`, which it
/// jumps to. Nothing runs before the registers are read, so frame 0 is the
/// exact call site, and the walk starts from the caller's own frame record,
/// whether or not rivet was built with frame pointers.
#[cfg(all(target_os = "none", any(target_arch = "arm", target_arch = "riscv32")))]
macro_rules! capture_entry {
    ($(#[$attr:meta])* $vis:vis fn $name:ident() $(-> $ret:ty)? => $body:path) => {
        $(#[$attr])*
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        $vis extern "C" fn $name() $(-> $ret)? {
            #[cfg(target_arch = "arm")]
            core::arch::naked_asm!("mov r0, lr", "mov r1, r7", "mov r2, sp", "b {body}", body = sym $body);
            #[cfg(target_arch = "riscv32")]
            core::arch::naked_asm!("mv a0, ra", "mv a1, s0", "mv a2, sp", "tail {body}", body = sym $body);
        }
    };
}
#[cfg(all(target_os = "none", any(target_arch = "arm", target_arch = "riscv32")))]
pub(crate) use capture_entry;

/// The state of the calling function, for a backtrace that starts inside
/// rivet, such as a panic's: its frame pointer and stack pointer, but no
/// return address. Frames inside rivet are walked only if rivet was built with
/// frame pointers.
#[inline(always)]
pub(crate) fn here() -> Capture {
    #[allow(unused_mut, unused_assignments)]
    let (mut fp, mut sp) = (0usize, 0usize);
    #[cfg(all(target_os = "none", target_arch = "arm"))]
    // SAFETY: reads two registers into two outputs, and touches nothing else.
    unsafe {
        core::arch::asm!("mov {}, r7", "mov {}, sp", out(reg) fp, out(reg) sp, options(nomem, nostack, preserves_flags));
    }
    #[cfg(all(target_os = "none", target_arch = "riscv32"))]
    // SAFETY: as above.
    unsafe {
        core::arch::asm!("mv {}, s0", "mv {}, sp", out(reg) fp, out(reg) sp, options(nomem, nostack, preserves_flags));
    }
    Capture { pc: 0, fp, sp }
}

/// The most frames a backtrace prints, from the walk and the scan each.
const MAX_FRAMES: u32 = 32;

/// The most stack words a scan reads.
const MAX_SCAN_WORDS: usize = 4096;

/// What rivet knows of the image, from the hooks.
struct Image<'a> {
    is_on_stack: Option<unsafe extern "C" fn(*const c_void) -> c_int>,
    code: core::ops::Range<usize>,
    build_id: Option<&'a [u8]>,
    name: &'a str,
}

impl Image<'_> {
    fn from_hooks(hooks: Option<&'static rivet_backtrace_hooks>) -> Image<'static> {
        let Some(hooks) = hooks else {
            return Image { is_on_stack: None, code: 0..0, build_id: None, name: "firmware" };
        };
        let start = hooks.code_start.addr();
        let note = if hooks.build_id_note.is_null() || hooks.build_id_note_size == 0 {
            None
        } else {
            // SAFETY: the hooks name a note readable for its size, which lives
            // as long as the program, per `rivet_backtrace_set_hooks`.
            Some(unsafe {
                core::slice::from_raw_parts(
                    hooks.build_id_note.cast::<u8>(),
                    hooks.build_id_note_size,
                )
            })
        };
        let name = if hooks.module_name.is_null() {
            "firmware"
        } else {
            // SAFETY: a non-null name is NUL-terminated and lives as long as
            // the program, per `rivet_backtrace_set_hooks`.
            core::str::from_utf8(unsafe { core::ffi::CStr::from_ptr(hooks.module_name) }.to_bytes())
                .unwrap_or("firmware")
        };
        Image {
            is_on_stack: hooks.is_on_stack,
            code: start..start.saturating_add(hooks.code_size),
            build_id: note.and_then(gnu_build_id),
            name,
        }
    }

    fn on_stack(&self, addr: usize) -> bool {
        // SAFETY: the kernel's hook takes any address.
        self.is_on_stack.is_some_and(|f| unsafe { f(core::ptr::without_provenance(addr)) } != 0)
    }

    /// Whether the whole frame record at `addr`, both its words, lies on the
    /// stack.
    fn on_stack_record(&self, addr: usize) -> bool {
        let second = addr.wrapping_add(core::mem::size_of::<usize>());
        self.on_stack(addr) && second > addr && self.on_stack(second)
    }

    /// Whether `pc` returns into the image's code; any value when the range is
    /// unknown.
    fn is_code(&self, pc: usize) -> bool {
        self.code.is_empty() || self.code.contains(&code_address(pc))
    }
}

/// `pc` as the symbolizer should see it. On Arm a return address carries the
/// Thumb bit, which would make "the return address minus one", where the
/// symbolizer looks, fall after the call rather than in it.
fn code_address(pc: usize) -> usize {
    if cfg!(target_arch = "arm") { pc & !1 } else { pc }
}

/// The descriptor of a GNU build ID note: its header, `namesz`, `descsz`, and
/// `type` (3), then the name `GNU\0`, then the descriptor, each padded to 4
/// bytes. `None` for anything else.
fn gnu_build_id(note: &[u8]) -> Option<&[u8]> {
    const NT_GNU_BUILD_ID: u32 = 3;
    let word = |i: usize| note.get(i..i + 4).map(|b| u32::from_ne_bytes([b[0], b[1], b[2], b[3]]));
    let (namesz, descsz, kind) = (word(0)? as usize, word(4)? as usize, word(8)?);
    let name = note.get(12..12 + namesz)?;
    if kind != NT_GNU_BUILD_ID || name != b"GNU\0" {
        return None;
    }
    let desc = 12 + namesz.next_multiple_of(4);
    note.get(desc..desc + descsz).filter(|d| !d.is_empty())
}

/// Set while a backtrace prints, so that a fault or panic while printing
/// does not print another.
static PRINTING: AtomicBool = AtomicBool::new(false);

/// Prints the backtrace from `capture` to `out`, with the context lines that
/// let a symbolizer find the image. Does nothing if a backtrace is already
/// printing.
pub(crate) fn print(capture: Capture, out: impl Sink) {
    if PRINTING.swap(true, Ordering::Acquire) {
        return;
    }
    let image = Image::from_hooks(hooks());
    print_with(&image, capture, out);
    PRINTING.store(false, Ordering::Release);
}

fn print_with(image: &Image<'_>, capture: Capture, out: impl Sink) {
    let w = Writer::new(LineBuffered::<128, _>::new(out));
    w.reset().newline();
    if let Some(build_id) = image.build_id {
        w.elf_module(0, image.name, build_id).newline();
        if !image.code.is_empty() {
            let rx = MemoryPermissions { read: true, write: false, execute: true };
            let start = image.code.start as u64;
            w.load_image_mmap(start, image.code.len() as u64, 0, rx, start).newline();
        }
    }
    let mut n = 0;
    if capture.pc != 0 {
        w.return_address_frame(n, code_address(capture.pc) as u64).newline();
        n += 1;
    }
    let walked = walk(image, capture, |pc| {
        if n < MAX_FRAMES {
            w.return_address_frame(n, code_address(pc) as u64).newline();
            n += 1;
        }
    });
    if let Err(scan_from) = walked
        && image.is_on_stack.is_some()
        && !image.code.is_empty()
    {
        #[cfg(feature = "forkpoint-coverage")]
        forkpoint::assert_sometimes!(true, "backtrace: scans the stack past the frame chain");
        let mut found = 0;
        scan(image, scan_from, |pc| {
            if found < MAX_FRAMES {
                w.literal("rivet: stack scan ")
                    .return_address_frame(n, code_address(pc) as u64)
                    .newline();
                n += 1;
                found += 1;
            }
        });
    }
}

/// Walks the frame records from `capture.fp` while each is plausible, calling
/// `frame` with each return address. Returns `Ok` if the walk reached the
/// outermost frame, a record with a null frame pointer, or else where a stack
/// scan should start: just above the last record it took, or the stack
/// pointer if it took none.
fn walk(image: &Image<'_>, capture: Capture, mut frame: impl FnMut(usize)) -> Result<(), usize> {
    if image.is_on_stack.is_none() || capture.fp == 0 {
        return Err(capture.sp);
    }
    // SAFETY: the kernel's hook holds only for words of the running thread's
    // stack, and a record is two words, so the walker reads one only when the
    // hook holds for both: a record starting at the stack's last word does not
    // pass.
    let records = unsafe { FramePointerBacktrace::new(capture.fp, |a| image.on_stack_record(a)) };
    let mut fp = capture.fp;
    let mut scan_from = capture.sp;
    for record in records {
        // Records lie at increasing addresses toward the outermost frame, and
        // each returns into code: anything else is a frame pointer that does
        // not point at a record.
        if !image.is_code(record.pc) || (record.fp != 0 && record.fp <= fp) {
            return Err(scan_from);
        }
        frame(record.pc);
        if record.fp == 0 {
            return Ok(());
        }
        // The record just taken, read at `fp`, ends where a scan would start.
        scan_from = CallFrame { fp, pc: 0 }.frame_address();
        fp = record.fp;
    }
    Err(scan_from)
}

/// Calls `found` with each word on the stack, upward from `sp`, that points
/// into the code range: on Arm, with the Thumb bit set, as return addresses
/// are.
fn scan(image: &Image<'_>, sp: usize, mut found: impl FnMut(usize)) {
    let word = core::mem::size_of::<usize>();
    let mut addr = sp.next_multiple_of(word);
    for _ in 0..MAX_SCAN_WORDS {
        if !image.on_stack(addr) {
            return;
        }
        let ptr = core::ptr::with_exposed_provenance::<usize>(addr);
        // SAFETY: `addr` is aligned, and on the stack per the kernel's hook.
        let value = unsafe { ptr.read_volatile() };
        let thumb_ok = !cfg!(target_arch = "arm") || value & 1 == 1;
        if thumb_ok && !image.code.is_empty() && image.code.contains(&code_address(value)) {
            found(value);
        }
        addr = addr.wrapping_add(word);
    }
}

#[cfg(test)]
mod tests {
    use core::mem::size_of;
    use core::sync::atomic::AtomicUsize;

    use super::*;
    use crate::support::backtrace::tests::Stack;

    /// The words of the fake stack `on_fake_stack` checks against.
    static FAKE_BASE: AtomicUsize = AtomicUsize::new(0);
    static FAKE_LEN: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn on_fake_stack(addr: *const c_void) -> c_int {
        let (base, len) = (FAKE_BASE.load(Ordering::Relaxed), FAKE_LEN.load(Ordering::Relaxed));
        c_int::from(addr.addr() >= base && addr.addr() + size_of::<usize>() <= base + len)
    }

    /// Code addresses as a return address carries them on this target.
    fn ra(addr: usize) -> usize {
        if cfg!(target_arch = "arm") { addr | 1 } else { addr }
    }

    struct Text {
        buf: [u8; 2048],
        len: usize,
    }

    impl Sink for &mut Text {
        fn write(&mut self, s: &str) {
            self.buf[self.len..self.len + s.len()].copy_from_slice(s.as_bytes());
            self.len += s.len();
        }
    }

    fn printed(image: &Image<'_>, capture: Capture) -> Text {
        let mut text = Text { buf: [0; 2048], len: 0 };
        print_with(image, capture, &mut text);
        text
    }

    fn lines(text: &Text) -> impl Iterator<Item = &str> {
        core::str::from_utf8(&text.buf[..text.len]).unwrap().lines()
    }

    // Tests share the fake-stack statics, so each holds this lock.
    static SERIAL: AtomicBool = AtomicBool::new(false);
    struct Serial;
    impl Drop for Serial {
        fn drop(&mut self) {
            SERIAL.store(false, Ordering::Release);
        }
    }
    fn serial(stack: &Stack) -> Serial {
        while SERIAL.swap(true, Ordering::Acquire) {
            core::hint::spin_loop();
        }
        FAKE_BASE.store(stack.base(), Ordering::Relaxed);
        FAKE_LEN.store(stack.words.len() * size_of::<usize>(), Ordering::Relaxed);
        Serial
    }

    fn image(code: core::ops::Range<usize>) -> Image<'static> {
        Image { is_on_stack: Some(on_fake_stack), code, build_id: None, name: "firmware" }
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "rebuilds frame and stack pointers from integers, which strict provenance cannot follow"
    )]
    fn a_whole_chain_prints_every_frame_and_no_scan() {
        let mut stack = Stack::new();
        let _serial = serial(&stack);
        stack.record(4, stack.fp_for(8), ra(0x1100));
        stack.record(8, 0, ra(0x1200));
        let capture = Capture { pc: ra(0x1010), fp: stack.fp_for(4), sp: stack.at(0) };
        let text = printed(&image(0x1000..0x2000), capture);
        let first: [&str; 4] = core::array::from_fn(|i| lines(&text).nth(i).unwrap_or(""));
        assert_eq!(first[0], "{{{reset}}}");
        assert_eq!(first[1], "{{{bt:0:0x1010:ra}}}");
        assert_eq!(first[2], "{{{bt:1:0x1100:ra}}}");
        assert_eq!(first[3], "{{{bt:2:0x1200:ra}}}");
        assert_eq!(lines(&text).count(), 4);
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "rebuilds frame and stack pointers from integers, which strict provenance cannot follow"
    )]
    fn a_broken_chain_stops_and_the_scan_lists_code_words() {
        let mut stack = Stack::new();
        let _serial = serial(&stack);
        // A GCC-style frame pointer: it points at locals, which hold a lower
        // "frame pointer" and a data word, not at the record above them.
        stack.words[2] = stack.at(1);
        stack.words[3] = 0x7777;
        stack.words[10] = ra(0x1300);
        stack.words[12] = 0x1301 ^ usize::from(cfg!(target_arch = "arm"));
        let capture = Capture { pc: ra(0x1010), fp: stack.fp_for(2), sp: stack.at(0) };
        let text = printed(&image(0x1000..0x2000), capture);
        let scanned: [&str; 2] = core::array::from_fn(|i| {
            lines(&text).filter(|l| l.starts_with("rivet: stack scan")).nth(i).unwrap_or("")
        });
        assert_eq!(lines(&text).nth(1), Some("{{{bt:0:0x1010:ra}}}"));
        assert_eq!(scanned[0], "rivet: stack scan {{{bt:1:0x1300:ra}}}");
        if cfg!(target_arch = "arm") {
            // On Arm a word without the Thumb bit is not a return address.
            assert_eq!(scanned[1], "");
        } else {
            assert_eq!(scanned[1], "rivet: stack scan {{{bt:2:0x1301:ra}}}");
        }
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "rebuilds frame and stack pointers from integers, which strict provenance cannot follow"
    )]
    fn the_scan_starts_above_the_last_record_the_walk_took() {
        let mut stack = Stack::new();
        let _serial = serial(&stack);
        // One good record at word 4, whose next frame pointer leads to a
        // record returning outside the code; a code word at word 2, below the
        // good record, and one at word 12, above it.
        stack.record(4, stack.fp_for(8), ra(0x1100));
        stack.record(8, 0, 0x9_0000);
        stack.words[2] = ra(0x1400);
        stack.words[12] = ra(0x1500);
        let capture = Capture { pc: 0, fp: stack.fp_for(4), sp: stack.at(0) };
        let image = image(0x1000..0x2000);
        let mut frames = 0;
        let scan_from = walk(&image, capture, |_| frames += 1).unwrap_err();
        assert_eq!((frames, scan_from), (1, stack.at(6)));
        let mut found = [0; 4];
        let mut n = 0;
        scan(&image, scan_from, |pc| {
            found[n] = pc;
            n += 1;
        });
        assert_eq!(&found[..n], &[ra(0x1500)]);
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "rebuilds frame and stack pointers from integers, which strict provenance cannot follow"
    )]
    fn a_record_whose_second_word_is_off_the_stack_is_not_read() {
        let mut stack = Stack::new();
        let _serial = serial(&stack);
        // The hook holds for the last word but not the one after it, so a
        // record there would read past the stack.
        let last = stack.words.len() - 1;
        stack.words[last] = 0;
        let capture = Capture { pc: 0, fp: stack.fp_for(last), sp: stack.at(0) };
        let image = image(0x1000..0x2000);
        assert!(image.on_stack(stack.at(last)) && !image.on_stack(stack.at(last + 1)));
        assert!(!image.on_stack_record(stack.at(last)));
        let mut frames = 0;
        assert!(walk(&image, capture, |_| frames += 1).is_err());
        assert_eq!(frames, 0);
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "rebuilds frame and stack pointers from integers, which strict provenance cannot follow"
    )]
    fn a_return_address_outside_the_code_stops_the_walk() {
        let mut stack = Stack::new();
        let _serial = serial(&stack);
        stack.record(4, stack.fp_for(8), 0x9_0000);
        stack.record(8, 0, ra(0x1200));
        let capture = Capture { pc: 0, fp: stack.fp_for(4), sp: stack.at(0) };
        let image = image(0x1000..0x2000);
        let mut frames = 0;
        assert!(walk(&image, capture, |_| frames += 1).is_err());
        assert_eq!(frames, 0);
    }

    #[test]
    fn without_hooks_only_the_caller_prints() {
        let image = Image::from_hooks(None);
        let capture = Capture { pc: ra(0x1010), fp: 0x40, sp: 0x80 };
        let text = printed(&image, capture);
        assert_eq!(lines(&text).next(), Some("{{{reset}}}"));
        assert_eq!(lines(&text).nth(1), Some("{{{bt:0:0x1010:ra}}}"));
        assert_eq!(lines(&text).count(), 2);
    }

    #[test]
    fn the_build_id_note_and_code_range_give_the_context_lines() {
        let mut note = [0u8; 12 + 4 + 4];
        note[0..4].copy_from_slice(&4u32.to_ne_bytes());
        note[4..8].copy_from_slice(&4u32.to_ne_bytes());
        note[8..12].copy_from_slice(&3u32.to_ne_bytes());
        note[12..16].copy_from_slice(b"GNU\0");
        note[16..20].copy_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(gnu_build_id(&note), Some(&[0xde, 0xad, 0xbe, 0xef][..]));
        let image = Image {
            is_on_stack: None,
            code: 0x1000..0x3000,
            build_id: gnu_build_id(&note),
            name: "app",
        };
        let text = printed(&image, Capture::default());
        let first: [&str; 3] = core::array::from_fn(|i| lines(&text).nth(i).unwrap_or(""));
        assert_eq!(
            first,
            [
                "{{{reset}}}",
                "{{{module:0:app:elf:deadbeef}}}",
                "{{{mmap:0x1000:0x2000:load:0:rx:0x1000}}}"
            ]
        );
        // Not a GNU build ID: another type, another name, or cut short.
        let mut other = note;
        other[8] = 1;
        assert_eq!(gnu_build_id(&other), None);
        other = note;
        other[12] = b'X';
        assert_eq!(gnu_build_id(&other), None);
        assert_eq!(gnu_build_id(&note[..18]), None);
    }
}
