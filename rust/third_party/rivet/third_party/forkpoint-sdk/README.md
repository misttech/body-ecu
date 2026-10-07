# Forkpoint SDK

The Forkpoint firmware SDK's Rust crate, `forkpoint`, through which rivet
reports properties (assertions) to Forkpoint's emulator when built with its
`forkpoint` feature. Without that feature rivet does not depend on it.

- Source: Forkpoint, `sdk/rust/forkpoint`, revision
  `8bfdc244dabf0981dbbfa48551e040d93fafbff3`.
- License: Mist Tecnologia LTDA, as the rest of rivet.
- Local changes: `Cargo.toml` names rivet's workspace, `../../src`, and calls
  the package `rivet-forkpoint-sdk`, so firmware that also depends on
  Forkpoint's own copy has two distinct packages rather than two named
  `forkpoint`. Rivet still uses it as `forkpoint`.

To refresh it, copy `src/` and `Cargo.toml` from one Forkpoint revision, keep
the local changes, and update the revision above.
