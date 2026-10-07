// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.
//
// Ported from upstream, under this notice:
//
// Copyright 2023 The Fuchsia Authors
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files
// (the "Software"), to deal in the Software without restriction,
// including without limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
// SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Walking a backtrace by frame pointers.
//!
//! Ported from Fuchsia's `zircon/kernel/lib/arch/include/lib/arch/backtrace.h`
//! at https://github.com/misttech/fuchsia revision
//! `0703a5c2d237c2c45afddbf4deb6e1cfc2b571ac`, with these changes:
//!
//! - [`FramePointerBacktrace`] is a Rust iterator. It yields each
//!   [`CallFrame`], as upstream's `WithFp` form does, and [`pcs`] maps it to
//!   the program counters, upstream's default form.
//! - It is built from a frame pointer the caller read, not from the calling
//!   function's own frame address, which Rust has no stable way to take.
//! - The frame-pointer offset covers the targets rivet builds for: 32-bit Arm
//!   and Thumb, where a frame record sits at the frame pointer as on aarch64
//!   and x86-64, and RISC-V, where it sits just below.
//! - Reading a frame record is a volatile read, since it reads memory no Rust
//!   object owns.
//!
//! [`pcs`]: FramePointerBacktrace::pcs

use core::mem::{align_of, size_of};

// Each frame records its caller's FP and PC (return address). A call pushes
// the PC and the prologue then pushes the caller's FP (x86), or the prologue
// pushes the return-address register and PC together (other CPUs). Since the
// stack grows down, the PC is always just after the FP in memory. It then
// sets the FP to point at (or above) the FP, PC pair just pushed. On x86 it's
// unavoidable that the FP is two words below the CFA (SP at call site), since
// the call itself puts the PC there; the FP points directly to the FP, PC pair
// describing the caller. On ARM, the compiler will often place the FP, PC
// pair at the bottom of the new frame instead of the top; the FP points
// directly to the FP, PC pair describing the caller, but there's no guarantee
// where the FP is in relation to the CFA. On RISC-V, the FP is set to the CFA
// (SP at call site / entry); so it points *just past* the FP, PC pair
// describing the caller, but it's also guaranteed to be the CFA.
//
// Frames are ordered by FP address. Since stacks grow down, the least address
// means the innermost callee.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub(crate) struct CallFrame {
    pub(crate) fp: usize,
    pub(crate) pc: usize,
}

zr::static_assert!(size_of::<CallFrame>() == 2 * size_of::<usize>());
zr::static_assert!(align_of::<CallFrame>() == align_of::<usize>());

/// Where the frame record lies from the frame pointer, in records: just below
/// it on RISC-V, at it elsewhere.
const ARCH_FP_OFFSET: isize =
    if cfg!(any(target_arch = "riscv32", target_arch = "riscv64")) { -1 } else { 0 };

impl CallFrame {
    /// On most machines, this is the CFA, which is the SP at the call site and
    /// thus the address that's the upper bound on this call frame. On ARM it's
    /// more likely to be near the bottom of the frame, but will still at least
    /// be an upper bound on the address range of the next frame in.
    pub(crate) fn frame_address(&self) -> usize {
        if self.fp == 0 {
            return 0;
        }
        record_address(self.fp).wrapping_add(size_of::<CallFrame>())
    }
}

/// The address of the record a frame pointer describes.
fn record_address(fp: usize) -> usize {
    fp.wrapping_add_signed(ARCH_FP_OFFSET * size_of::<CallFrame>() as isize)
}

/// A forward iterator over the frames of a backtrace. `is_on_stack` says
/// whether it is safe to read a known-aligned frame record; the walk ends at
/// the first record that is not, or at a null or misaligned frame pointer.
#[derive(Clone)]
pub(crate) struct FramePointerBacktrace<F: Fn(usize) -> bool> {
    frame: CallFrame,
    is_on_stack: F,
}

impl<F: Fn(usize) -> bool> FramePointerBacktrace<F> {
    /// The backtrace from the frame `fp` points at: its first frame is the
    /// record `fp` describes, the frame's caller.
    ///
    /// # Safety
    ///
    /// `is_on_stack` holds only for addresses readable for a whole
    /// [`CallFrame`].
    pub(crate) unsafe fn new(fp: usize, is_on_stack: F) -> Self {
        let mut bt = Self { frame: CallFrame { fp, pc: 0 }, is_on_stack };
        bt.advance();
        bt
    }

    /// The program counters alone, upstream's default form.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "upstream's API; the printer needs each frame pointer")
    )]
    pub(crate) fn pcs(self) -> impl Iterator<Item = usize> {
        self.map(|frame| frame.pc)
    }

    fn advance(&mut self) {
        if self.frame.fp != 0 && self.frame.fp.is_multiple_of(align_of::<CallFrame>()) {
            let record = record_address(self.frame.fp);
            if (self.is_on_stack)(record) {
                let record = core::ptr::with_exposed_provenance::<CallFrame>(record);
                // SAFETY: `record` is aligned, since `fp` is and the offset is
                // whole records, and `is_on_stack` holds for it, which the
                // constructor's caller guarantees makes it readable.
                self.frame = unsafe { record.read_volatile() };
                return;
            }
        }
        self.frame = CallFrame::default();
    }
}

impl<F: Fn(usize) -> bool> Iterator for FramePointerBacktrace<F> {
    type Item = CallFrame;

    fn next(&mut self) -> Option<CallFrame> {
        if self.frame == CallFrame::default() {
            return None;
        }
        let frame = self.frame;
        self.advance();
        Some(frame)
    }
}

