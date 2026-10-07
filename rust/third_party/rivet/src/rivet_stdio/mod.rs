// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `rivet_stdio.h`: how a kernel gives rivet's streams a lock. See
//! `support::stdout_lock` for the design.

mod rivet_stdio_set_hooks;

pub use rivet_stdio_set_hooks::rivet_stdio_set_hooks;
