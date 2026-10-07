# The Body ECU's MCU firmware in Rust

The port of the MCU side of this repository to Rust: one crate per C++ library under
`libs/`, the application of `app/src/main.cpp`, and the glue to Zephyr, built as a
Zephyr application on the Rust port of OpenBSW and of its Zephyr adaptation
([misttech/openbsw](https://github.com/misttech/openbsw) and
[misttech/openbsw-zephyr](https://github.com/misttech/openbsw-zephyr), branch
`main-mss`), which `west update` checks out under `../openbsw-mss/` beside the upstream
trees the C++ firmware builds on.

| Crate | Ports | Contents |
|---|---|---|
| `ports` | `libs/platform/ports` | the port traits, `SomeIpMessage`, the signal value, the service ids of `config/services.yaml`, the console sink the domain modules print through, and the recording mocks (feature `mock`) |
| `lighting`, `door-lock`, `vehicle-mode`, `ignition`, `speed-simulator` | `libs/body/*` | the domain controllers, with their Forkpoint properties and events |
| `diagnostics` | `libs/platform/diagnostics` | the UDS handler, the DTC store, the transport trait, the DoIP codec |
| `can-gateway` | `libs/platform/can-gateway` | the mappings and the gateway |
| `someip` | the part of `opensomeip` the firmware uses | the 16-byte header and its validation, UDP endpoints |
| `ffi` | `libs/adapters/zephyr`'s Zephyr calls | hand-written `extern "C"` declarations of the shim's `cpp_` functions, with host stubs |
| `adapters` | `libs/adapters/zephyr`, `libs/adapters/system` | the GPIO, button, timer, signal bus, and ADC adapters; the systems as OpenBSW lifecycle components; the SOME/IP server |
| `app` | `app/src/main.cpp` | the application: the statics, their wiring, and `src/zephyr_shim.c`, the one C file |

## No bindings

As in the Rust OpenBSW demo, Rust does not depend on the `zephyr` crate, on `zephyr-sys`,
or on bindgen. `app/src/zephyr_shim.c` owns every kernel object and device (the async
contexts' threads, events and timers, the LEDs, the button, the timer slots, the UDP
socket and the SOME/IP receive thread, the signal bus mutex, the log module) and exports
`cpp_`-prefixed functions that take and return plain scalars; the `ffi` crate declares
them, and `zephyr_ffi` from the OpenBSW port declares the kernel ones. Rust exports the
`rust_`-prefixed callbacks (the button press, a timer slot firing, the receive thread).

## What is the same, and what is not

The domain logic, the systems, and the application are direct translations: the same
state machines, the same console lines, the same SOME/IP responses, byte for byte, which
Forkpoint checks (`forkpoint/README.md`). What differs:

- **SOME/IP** is the part of OpenSOME/IP the firmware uses under emulation: the header
  codec and the UDP server with its polling receive thread. Service discovery, which the
  C++ enables on real silicon only, TP, E2E, and TCP are not ported.
- **Contexts.** The C++ numbers its async contexts from 1 and leaves 0 unused; the Rust
  OpenBSW adapter creates a thread per context, so context 0 gets a thread that only
  waits (`app/src/config.rs`).
- **Allocation.** The C++ uses the standard containers on Zephyr's kernel heap; the Rust
  uses `alloc` on the same heap, through the shim.
- **The blink lines** print the port device's address, which differs between any two
  images; the comparison masks it.

## Building and testing

```sh
cd rust
cargo test --workspace --exclude rustapp
cargo clippy --workspace --exclude rustapp --all-targets -- -D warnings
cargo fmt --check
FPT_HOSTCALL_BASE=0xA0000000 cargo clippy -p rustapp --target thumbv7em-none-eabi --features forkpoint -- -D warnings
```

The firmware builds like the C++ one, through west (`make -C forkpoint build-rust`):
`west build -b nucleo_h753zi rust/app -- -DBODY_ECU_FORKPOINT=ON`, with the Rust target
`thumbv7em-none-eabi` (`rustup target add`). See `PORTING.md` for how the C++ is ported
and `UNSAFE.md` for the `unsafe` count per crate.
