# rivet

Rivet, the C library for embedded firmware written in Rust, which the Rust Body ECU
links as Zephyr's C library (`CONFIG_EXTERNAL_LIBC`, through `forkpoint/rivet_module`)
in place of newlib. Its own `README.md` is beside this file.

- Source: https://github.com/misttech/rivet, revision `d715d540b22c72dbf8b548a20d410e58d0a10511` (`main`):
  `include/`, `src/`, `build/` (what its build script includes), and its `third_party/`
  (`zr` and its copy of the Forkpoint SDK, which it builds with, and the licenses of the
  code it ports); not its tests or docs.
- License: Mist Tecnologia LTDA, and the licenses under `third_party/`.
- Local changes: `src/Cargo.toml` leaves the `../test` workspace member out, since the
  tests are not vendored; `src/.cargo/config.toml`, which named rivet's own output
  directory, is left out.

Change the library in rivet first, with its tests, then copy it here and update the
revision above. `forkpoint/Makefile` builds it with Cargo for the board's Rust target.
