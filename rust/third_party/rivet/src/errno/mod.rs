// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `errno.h`: `errno`, as the `__errno_location` the header's macro expands
//! to. Its storage and the codes are in `support::errno`.

mod errno_location;

pub use errno_location::__errno_location;
