# zr

Zero-dependency `no_std` building blocks from Fuchsia's `src/lib/zr` ("Zircon
Rust Core"), used by rivet's C library and named by the
`cpp-to-rust-rubric` skill.

- Upstream: https://github.com/misttech/fuchsia, path `src/lib/zr`
- Revision: `0703a5c2d237c2c45afddbf4deb6e1cfc2b571ac`
- License: BSD-3-Clause, see `LICENSE`
- Edition: Rust 2024, from the host workspace (`edition.workspace = true`)

Fuchsia's `BUILD.gn` and `OWNERS` are not vendored. This crate is a member of
the host workspace, `src/Cargo.toml`.

## Vendored files

Module sources match that revision. Two tests are written for edition 2024:

- `src/defer.rs` stores flags in a `Cell`. Edition 2024 does not allow a
  closure to mutably borrow a local while the test reads that local.
- `src/ptr.rs` compares the `try_update` pointers with `core::ptr::eq`.
  Clippy rejects `==` on raw pointers.

| File | Provides |
|---|---|
| `src/defer.rs` | `defer`, `Deferred` |
| `src/lossy_utf8.rs` | `from_utf8_lossy` |
| `src/opaque.rs` | `Opaque`, `OpaqueFacade` |
| `src/opaque_bytes.rs` | `OpaqueBytes`, `define_opaque_storage_ffi!` |
| `src/pin_init.rs` | `pin_init_ffi!`, `unsafe_pinned_drop_ffi!` |
| `src/ptr.rs` | `AtomicConstPtr`, `ToMutPtr`, `slice_from_raw_parts`, `slice_from_raw_parts_mut` |
| `src/static_assert.rs` | `static_assert!`, `static_assert_size_and_align!` |
| `src/string.rs` | `parse_usize`, `to_array` |

## Local files

- `src/lib.rs` declares the same modules and re-exports as upstream `lib.rs`.
  It also exports `LossyUtf8`, the type `from_utf8_lossy` returns, which
  upstream leaves private, and keeps local tests for the modules upstream
  does not test.
- `Cargo.toml` makes the crate a member of the host workspace.
- `.cargo/config.toml` keeps a Cargo run started here building into `out/`.

`pin_init_ffi!`, `unsafe_pinned_drop_ffi!`, and `define_opaque_storage_ffi!`
expand to paths in the `pin_init` crate. That crate is not a dependency of
`zr`; the caller that expands the macros must depend on it. Do not add it
until a port uses those macros.

To refresh the vendor, copy every module from one upstream revision and update
the revision above. Do not mix revisions.

## Where rivet uses it

`src/`, the C library, is the only current caller. `__assert_func` formats C strings
with `zr::from_utf8_lossy` because that path is `no_std` and the bytes may not
be UTF-8.

Use the rest when a firmware or SDK port needs the matching C or C++ pattern.
Do not switch host code that already has `std` onto these helpers.

| Item | Use it for | Leave it unused when |
|---|---|---|
| `static_assert!`, `static_assert_size_and_align!` | A `#[repr(C)]` struct shared with C or C++, checked at compile time | The layout is host-only and not an ABI |
| `from_utf8_lossy` | Formatting bytes that may not be UTF-8 in `no_std` code | The caller is on the host and wants an owned `String` (`String::from_utf8_lossy`) |
| `defer` | `fit::defer`, `goto cleanup`, or the same cleanup copied before each early return | A `Drop` impl that owns a resource for its whole lifetime |
| `Opaque`, `OpaqueFacade`, `OpaqueBytes`, `define_opaque_storage_ffi!` | A Rust facade over a C++ object Rust must not read | There is no C++ object to wrap |
| `pin_init_ffi!`, `unsafe_pinned_drop_ffi!` | In-place FFI construct and destroy of a pinned object | Nothing is pinned through FFI. These need the `pin_init` crate at the call site |
| `slice_from_raw_parts`, `slice_from_raw_parts_mut` | An FFI `(pointer, length)` where length 0 may be null | rivet's `mem*` and `str*` functions (`src/string/`), which must not form slices |
| `ToMutPtr` | A reference handed to an FFI destructor as `*mut` | An integer device address. Those use `with_exposed_provenance` |
| `AtomicConstPtr` | An atomic `*const T` shared across threads | A plain `AtomicPtr` already expresses the type |
| `parse_usize` | A decimal `usize` parsed in a `const` context | Parsing that accepts hex or must report an error |
| `to_array` | A string copied into a fixed NUL-terminated array at compile time | A runtime copy |

No first-party `#[repr(C)]` struct, cleanup-on-early-return, opaque C++ object,
or const C string buffer exists yet, so those items have no call site in this
tree.
