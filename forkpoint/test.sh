#!/usr/bin/env bash
# Copyright 2026 Mist Tecnologia LTDA. All rights reserved.
#
# Run one Body ECU image on Forkpoint and check what the firmware does: the boot
# milestones, the scenario's answers, that two runs are the same execution, that a
# recorded run replays, and that every property the firmware states holds.
#
#   ./test.sh NAME IMAGE        outputs under out/NAME-*
set -euo pipefail

here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
name=$1
elf=$2
fpt=${FPT:-fpt}
boards=${FPT_BOARDS:?set FPT_BOARDS to the boards directory of the Forkpoint checkout}
board=${BOARD:-nucleo-h743zi}
cycles=${CYCLES:-2400000000}
out="$here/out"
mkdir -p "$out"
script="$here/scripts/scenario.script"

error="$out/$name-error.txt"
# A run must reach the cycle budget. A fault, an idle CPU, or a livelock also ends the run
# with a zero exit status, so the outcome explore prints is what tells them apart.
run() {
    local outcome
    outcome=$("$fpt" explore --boards-dir "$boards" --board "$board" --max-cycles "$cycles" \
        --i2c-script "$script" "$@" "$elf" 2>"$error") || {
        cat "$error" >&2
        exit 1
    }
    case "$outcome" in
        "explore: halted after"*) ;;
        *)
            echo "unexpected explore outcome: $outcome" >&2
            cat "$error" >&2
            exit 1
            ;;
    esac
}
run --properties "$out/$name-properties.jsonl"
grep -F "[UART]" "$error" | grep -vF " write 0x" | sed 's/.*usart3@40004800 //' \
    >"$out/$name-console.txt"
# Each milestone is a line the firmware prints on USART3: Zephyr's banner, the
# application's checkpoints, and what the scenario makes it do.
for milestone in \
    "*** Booting Zephyr OS build" \
    "body_ecu: Body ECU starting (Zephyr + OpenBSW)" \
    "body_ecu: Hardware detection: emulated" \
    "[SOME/IP] Transport running on udp://0.0.0.0:30490" \
    "[ignition] Init: Off -> Accessory" \
    "body_ecu: LED blink test done (3 LEDs x 3 blinks)" \
    "body_ecu: CP11: lifecycle components added" \
    "body_ecu: Body ECU ready - all systems running" \
    "body_ecu: Published VIN event via SOME/IP" \
    "[SOME/IP] Received service=0x1004 method=0x0001" \
    "[light] set id=0 state=1" \
    "[SOME/IP] Received service=0x1001 method=0x0001" \
    "[SOME/IP] Received service=0x1002 method=0x0002" \
    "[SOME/IP] dispatch: method NOT FOUND" \
    "[light] set id=0 state=0" \
    "[ignition] Run -> Accessory" \
    "[SOME/IP] Received service=0x1001 method=0x0002"; do
    grep -qF -- "$milestone" "$out/$name-console.txt" || {
        echo "the firmware never printed: $milestone" >&2
        cat "$out/$name-console.txt" >&2
        exit 1
    }
done
echo "OK: $name boots, serves the scenario's SOME/IP requests, and takes the button"

# Every property the firmware states must hold: an always that was false or never
# reached, a sometimes never true, or a reachable never hit fails.
failed=$(grep -c '"status":"failed"' "$out/$name-properties.jsonl" || true)
total=$(wc -l <"$out/$name-properties.jsonl")
if [ "$failed" != 0 ]; then
    echo "$failed of $total properties failed:" >&2
    grep '"status":"failed"' "$out/$name-properties.jsonl" >&2
    exit 1
fi
echo "OK: $name holds all $total properties"

# Determinism: two runs are the same execution, and a recorded run replays.
for index in 1 2; do
    run --trace "$out/$name-run-$index.fpt"
done
"$fpt" diff "$out/$name-run-1.fpt" "$out/$name-run-2.fpt" >"$out/$name-diff.txt" || {
    echo "the second run is not the same execution as the first:" >&2
    cat "$out/$name-diff.txt" >&2
    exit 1
}
rm -f "$out/$name-run-"*.fpt
bundle="$out/$name-bundle"
rm -rf "$bundle"
run --record "$bundle" --flamegraph "$out/$name-stacks.folded"
"$fpt" replay "$bundle" >"$out/$name-replay.txt" 2>"$error" || {
    cat "$out/$name-replay.txt" "$error" >&2
    exit 1
}
echo "OK: $name is deterministic and replays"
