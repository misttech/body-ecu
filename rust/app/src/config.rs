// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The application's configuration, ported from `openbswConfig`'s `async/Config.h` and
//! the thread names and stacks of `main.cpp`.
//!
//! The C++ numbers its contexts from 1 and leaves context 0 unused: it defines no task
//! for it. The Rust Zephyr adapter creates a thread per context, so context 0 gets a
//! thread that only waits, on the smallest stack the shim gives it; it never runs.

use core::ffi::CStr;

use openbsw_async::ContextType;

/// The lowest-priority task.
pub const TASK_BACKGROUND: ContextType = 1;
/// The body task.
pub const TASK_BODY: ContextType = 2;
/// The SOME/IP task.
pub const TASK_SOMEIP: ContextType = 3;
/// The diagnostics task.
pub const TASK_DIAG: ContextType = 4;
/// The task the lifecycle transitions run on.
pub const TASK_SYSADMIN: ContextType = 5;
/// The number of contexts, the unused context 0 included (`ASYNC_CONFIG_TASK_COUNT`).
pub const TASK_COUNT: usize = 6;

/// The task names, in context order; context 0 has none in the C++.
pub const TASK_NAMES: [&CStr; TASK_COUNT] =
    [c"unused", c"background", c"body", c"someip", c"diag", c"sysadmin"];

/// The number of interrupt groups (`ISR_GROUP_COUNT`).
pub const ISR_GROUP_COUNT: usize = 1;
/// The interrupt group names, as `main.cpp` names them.
pub const ISR_GROUP_NAMES: [Option<&[u8]>; ISR_GROUP_COUNT] = [Some(b"test")];

#[allow(dead_code)]
const _: () = {
    // Kept for parity with `async/Config.h`; the firmware does not run runnables on
    // these contexts itself.
    let _ = (TASK_BACKGROUND, TASK_BODY, TASK_SOMEIP, TASK_DIAG);
};
