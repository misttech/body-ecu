// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Zephyr's log, as `main.cpp` uses it: `LOG_INF` and `LOG_WRN` on the `body_ecu`
//! module, through the shim, which registers the module. A line is formatted here and
//! handed over whole.

use core::fmt::{self, Write};

/// The longest line logged; `main.cpp`'s are well under it.
const LINE_MAX: usize = 160;

struct Line {
    bytes: [u8; LINE_MAX],
    len: usize,
}

impl Write for Line {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let take = s.len().min(LINE_MAX - self.len);
        self.bytes[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}

fn format(args: fmt::Arguments<'_>) -> Line {
    let mut line = Line { bytes: [0; LINE_MAX], len: 0 };
    // Writing cannot fail: a line longer than the buffer is cut.
    let _ = line.write_fmt(args);
    line
}

/// Log `args` at INFO.
pub fn info(args: fmt::Arguments<'_>) {
    let line = format(args);
    body_ecu_ffi::log_inf(&line.bytes[..line.len]);
}

/// Log `args` at WARNING.
pub fn warning(args: fmt::Arguments<'_>) {
    let line = format(args);
    body_ecu_ffi::log_wrn(&line.bytes[..line.len]);
}

/// Bytes shown as text, as `%s` shows a C string.
pub struct Text<'a>(pub &'a [u8]);

impl fmt::Display for Text<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in self.0.utf8_chunks() {
            f.write_str(chunk.valid())?;
            if !chunk.invalid().is_empty() {
                f.write_char(char::REPLACEMENT_CHARACTER)?;
            }
        }
        Ok(())
    }
}

/// `LOG_INF`.
#[macro_export]
macro_rules! log_inf {
    ($($arg:tt)*) => {
        $crate::log::info(core::format_args!($($arg)*))
    };
}

/// `LOG_WRN`.
#[macro_export]
macro_rules! log_wrn {
    ($($arg:tt)*) => {
        $crate::log::warning(core::format_args!($($arg)*))
    };
}
