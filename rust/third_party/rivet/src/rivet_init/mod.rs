// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `rivet_init.h`: `__libc_init_array`, for startup code that expects the C
//! runtime's. Exported only for a bare-metal target: on the host, the
//! program's own C runtime defines it and the array bounds it reads.

mod libc_init_array;

#[cfg(target_os = "none")]
pub use libc_init_array::__libc_init_array;
