# Set 3: the Rust port against the C++ baseline

Both images, built from this branch (`forkpoint/Makefile`: `build-cpp`, `build-rust`),
run on Forkpoint's virtual NUCLEO-H743ZI through the same scenarios, and an observer
outside the chip is asked to tell them apart: `fpt equiv` compares every frame on the
Ethernet link byte for byte and how the runs end, `compare_consoles.py` the consoles
with Zephyr's log timestamps and the LED port addresses masked, and `equiv.sh` the
property verdicts of each image's own run. `make equiv` reproduces it; `make report`
wrote this file.

## Verdict

| Scenario | `fpt equiv` | Console |
|---|---|---|
| boot alone, 5 s | equivalent: 37 console lines, 6 frames (6 sent, 0 received) | match |
| the scenario (SOME/IP tester, button), 5 s | equivalent: 136 console lines, 38 frames (25 sent, 13 received) | match |

Every UDP datagram the Rust ECU sends, the responses and the events, is the one the C++
ECU sends, byte for byte, IP identifiers included; the ARP the ECU answers with is the
same; and the consoles match line for line.

## The two images

| | C++ | Rust |
|---|---|---|
| text | 545588 B | 156056 B |
| data | 9908 B | 6524 B |
| bss | 220557 B | 218836 B |
| scenario outcome | `explore: halted after 1195067 steps at pc=0x80669da` | `explore: halted after 1751203 steps at pc=0x801077a` |
| steps in 5 s | 1195067 | 1751203 |
| busy ticks of 2416297176 | 1194890 (0.05%) | 1740371 (0.07%) |
| console lines (test.sh run) | 136 | 136 |
| wire capture of the scenario | 2962 B | 2962 B |
| properties reported | 17 | 18 |
| catalog | none (the C++ SDK records none) | 18 entries |

The Rust image is smaller: the C++ carries the C++ standard library, the full
OpenSOME/IP stack, and OpenBSW's C++ libraries, where the Rust carries the part of
SOME/IP the firmware uses and the OpenBSW crates it links.

### Where the busy time goes

C++:

| Function | Share of busy time |
|---|---|
| `z_cbvprintf_impl` | 12.9% |
| `uart_stm32_poll_out` | 9.4% |
| `memset` | 5.2% |
| `console_out` | 4.9% |
| `char_out` | 3.5% |
| `out_func` | 3.0% |

Rust:

| Function | Share of busy time |
|---|---|
| `uart_stm32_poll_out` | 6.4% |
| `memset` | 5.8% |
| `z_cbvprintf_impl` | 5.0% |
| `sys_clock_set_timeout` | 4.5% |
| `elapsed` | 3.7% |
| `memcpy` | 3.6% |

## Properties

Each image judged its own run. The C++ reports a property by the hash of its message
(its SDK records no catalog); the Rust reports the same hashes with the messages, so
the verdicts are compared by id.

| Property | Kind | C++ | Rust |
|---|---|---|---|
| someip: an unknown method is answered with an error | sometimes | passed (12) | passed (12) |
| someip: a response names the request's service and method | always | passed (10) | passed (10) |
| someip: a handler refuses a request | sometimes | passed (10) | passed (10) |
| body_ecu: the LEDs' GPIO ports are ready | always | passed (1) | passed (1) |
| body_ecu: all systems running | reachable | passed (1) | passed (1) |
| body_ecu: an estd assertion fails | unreachable | not reached | passed (0) |
| door_lock: a state change changes the state | always | passed (2) | passed (2) |
| door_lock: the door locks | sometimes | passed (2) | passed (2) |
| door_lock: the door unlocks | sometimes | passed (2) | passed (2) |
| door_lock: entering Run locks the door | sometimes | passed (1) | passed (1) |
| ignition: cranking only while the mode is Crank | always | passed (1) | passed (1) |
| ignition: the button is pressed in Run | sometimes | passed (1) | passed (1) |
| lighting: a light's state follows the request | always | passed (2) | passed (2) |
| lighting: a light turns on | sometimes | passed (2) | passed (2) |
| lighting: a light turns off | sometimes | passed (2) | passed (2) |
| vehicle_mode: an invalid transition is refused | sometimes | passed (4) | passed (4) |
| vehicle_mode: only valid transitions happen | always | passed (3) | passed (3) |
| vehicle_mode: the vehicle runs | sometimes | passed (3) | passed (3) |

The one property the C++ never reports, `body_ecu: an estd assertion fails`, is an
unreachable: the C++ SDK only learns of it when it is hit, and it never is; the Rust
catalog lists it, and `fpt` judges it passed.

## Known differences

- The blink test prints each LED's port device address (`port=0x...`), which the
  linker decides and no two images share; the comparison masks it.
- Zephyr's log timestamps differ by the microseconds each image takes to reach a
  line; the comparison masks them. The log thread drains the deferred log once a
  second, so the log lines' place among the `printk` lines is timing, not behavior,
  and it happened to match in every run here.
- Service discovery is not in the Rust image; the C++ disables it under emulation
  (`enable_sd = real_hw`), so neither image offers services, and the wire shows none.

## How to reproduce

```sh
cd forkpoint
make test-cpp test-rust   # each image: milestones, properties, determinism, replay
make equiv                # fpt equiv on both scenarios, consoles, properties
make report               # this file
```
