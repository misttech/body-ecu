# Third-party code

Prefer `core` and small first-party code over adding a dependency; vendor
external source here only when there is no practical alternative. Rivet
builds with no crate from a registry.

Each vendored project gets its own directory containing:

- the upstream source at a pinned version or revision;
- a `README.md` with the upstream URL, the version or commit vendored,
  and the license;
- any local changes kept separate from upstream files, or listed in the
  `README.md`, with the reason for each.

Do not vendor build output, tests, or files unrelated to the code actually
used.

This directory holds only code rivet builds or links: the projects it
vendors, and upstream tests ported into its test suite, such as newlib's. A
directory here never holds only a license.

Code converted from C or C++ into Rust under `src/` is not listed here. The
Rust file keeps the upstream notice its license requires, and its header
comment names the upstream file and release or revision. Code adapted without
converting it to Rust, such as a C header, and public-domain code are not
listed either; a file whose upstream license requires its notice keeps the
notice itself, after rivet's copyright line.

| Path | Upstream | Revision | License |
|---|---|---|---|
| `third_party/newlib` | https://sourceware.org/newlib/ | 4.5.0; tests ported into `third_party/newlib/test/` | BSD-style, per file (`COPYING.NEWLIB`) |
| `third_party/forkpoint-sdk` | Forkpoint `sdk/rust/forkpoint` | See its `README.md` | Mist Tecnologia LTDA |
| `third_party/zr` | See `third_party/zr/README.md` | See its `README.md` | BSD-3-Clause |
