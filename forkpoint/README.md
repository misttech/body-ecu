# Body ECU on Forkpoint

Run the MCU firmware on [Forkpoint](https://github.com/misttech/forkpoint)'s
deterministic virtual MCU: the same `nucleo_h753zi` image, on the virtual NUCLEO-H743ZI
(`--board nucleo-h743zi`), with a SOME/IP tester and the user button scripted from
outside, every property the firmware states judged after the run, and every run
recorded and replayable.

```sh
# Once: a west workspace, Forkpoint built from its checkout, and an Arm GNU toolchain.
west init -l body-ecu && west update
make -C <forkpoint> release                       # out/default/release/fpt
export FPT=<forkpoint>/out/default/release/fpt FPT_BOARDS=<forkpoint>/boards
export ZEPHYR_TOOLCHAIN_VARIANT=gnuarmemb \
       GNUARMEMB_TOOLCHAIN_PATH=<forkpoint>/prebuilt/toolchains/arm-none-eabi

cd body-ecu/forkpoint
make build-cpp     # the C++ firmware, instrumented (BODY_ECU_FORKPOINT=ON)
make run-cpp       # one scripted run: out/cpp-run.txt, out/cpp-log.txt, properties
make test-cpp      # test.sh: milestones, properties, determinism, replay
make build-rust    # the Rust firmware (rust/, on the Rust OpenBSW port)
make test-rust     # the same checks on the Rust image
make equiv         # equiv.sh: fpt equiv of both images, consoles, catalogs
make report        # docs/forkpoint/set3-equivalence.md from out/
```

`west build` needs Zephyr's Python modules (`pip install -r
<forkpoint>/build/python/zephyr-requirements.txt west`), and `make test-cpp` writes
everything under `out/`.

## What the firmware reports

`BODY_ECU_FORKPOINT=ON` defines `FPT_ENABLE` for every library, and the
[Forkpoint SDK](../third_party/forkpoint-sdk/README.md) macros then report through the
hostcall window at `0xA0000000`, which exists only on the virtual board. Without the
option they compile to nothing, so production images and the host unit tests are
unchanged.

| Where | What |
|---|---|
| `app/src/main.cpp` | `fpt_setup_complete` once every system runs, so `fpt explore --fork-after-setup` branches from there; the GPIO ports are ready; an `estd` assertion failing is unreachable |
| `SomeIpSystem::dispatch` | every request as an event; a response names the request's service and method; an unknown method is answered with an error; a handler sometimes refuses |
| `DoorLockController` | a state change changes the state; the door locks and unlocks; entering Run locks it; each state as an event |
| `LightingController` | a light's state follows the request; lights turn on and off; each change as an event |
| `VehicleModeManager` | only valid transitions happen; an invalid one is refused; the vehicle runs; each mode as an event |
| `IgnitionController` | cranking only while the mode is Crank; the button is pressed in Run; each press as an event |

The C++ SDK records no catalog, so `fpt` judges only the properties a run reaches:
an `FPT_ALWAYS` the run never reaches is not reported. The Rust build carries the
catalog: `fpt catalog out/body-ecu-rust.elf` lists every property it states, and
`rust/app/fpt_catalog.ld` keeps the records out of the image.

## The Rust firmware

`rust/` is the MCU firmware in Rust (see `rust/README.md`): the same systems on the Rust
port of OpenBSW, wired as `main.cpp` wires them, printing the same lines and stating the
same properties. `make build-rust` builds it through west like the C++ one, and
`make equiv` runs `fpt equiv` on the two images through the scenario: every UDP frame
the ECU sends must match byte for byte, and `compare_consoles.py` requires the consoles
to match line for line once Zephyr's log timestamps and the LED port addresses, which
differ between any two images, are masked.

## The scenario

`scripts/scenario.script` is what happens to the ECU from outside, on virtual time: a
SOME/IP tester at `192.168.100.1` sends requests to `192.168.100.10:30490` from 2 s
on (`vehicle_info` GetVIN, `lighting` set and get, `door_lock` lock, status and unlock,
`vehicle_mode` set Run, get, and an invalid set Off, `speed_sensor` GetSpeed, and a
method nobody registered), and the user presses the button at 3.5 s, which takes the
vehicle from Run back to Accessory. The scripted network answers ARP for the tester,
so the ECU's responses reach the wire, where `fpt equiv` compares them between images.

## The virtual board

Forkpoint's `nucleo-h743zi` models what this image's drivers wait on: the RCC's
oscillators and PLLs ready at once, USART3 (the console) and LPUART1, the RNG behind
the entropy driver (seeded, so a recording replays its words), the Ethernet MAC with a
scripted network peer, GPIO ports and EXTI for the LEDs and the button, the timers
behind the kernel tick, and the debug identity code, which reads 0 so the application
takes its emulator path, as it does on Renode. `forkpoint.conf` leaves the ADC out as
`renode.conf` does: there is no STM32H7 ADC model, and the speed simulator is off on an
emulator anyway.