/// Stores a backtrace in `pcs`, and returns the number of frames stored. `ra`
/// is the return address of the function that collected the backtrace, if it
/// is known: it is stored first, unless the backtrace already starts with it,
/// as it does when the collecting function has no frame pointer itself.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "upstream's API; the printer prints frames as it walks")
)]
pub(crate) fn store_backtrace(
    bt: impl Iterator<Item = usize>,
    pcs: &mut [usize],
    ra: Option<usize>,
) -> usize {
    if pcs.is_empty() {
        return 0;
    }
    let mut bt = bt.peekable();
    let mut i = 0;
    if let Some(ra) = ra
        && bt.peek().is_none()
    {
        pcs[0] = ra;
        return 1;
    }
    for pc in bt {
        if let Some(ra) = ra
            && i == 0
            && pc != ra
        {
            pcs[i] = ra;
            i += 1;
        }
        if i == pcs.len() {
            break;
        }
        pcs[i] = pc;
        i += 1;
    }
    i
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A fake stack: words that frame records live in, with the frame pointer
    /// of a record as the walker expects to find it.
    pub(crate) struct Stack {
        pub(crate) words: [usize; 32],
    }

    impl Stack {
        pub(crate) fn new() -> Self {
            Self { words: [0; 32] }
        }

        pub(crate) fn base(&self) -> usize {
            self.words.as_ptr().expose_provenance()
        }

        /// The address of word `i`.
        pub(crate) fn at(&self, i: usize) -> usize {
            self.base() + i * size_of::<usize>()
        }

        /// The frame pointer that describes a record at word `i`.
        pub(crate) fn fp_for(&self, i: usize) -> usize {
            self.at(i).wrapping_add_signed(-ARCH_FP_OFFSET * size_of::<CallFrame>() as isize)
        }

        /// Puts a record at word `i`: the caller's frame pointer and pc.
        pub(crate) fn record(&mut self, i: usize, fp: usize, pc: usize) {
            self.words[i] = fp;
            self.words[i + 1] = pc;
        }

        pub(crate) fn contains(&self, addr: usize) -> bool {
            addr >= self.base() && addr + size_of::<CallFrame>() <= self.at(self.words.len())
        }
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "rebuilds frame and stack pointers from integers, which strict provenance cannot follow"
    )]
    fn walks_records_until_a_null_frame_pointer() {
        let mut stack = Stack::new();
        let outer = stack.fp_for(8);
        stack.record(2, stack.fp_for(4), 0x100);
        stack.record(4, outer, 0x200);
        stack.record(8, 0, 0x300);
        // SAFETY: `contains` holds only for words of `stack`.
        let bt = unsafe { FramePointerBacktrace::new(stack.fp_for(2), |a| stack.contains(a)) };
        let mut pcs = [0; 8];
        let n = store_backtrace(bt.pcs(), &mut pcs, None);
        assert_eq!(&pcs[..n], &[0x100, 0x200, 0x300]);
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "rebuilds frame and stack pointers from integers, which strict provenance cannot follow"
    )]
    fn stops_at_an_off_stack_or_misaligned_record() {
        let mut stack = Stack::new();
        stack.record(2, 0x1000, 0x100);
        // SAFETY: as above.
        let bt = unsafe { FramePointerBacktrace::new(stack.fp_for(2), |a| stack.contains(a)) };
        assert_eq!(bt.pcs().count(), 1);
        stack.record(2, stack.fp_for(4) + 1, 0x100);
        // SAFETY: as above.
        let bt = unsafe { FramePointerBacktrace::new(stack.fp_for(2), |a| stack.contains(a)) };
        assert_eq!(bt.pcs().count(), 1);
        // SAFETY: as above.
        let bt = unsafe { FramePointerBacktrace::new(0, |a| stack.contains(a)) };
        assert_eq!(bt.count(), 0);
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "rebuilds frame and stack pointers from integers, which strict provenance cannot follow"
    )]
    fn frames_carry_their_frame_pointer_and_address() {
        let mut stack = Stack::new();
        stack.record(2, stack.fp_for(4), 0x100);
        stack.record(4, 0, 0x200);
        // SAFETY: as above.
        let mut bt = unsafe { FramePointerBacktrace::new(stack.fp_for(2), |a| stack.contains(a)) };
        let first = bt.next().unwrap();
        assert_eq!((first.fp, first.pc), (stack.fp_for(4), 0x100));
        assert_eq!(first.frame_address(), stack.at(4) + size_of::<CallFrame>());
        assert_eq!(CallFrame::default().frame_address(), 0);
    }

    #[test]
    fn store_backtrace_puts_the_return_address_first_once() {
        let mut pcs = [0; 4];
        assert_eq!(store_backtrace([].into_iter(), &mut pcs, Some(0x9)), 1);
        assert_eq!(pcs[0], 0x9);
        let n = store_backtrace([0x1, 0x2].into_iter(), &mut pcs, Some(0x9));
        assert_eq!(&pcs[..n], &[0x9, 0x1, 0x2]);
        let n = store_backtrace([0x9, 0x2].into_iter(), &mut pcs, Some(0x9));
        assert_eq!(&pcs[..n], &[0x9, 0x2]);
        let n = store_backtrace([0x1, 0x2, 0x3, 0x4, 0x5].into_iter(), &mut pcs, None);
        assert_eq!(&pcs[..n], &[0x1, 0x2, 0x3, 0x4]);
        assert_eq!(store_backtrace([0x1].into_iter(), &mut [], None), 0);
    }
}
