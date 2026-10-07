#!/usr/bin/env bash
# Copyright 2026 Mist Tecnologia LTDA. All rights reserved.
#
# Compare the C++ and the Rust Body ECU images on Forkpoint: an observer outside the chip
# must not tell them apart. For each scenario, `fpt equiv` runs both images for the same
# virtual time with the same scripted inputs and compares every frame on the Ethernet link
# (UDP byte for byte) and how the runs ended; compare_consoles.py compares the consoles
# with the stamps and addresses that differ between any two builds masked; and the
# property verdicts of each image's own run are compared by message.
#
#   FPT=... FPT_BOARDS=... ./equiv.sh         outputs under out/equiv-*
#
# CPP_ELF and RUST_ELF name other images (defaults: the C++ and the Rust image), CPP_NAME
# and RUST_NAME the test.sh runs whose property verdicts are compared (cpp, rust), and
# PREFIX where the outputs go (out/PREFIX-*).
set -euo pipefail

here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
fpt=${FPT:-fpt}
boards=${FPT_BOARDS:?set FPT_BOARDS to the boards directory of the Forkpoint checkout}
board=${BOARD:-nucleo-h743zi}
cycles=${CYCLES:-2400000000}
out="$here/out"
cpp=${CPP_ELF:-$out/body-ecu-cpp.elf}
rust=${RUST_ELF:-$out/body-ecu-rust.elf}
cpp_name=${CPP_NAME:-cpp}
rust_name=${RUST_NAME:-rust}
prefix=${PREFIX:-equiv}
error="$out/$prefix-error.txt"

equiv() {
    local name="$1"
    shift
    rm -rf "$out/$prefix-$name-console" "$out/$prefix-$name-wire"
    "$fpt" equiv --boards-dir "$boards" --board "$board" --max-cycles "$cycles" "$@" \
        --console "$out/$prefix-$name-console" --ignore-console --wire "$out/$prefix-$name-wire" \
        "$cpp" "$rust" >"$out/$prefix-$name.txt" 2>"$error" || {
        cat "$out/$prefix-$name.txt" "$error" >&2
        exit 1
    }
    python3 "$here/compare_consoles.py" \
        "$out/$prefix-$name-console/a.txt" "$out/$prefix-$name-console/b.txt" || exit 1
    echo "OK: $name: $(cat "$out/$prefix-$name.txt")"
}

# Boot alone: nothing arrives, the ECU's own traffic (ARP for the tester it never
# hears from, nothing else) and every console line through the lifecycle.
equiv boot
# The scenario: the SOME/IP tester's requests and the button.
equiv scenario --i2c-script "$here/scripts/scenario.script"
grep -qE 'frames \([1-9][0-9]* sent, [1-9][0-9]* received' "$out/$prefix-scenario.txt" || {
    echo "the scenario put nothing on the wire:" >&2
    cat "$out/$prefix-scenario.txt" >&2
    exit 1
}

# The properties each image's own run judged (test.sh wrote them): the C++ names its by
# the message's hash alone, the Rust by the message too, so they are compared by id. The
# verdict is each property's status; the hit counts are reported, not required to match,
# since a scheduling difference can change how often a passing property is checked.
python3 - "$out/$cpp_name-properties.jsonl" "$out/$rust_name-properties.jsonl" <<'PY' || exit 1
import json, sys
verdicts = []
for path in sys.argv[1:]:
    rows = [json.loads(line) for line in open(path)]
    verdicts.append({(r["id"], r["kind"]): (r["status"], r["hits"]) for r in rows})
cpp, rust = verdicts
missing = set(cpp) - set(rust)
if missing:
    sys.exit(f"properties the C++ reported and the Rust did not: {sorted(missing)}")
differ = {k: (cpp[k][0], rust[k][0]) for k in cpp if cpp[k][0] != rust[k][0]}
if differ:
    sys.exit(f"property verdicts differ: {differ}")
hits = {k: (cpp[k][1], rust[k][1]) for k in cpp if cpp[k][1] != rust[k][1]}
extra = set(rust) - set(cpp)
print(f"{len(cpp)} property verdicts match; the Rust catalog adds {len(extra)} the C++ run never reached")
if hits:
    print(f"hit counts differ on {len(hits)} of them (not part of the verdict): {hits}")
else:
    print("the hit counts match too")
PY
echo "OK: the property verdicts match"
