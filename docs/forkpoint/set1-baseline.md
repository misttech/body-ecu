# Set 1: the C++ Body ECU on Forkpoint

The MCU firmware as upstream builds it for `nucleo_h753zi`, instrumented with the
Forkpoint SDK (`BODY_ECU_FORKPOINT=ON`), run on Forkpoint's virtual NUCLEO-H743ZI
through `forkpoint/scripts/scenario.script`. This is the baseline the Rust port is
compared with (Set 3).

## The run

| | |
|---|---|
| Image | `forkpoint/out/body-ecu-cpp.elf`, Zephyr `ea6eade`, OpenBSW `82397e0`, opensomeip `v0.1.0` |
| Board | `nucleo-h743zi` (Forkpoint `boards/stm/nucleo-h743zi.dts`), `forkpoint.conf` (`CONFIG_ADC=n`) |
| Virtual time | 5 s at 480 MHz (`--max-cycles 2400000000`) |
| Outcome | `explore: halted after 1195067 steps`: the budget ran out, nothing faulted |
| CPU | busy 1.19 M of 2416 M ticks (0.05%); the rest in `wfi` between the tick, the scenario's requests, and the log thread |
| Console | 136 lines on USART3 |
| Determinism | two runs `fpt diff` identical; the recording replays (`10211 records compared`) |
| Properties | 17 reported, 17 passed (6 always, 10 sometimes, 1 reachable) |
| Events | 20: 12 `someip.request`, 3 `vehicle_mode.changed`, 2 `lighting.light`, 2 `door_lock.state`, 1 `ignition.button` |

Busy time goes mostly to logging: `z_cbvprintf_impl` 13%, `uart_stm32_poll_out` 9%,
`memset` 5%, `console_out` 5%; `SomeIpSystem::dispatch` is 2%.

## What the firmware does

Boot: Zephyr's banner, the PHY (`ID 7C131`, 100 Mb full duplex), the application's
`Body ECU starting`, `Hardware detection: emulated (Renode) (DBGMCU IDCODE=0x00000000)`,
the SOME/IP transport on `udp://0.0.0.0:30490` with service discovery disabled, the
ignition's `Off -> Accessory`, the DoIP stub, the LED blink test, checkpoints CP1 to
CP11, `Transitioning to run level 3...`, `Body ECU ready - all systems running`, and
`Published VIN event via SOME/IP` at 1.252 s.

The scenario, from 2 s:

| Request | Answer |
|---|---|
| `vehicle_info` GetVIN | response with the VIN |
| `lighting` set headlight on | `[light] set id=0 state=1`, response, a `light_status_changed` event after it |
| `lighting` get status | response `01 00 00` |
| `door_lock` lock | response, `lock_state_changed` event |
| `door_lock` get status | response `01` (locked) |
| `vehicle_mode` set Run | response, mode notifier event; the door lock observer locks (already locked) |
| `vehicle_mode` get | response `02` |
| `vehicle_mode` set Off | refused (`rc=0x01`): Run cannot go to Off |
| `speed_sensor` GetSpeed | `method NOT FOUND`, error response: the software-only handler is only registered when `CONFIG_ADC` is on |
| `lighting` method `0x00ff` | `method NOT FOUND`, error response |
| `lighting` set headlight off | `[light] set id=0 state=0` |
| button at 3.5 s | `[ignition] Run -> Accessory` |
| `door_lock` unlock at 4 s | response, `lock_state_changed` event |

## Properties

| Property | Kind | Verdict | Hits |
|---|---|---|---|
| door_lock: entering Run locks the door | sometimes | passed | 1 |
| body_ecu: all systems running | reachable | passed | 1 |
| vehicle_mode: the vehicle runs | sometimes | passed | 3 |
| body_ecu: the LEDs' GPIO ports are ready | always | passed | 1 |
| ignition: the button is pressed in Run | sometimes | passed | 1 |
| vehicle_mode: only valid transitions happen | always | passed | 3 |
| door_lock: the door unlocks | sometimes | passed | 2 |
| door_lock: the door locks | sometimes | passed | 2 |
| lighting: a light turns on | sometimes | passed | 2 |
| someip: an unknown method is answered with an error | sometimes | passed | 12 |
| door_lock: a state change changes the state | always | passed | 2 |
| vehicle_mode: an invalid transition is refused | sometimes | passed | 4 |
| lighting: a light turns off | sometimes | passed | 2 |
| someip: a handler refuses a request | sometimes | passed | 10 |
| lighting: a light's state follows the request | always | passed | 2 |
| ignition: cranking only while the mode is Crank | always | passed | 1 |
| someip: a response names the request's service and method | always | passed | 10 |

`fpt` names a C++ property by the hash of its message, since the C++ SDK records no
catalog; the table maps them back. `estd` assertions never fail (the unreachable
property is never hit), and no `FPT_ALWAYS` the run reaches is false.

## What Forkpoint needed

Running this image needed four additions to Forkpoint, submitted as
[misttech/forkpoint#220](https://github.com/misttech/forkpoint/pull/220) and
[#221](https://github.com/misttech/forkpoint/pull/221); until they land, `fpt` must be
built from a checkout carrying them (the `FPT` and `FPT_BOARDS` the Makefile takes).
They are the STM32H743's LPUART1, RNG, debug identity code, system memory, and the
RCC's low-speed oscillator ready flags, which Zephyr's board support touches during
boot; the NVIC lowering a level line that fell before it was taken; and the input
script's `udp` line, with the scripted network answering ARP for the script's hosts,
without which a server with a static address on the link receives nothing.

## Not exercised

DoIP is a stub on Zephyr (no TCP server), DoCAN and the CAN gateway are compiled out
(`CONFIG_CAN=n` in the upstream board configuration), and the speed simulator needs
real silicon (`is_real_hardware()`), so UDS and the ADC path are not in this baseline.
