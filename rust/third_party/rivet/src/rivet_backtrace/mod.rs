// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `rivet_backtrace.h`, with the `backtrace` feature: printing a backtrace as
//! symbolizer markup, and the hooks a kernel gives it. See
//! `support::symbolize` for the design.

#[expect(clippy::module_inception, reason = "the header and its one function share the name")]
mod rivet_backtrace;
mod rivet_backtrace_set_hooks;

pub use rivet_backtrace::rivet_backtrace;
pub use rivet_backtrace_set_hooks::rivet_backtrace_set_hooks;
