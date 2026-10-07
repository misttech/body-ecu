// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Where the domain modules' `printk` lines go. The C++ prints them with Zephyr's
//! `printk` on the target and `std::printf` elsewhere; here the application installs a
//! sink at startup, and a host test installs none, so the lines are dropped.

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicUsize, Ordering};

/// A sink: takes one formatted line.
pub type Sink = fn(&[u8]);

/// The installed sink, as a function pointer's address; 0 is none.
static SINK: AtomicUsize = AtomicUsize::new(0);

/// Send every line printed from now on to `sink`.
pub fn set_sink(sink: Sink) {
    SINK.store(sink as usize, Ordering::Release);
}

/// The longest line a `printk` formats; the C++ lines are well under it.
const LINE_MAX: usize = 160;

struct Line {
    bytes: [u8; LINE_MAX],
    len: usize,
}

impl Write for Line {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let room = LINE_MAX - self.len;
        let take = s.len().min(room);
        self.bytes[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}

/// Format `args` and hand the line to the sink, if one is installed.
pub fn printk(args: fmt::Arguments<'_>) {
    let address = SINK.load(Ordering::Acquire);
    if address == 0 {
        return;
    }
    let mut line = Line { bytes: [0; LINE_MAX], len: 0 };
    // Writing cannot fail: a line longer than the buffer is cut.
    let _ = line.write_fmt(args);
    // SAFETY: the address was stored by `set_sink` from a `Sink`, and a function pointer
    // round-trips through usize.
    let sink: Sink = unsafe { core::mem::transmute::<usize, Sink>(address) };
    sink(&line.bytes[..line.len]);
}

/// `printk!("...", args)`: the C++ `printk` of the domain modules.
#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => {
        $crate::console::printk(core::format_args!($($arg)*))
    };
}
