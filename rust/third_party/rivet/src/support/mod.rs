// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Internal helpers the C functions share. Nothing here is exported to C.

// The allocator's unit tests run in every host test build, with or without the
// feature that exports it.
#[cfg(any(feature = "cmpctmalloc", test))]
#[cfg_attr(not(feature = "cmpctmalloc"), allow(dead_code))]
pub(crate) mod cmpctmalloc;
// The backtrace's markup writer and frame walker: their unit tests run in every
// host test build, with or without the feature that prints backtraces.
#[cfg(any(feature = "backtrace", test))]
#[cfg_attr(not(feature = "backtrace"), allow(dead_code))]
pub(crate) mod backtrace;
pub(crate) mod cstr;
pub(crate) mod ctype_utils;
pub(crate) mod errno;
pub(crate) mod file;
// The formatter serves the C entry points, which only a bare-metal target has;
// its unit tests run in every host test build.
#[cfg(any(target_os = "none", test))]
#[cfg_attr(not(target_os = "none"), allow(dead_code))]
pub(crate) mod format;
#[cfg(any(feature = "backtrace", test))]
#[cfg_attr(not(feature = "backtrace"), allow(dead_code))]
pub(crate) mod markup;
pub(crate) mod mem_ops;
pub(crate) mod os_util;
#[cfg(feature = "forkpoint")]
pub(crate) mod property;
pub(crate) mod smoothsort;
pub(crate) mod stdout_lock;
pub(crate) mod strto;
#[cfg(any(feature = "backtrace", test))]
#[cfg_attr(not(feature = "backtrace"), allow(dead_code, unused_imports))]
pub(crate) mod symbolize;
#[cfg(target_os = "none")]
pub(crate) mod va_list;
