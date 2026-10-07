# `unsafe` per crate

Counted as `unsafe` blocks, `unsafe impl`, `unsafe fn`, and `unsafe extern` items. Every
one carries a `// SAFETY:` comment.

| Crate | Count | Why |
|---|---|---|
| `ports` | 1 | the console sink's function pointer round-trips through an atomic `usize` |
| `lighting`, `door-lock`, `vehicle-mode`, `ignition`, `speed-simulator`, `can-gateway`, `someip` | 0 | |
| `diagnostics` | 1 | `DtcStore` is `Sync`: read and written one request at a time |
| `ffi` | 32 | the `extern "C"` block and one call per shim function |
| `adapters` | 16 | the adapters and systems are `Sync`, as the C++ objects are shared without locks on the MCU |
| `app` | 7 | the global allocator, the panic handler's call into C, the shared cell of the software-only door lock, and the C entry points |
