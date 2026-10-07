# Set 4: the Rust port with rivet as its C library

The Rust image is built a second time with [rivet](../../rust/third_party/rivet/VENDOR.md),
the C library written in Rust, as Zephyr's C library (`CONFIG_EXTERNAL_LIBC`,
`forkpoint/rivet.conf`, `forkpoint/rivet_module/`), and run through the same scenarios
as the two images of Set 3, against each of them: `fpt equiv` on every Ethernet frame
and how the runs end, `compare_consoles.py` on the consoles, `equiv.sh` on the property
verdicts. `make equiv-rivet` reproduces it; `make report-rivet` wrote this file.

## Verdict

| Scenario | rivet against C++ | rivet against Rust on Zephyr's C library | Consoles |
|---|---|---|---|
| boot alone, 5 s | equivalent: 37 console lines, 6 frames (6 sent, 0 received) | equivalent: 37 console lines, 6 frames (6 sent, 0 received) | match |
| the scenario (SOME/IP tester, button), 5 s | equivalent: 136 console lines, 38 frames (25 sent, 13 received) | equivalent: 136 console lines, 38 frames (25 sent, 13 received) | match |

Changing the C library under the Rust firmware changes nothing an observer outside the
chip can see: the same frames, byte for byte, the same console lines, the same property
verdicts.

## What each image links as its C library

From the linker maps (`newlib_free.sh` makes the same check for the rivet image).
libgcc, the compiler's arithmetic helpers (soft-float, 64-bit division, popcount), is
linked by every image: the C++ through Zephyr, the Rust ones because
`openbsw_rust_application()` links it ahead of the Rust archive so that Zephyr's C and
the Rust code share one set of helpers.

| Archive members taken | C++ (newlib) | Rust (Zephyr's minimal C library) | Rust (rivet) |
|---|---|---|---|
| newlib (`libc.a`, `libc_nano.a`, `libm.a`) | 183 | 0 | 0 |
| Zephyr's own (`lib/libc/minimal`, `lib/libc/common`) | 2 | 26 | 0 |
| rivet (`librivet_libc.a`) | 0 | 0 | 8 |
| libgcc | 25 | 18 | 18 |

The functions the rivet image takes from rivet (`.text` sections from its archive):

`__aeabi_memclr`, `__aeabi_memcpy`, `__aeabi_memmove`, `__errno_location`, `isprint`, `isupper`, `memcmp`, `memcpy`, `memmove`, `memset`, `rivet_errno_set_hooks`, `rivet_stdio_set_hooks`, `strchr`, `strlen`, `strncmp`, `strnlen`, `strrchr`, `strtol`.

The firmware's own heap is Zephyr's (`rust/app/src/allocator.rs`), so rivet's
allocator, built in, is linked only if something calls `malloc`; nothing here does.
rivet's `printf` family is built in too and unused: the firmware prints with `printk`
and Zephyr's log, which have their own formatter. What the link takes is the string
and memory functions Zephyr and the Rust code call, `strtol` from the network stack,
`errno`, and the Arm run-time ABI's memory helpers.

## The three images

| | C++ (newlib) | Rust (Zephyr's minimal C library) | Rust (rivet) |
|---|---|---|---|
| text | 545588 B | 156056 B | 155528 B |
| data | 9908 B | 6524 B | 6540 B |
| bss | 220557 B | 218836 B | 218848 B |
| scenario outcome | `explore: halted after 1195067 steps at pc=0x80669da` | `explore: halted after 1751203 steps at pc=0x801077a` | `explore: halted after 1609682 steps at pc=0x8010730` |
| steps in 5 s | 1195067 | 1751203 | 1609682 |
| busy ticks | 1194890 (0.05%) | 1740371 (0.07%) | 1609231 (0.07%) |
| console lines (test.sh run) | 136 | 136 | 136 |
| properties reported | 17 | 18 | 18 |

The rivet image is a little smaller than the Rust image on Zephyr's C library and
takes fewer steps through the same scenario: rivet's `memcpy`, `memset`, and `memcmp`
dispatch on the size to word and block patterns, where Zephyr's minimal library runs
byte loops, and `memset` and `memcpy` are among the busiest functions of the run.

### Where the busy time goes

Rust (Zephyr's minimal C library):

| Function | Share of busy time |
|---|---|
| `uart_stm32_poll_out` | 6.4% |
| `memset` | 5.8% |
| `z_cbvprintf_impl` | 5.0% |
| `sys_clock_set_timeout` | 4.5% |
| `elapsed` | 3.7% |
| `memcpy` | 3.6% |

Rust (rivet):

| Function | Share of busy time |
|---|---|
| `uart_stm32_poll_out` | 7.0% |
| `z_cbvprintf_impl` | 5.4% |
| `sys_clock_set_timeout` | 4.8% |
| `elapsed` | 4.1% |
| `console_out` | 3.7% |
| `z_add_timeout` | 2.9% |

## Properties

| Property | Kind | C++ (newlib) | Rust (Zephyr's minimal C library) | Rust (rivet) |
|---|---|---|---|---|
| someip: an unknown method is answered with an error | sometimes | passed (12) | passed (12) | passed (12) |
| someip: a response names the request's service and method | always | passed (10) | passed (10) | passed (10) |
| someip: a handler refuses a request | sometimes | passed (10) | passed (10) | passed (10) |
| body_ecu: the LEDs' GPIO ports are ready | always | passed (1) | passed (1) | passed (1) |
| body_ecu: all systems running | reachable | passed (1) | passed (1) | passed (1) |
| body_ecu: an estd assertion fails | unreachable | not reached | passed (0) | passed (0) |
| door_lock: a state change changes the state | always | passed (2) | passed (2) | passed (2) |
| door_lock: the door locks | sometimes | passed (2) | passed (2) | passed (2) |
| door_lock: the door unlocks | sometimes | passed (2) | passed (2) | passed (2) |
| door_lock: entering Run locks the door | sometimes | passed (1) | passed (1) | passed (1) |
| ignition: cranking only while the mode is Crank | always | passed (1) | passed (1) | passed (1) |
| ignition: the button is pressed in Run | sometimes | passed (1) | passed (1) | passed (1) |
| lighting: a light's state follows the request | always | passed (2) | passed (2) | passed (2) |
| lighting: a light turns on | sometimes | passed (2) | passed (2) | passed (2) |
| lighting: a light turns off | sometimes | passed (2) | passed (2) | passed (2) |
| vehicle_mode: an invalid transition is refused | sometimes | passed (4) | passed (4) | passed (4) |
| vehicle_mode: only valid transitions happen | always | passed (3) | passed (3) | passed (3) |
| vehicle_mode: the vehicle runs | sometimes | passed (3) | passed (3) | passed (3) |

## What rivet needed

Zephyr's socket types include `<sys/_timeval.h>`, the header newlib keeps
`struct timeval` in, from every C library but Zephyr's minimal one; rivet gained it
(`rust/third_party/rivet/VENDOR.md` names the revision). Everything else Zephyr's
kernel, network stack, and drivers and the OpenBSW crates call, rivet already had.

## How to reproduce

```sh
cd forkpoint
make test-cpp test-rust test-rust-rivet   # each image; the rivet one checks its map first
make run-cpp run-rust run-rust-rivet       # the scenario outcome of each
make equiv-rivet                           # fpt equiv against both other images
make report-rivet                          # this file
```
