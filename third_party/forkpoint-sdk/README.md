# Forkpoint SDK

The Forkpoint firmware SDK, through which the Body ECU firmware reports properties
(assertions), lifecycle events, and probes to Forkpoint's deterministic virtual MCU when
built with `BODY_ECU_FORKPOINT=ON`. Without that option the C macros compile to nothing
and the Rust crate's calls compile out, so production builds are unchanged.

- Source: [Forkpoint](https://github.com/misttech/forkpoint), `sdk/c/include/forkpoint`
  and `sdk/rust/forkpoint`, at the revision `09f99b0` (SDK last changed in `537ceee`).
- License: Mist Tecnologia LTDA, as the files' headers say.
- Local changes: `rust/forkpoint/Cargo.toml` names this repository's Rust workspace
  instead of Forkpoint's, and the sources are formatted by that workspace's `cargo fmt`.

To refresh it, copy the two headers and the crate's `src/` and `Cargo.toml` from one
Forkpoint revision, keep the local change, and update the revision above.
