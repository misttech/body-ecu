// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `rivet_errno.h`: where a kernel tells rivet the running thread's `errno`
//! lives. See `support::errno` for the design.

mod rivet_errno_set_hooks;

pub use rivet_errno_set_hooks::rivet_errno_set_hooks;
