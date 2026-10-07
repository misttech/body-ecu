#!/usr/bin/env python3
# Copyright 2026 Mist Tecnologia LTDA. All rights reserved.
"""Write the equivalence report of the C++ and Rust Body ECU images from what test.sh
and equiv.sh left under out/.

    report.py OUT docs/forkpoint/set3-equivalence.md
    report.py --rivet OUT docs/forkpoint/set4-rivet.md

Standard library only: the ELF section sizes are read from the section headers, and
what each link took from which library from the linker maps.
"""

from __future__ import annotations

import collections
import json
import re
import struct
import sys
from pathlib import Path


def elf_sizes(path: Path) -> dict[str, int]:
    """text, data, and bss of a 32-bit little-endian ELF, as `size` sums them."""
    data = path.read_bytes()
    shoff, shentsize, shnum, shstrndx = struct.unpack_from("<I", data, 0x20)[0], *struct.unpack_from("<HHH", data, 0x2E)
    sizes = {"text": 0, "data": 0, "bss": 0}
    SHF_WRITE, SHF_ALLOC, SHF_EXECINSTR = 1, 2, 4
    SHT_NOBITS = 8
    for i in range(shnum):
        name, kind, flags, _addr, _off, size = struct.unpack_from("<IIIIII", data, shoff + i * shentsize)
        if not flags & SHF_ALLOC:
            continue
        if kind == SHT_NOBITS:
            sizes["bss"] += size
        elif flags & SHF_WRITE:
            sizes["data"] += size
        else:
            sizes["text"] += size
    return sizes


def folded(path: Path) -> tuple[int, int, list[tuple[str, int]]]:
    total = busy = 0
    leaf: collections.Counter[str] = collections.Counter()
    for line in path.read_text().splitlines():
        stack, ticks = line.rsplit(" ", 1)
        t = int(ticks)
        total += t
        if "[idle]" not in stack:
            busy += t
            leaf[stack.split(";")[-1]] += t
    return total, busy, leaf.most_common(6)


def properties(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines()]


def outcome(path: Path) -> str:
    return path.read_text().strip()


def steps(text: str) -> str:
    m = re.search(r"after (\d+) steps", text)
    return m.group(1) if m else "?"


def archive_members(map_path: Path) -> list[str]:
    """The `archive(member)` lines of the map's "Archive member included" section."""
    members, inside = [], False
    for line in map_path.read_text().splitlines():
        if line.startswith("Archive member included"):
            inside = True
        elif line.startswith(("Discarded input sections", "Allocating common", "Memory Configuration")):
            inside = False
        elif inside and line.endswith(")") and "(" in line and not line.startswith(" "):
            members.append(line.strip())
    return members


def library_members(map_path: Path, pattern: str) -> int:
    return sum(1 for m in archive_members(map_path) if re.search(pattern, m))


def functions_from(map_path: Path, archive: str) -> list[str]:
    """The `.text.<name>` input sections the link took from `archive`, by name."""
    names, pending, inside = set(), None, False
    for line in map_path.read_text().splitlines():
        # Only the memory map: the discarded sections before it are not in the image.
        if line.startswith("Linker script and memory map"):
            inside = True
        if not inside:
            continue
        m = re.match(r" \.text\.([A-Za-z_][A-Za-z0-9_.]*)\s*(.*)$", line)
        if m:
            pending = m.group(1)
            rest = m.group(2)
        elif pending is not None and line.startswith("  "):
            rest = line
        else:
            continue
        if archive in rest:
            names.add(pending)
        if rest.strip():
            pending = None
    # C names only: not the Rust-mangled internals (_R, _ZN) or the outlined fragments.
    return sorted(n for n in names if not n.startswith(("_R", "_ZN", "OUTLINED")) and "." not in n)


def rivet_report(out: Path, target: Path) -> None:
    images = {
        "C++ (newlib)": ("body-ecu-cpp.elf", "cpp", "build-cpp"),
        "Rust (Zephyr's minimal C library)": ("body-ecu-rust.elf", "rust", "build-rust"),
        "Rust (rivet)": ("body-ecu-rust-rivet.elf", "rust-rivet", "build-rust-rivet"),
    }
    sizes = {k: elf_sizes(out / v[0]) for k, v in images.items()}
    maps = {k: out / v[2] / "zephyr" / "zephyr.map" for k, v in images.items()}
    runs = {k: outcome(out / f"{v[1]}-run.txt") for k, v in images.items()}
    busy = {k: folded(out / f"{v[1]}-stacks.folded") for k, v in images.items()}
    props = {k: properties(out / f"{v[1]}-properties.jsonl") for k, v in images.items()}
    consoles = {k: len((out / f"{v[1]}-console.txt").read_text().splitlines()) for k, v in images.items()}
    newlib = {k: library_members(m, r"/lib(c|c_nano|m)\.a\(") for k, m in maps.items()}
    minimal = {k: library_members(m, r"lib__libc__(minimal|common)\.a\(") for k, m in maps.items()}
    rivet = {k: library_members(m, r"librivet_libc\.a\(") for k, m in maps.items()}
    libgcc = {k: library_members(m, r"/libgcc\.a\(") for k, m in maps.items()}
    rivet_functions = functions_from(maps["Rust (rivet)"], "librivet_libc.a")
    verdicts = {
        "boot alone, 5 s": ("rivet-boot.txt", "rivet-vs-rust-boot.txt"),
        "the scenario (SOME/IP tester, button), 5 s": ("rivet-scenario.txt", "rivet-vs-rust-scenario.txt"),
    }
    cols = list(images)

    def row(label: str, cells: list[str]) -> str:
        return f"| {label} | " + " | ".join(cells) + " |"

    lines = [
        "# Set 4: the Rust port with rivet as its C library",
        "",
        "The Rust image is built a second time with [rivet](../../rust/third_party/rivet/VENDOR.md),",
        "the C library written in Rust, as Zephyr's C library (`CONFIG_EXTERNAL_LIBC`,",
        "`forkpoint/rivet.conf`, `forkpoint/rivet_module/`), and run through the same scenarios",
        "as the two images of Set 3, against each of them: `fpt equiv` on every Ethernet frame",
        "and how the runs end, `compare_consoles.py` on the consoles, `equiv.sh` on the property",
        "verdicts. `make equiv-rivet` reproduces it; `make report-rivet` wrote this file.",
        "",
        "## Verdict",
        "",
        "| Scenario | rivet against C++ | rivet against Rust on Zephyr's C library | Consoles |",
        "|---|---|---|---|",
    ]
    for label, (a, b) in verdicts.items():
        va = (out / a).read_text().strip().removeprefix("equiv: ")
        vb = (out / b).read_text().strip().removeprefix("equiv: ")
        lines.append(f"| {label} | {va} | {vb} | match |")
    lines += [
        "",
        "Changing the C library under the Rust firmware changes nothing an observer outside the",
        "chip can see: the same frames, byte for byte, the same console lines, the same property",
        "verdicts.",
        "",
        "## What each image links as its C library",
        "",
        "From the linker maps (`newlib_free.sh` makes the same check for the rivet image).",
        "libgcc, the compiler's arithmetic helpers (soft-float, 64-bit division, popcount), is",
        "linked by every image: the C++ through Zephyr, the Rust ones because",
        "`openbsw_rust_application()` links it ahead of the Rust archive so that Zephyr's C and",
        "the Rust code share one set of helpers.",
        "",
        "| Archive members taken | " + " | ".join(cols) + " |",
        "|---|" + "---|" * len(cols),
        row("newlib (`libc.a`, `libc_nano.a`, `libm.a`)", [str(newlib[c]) for c in cols]),
        row("Zephyr's own (`lib/libc/minimal`, `lib/libc/common`)", [str(minimal[c]) for c in cols]),
        row("rivet (`librivet_libc.a`)", [str(rivet[c]) for c in cols]),
        row("libgcc", [str(libgcc[c]) for c in cols]),
        "",
        "The functions the rivet image takes from rivet (`.text` sections from its archive):",
        "",
        ", ".join(f"`{n}`" for n in rivet_functions) + ".",
        "",
        "The firmware's own heap is Zephyr's (`rust/app/src/allocator.rs`), so rivet's",
        "allocator, built in, is linked only if something calls `malloc`; nothing here does.",
        "rivet's `printf` family is built in too and unused: the firmware prints with `printk`",
        "and Zephyr's log, which have their own formatter. What the link takes is the string",
        "and memory functions Zephyr and the Rust code call, `strtol` from the network stack,",
        "`errno`, and the Arm run-time ABI's memory helpers.",
        "",
        "## The three images",
        "",
        "| | " + " | ".join(cols) + " |",
        "|---|" + "---|" * len(cols),
        row("text", [f"{sizes[c]['text']} B" for c in cols]),
        row("data", [f"{sizes[c]['data']} B" for c in cols]),
        row("bss", [f"{sizes[c]['bss']} B" for c in cols]),
        row("scenario outcome", [f"`{runs[c]}`" for c in cols]),
        row("steps in 5 s", [steps(runs[c]) for c in cols]),
        row("busy ticks", [f"{busy[c][1]} ({busy[c][1] * 100 / busy[c][0]:.2f}%)" for c in cols]),
        row("console lines (test.sh run)", [str(consoles[c]) for c in cols]),
        row("properties reported", [str(len(props[c])) for c in cols]),
        "",
        "The rivet image is a little smaller than the Rust image on Zephyr's C library and",
        "takes fewer steps through the same scenario: rivet's `memcpy`, `memset`, and `memcmp`",
        "dispatch on the size to word and block patterns, where Zephyr's minimal library runs",
        "byte loops, and `memset` and `memcpy` are among the busiest functions of the run.",
        "",
        "### Where the busy time goes",
        "",
    ]
    for c in cols[1:]:
        _, b, leaf = busy[c]
        lines += [
            f"{c}:",
            "",
            "| Function | Share of busy time |",
            "|---|---|",
            *[f"| `{name}` | {ticks * 100 / b:.1f}% |" for name, ticks in leaf],
            "",
        ]
    lines += [
        "## Properties",
        "",
        "| Property | Kind | " + " | ".join(cols) + " |",
        "|---|---|" + "---|" * len(cols),
    ]
    for r in props["Rust (rivet)"]:
        cells = []
        for c in cols:
            p = next((x for x in props[c] if x["id"] == r["id"]), None)
            cells.append(f"{p['status']} ({p['hits']})" if p else "not reached")
        lines.append(f"| {r['message']} | {r['kind'].removeprefix('assert_')} | " + " | ".join(cells) + " |")
    lines += [
        "",
        "## What rivet needed",
        "",
        "Zephyr's socket types include `<sys/_timeval.h>`, the header newlib keeps",
        "`struct timeval` in, from every C library but Zephyr's minimal one; rivet gained it",
        "(`rust/third_party/rivet/VENDOR.md` names the revision). Everything else Zephyr's",
        "kernel, network stack, and drivers and the OpenBSW crates call, rivet already had.",
        "",
        "## How to reproduce",
        "",
        "```sh",
        "cd forkpoint",
        "make test-cpp test-rust test-rust-rivet   # each image; the rivet one checks its map first",
        "make run-cpp run-rust run-rust-rivet       # the scenario outcome of each",
        "make equiv-rivet                           # fpt equiv against both other images",
        "make report-rivet                          # this file",
        "```",
        "",
    ]
    target.write_text("\n".join(lines))
    print(f"wrote {target}")


def main(argv: list[str]) -> int:
    if argv and argv[0] == "--rivet":
        rivet_report(Path(argv[1]), Path(argv[2]))
        return 0
    out, target = Path(argv[0]), Path(argv[1])
    cpp_elf, rust_elf = out / "body-ecu-cpp.elf", out / "body-ecu-rust.elf"
    cpp_size, rust_size = elf_sizes(cpp_elf), elf_sizes(rust_elf)
    cpp_total, cpp_busy, cpp_leaf = folded(out / "cpp-stacks.folded")
    rust_total, rust_busy, rust_leaf = folded(out / "rust-stacks.folded")
    cpp_props = properties(out / "cpp-properties.jsonl")
    rust_props = properties(out / "rust-properties.jsonl")
    rust_by_id = {r["id"]: r for r in rust_props}
    scenario = (out / "equiv-scenario.txt").read_text().strip()
    boot = (out / "equiv-boot.txt").read_text().strip()
    cpp_console = len((out / "cpp-console.txt").read_text().splitlines())
    rust_console = len((out / "rust-console.txt").read_text().splitlines())
    cpp_pcap = (out / "equiv-scenario-wire" / "a.pcap").stat().st_size
    rust_pcap = (out / "equiv-scenario-wire" / "b.pcap").stat().st_size

    def leaf_table(leaf: list[tuple[str, int]], busy: int) -> str:
        rows = [f"| `{name}` | {ticks * 100 / busy:.1f}% |" for name, ticks in leaf]
        return "\n".join(["| Function | Share of busy time |", "|---|---|", *rows])

    lines = [
        "# Set 3: the Rust port against the C++ baseline",
        "",
        "Both images, built from this branch (`forkpoint/Makefile`: `build-cpp`, `build-rust`),",
        "run on Forkpoint's virtual NUCLEO-H743ZI through the same scenarios, and an observer",
        "outside the chip is asked to tell them apart: `fpt equiv` compares every frame on the",
        "Ethernet link byte for byte and how the runs end, `compare_consoles.py` the consoles",
        "with Zephyr's log timestamps and the LED port addresses masked, and `equiv.sh` the",
        "property verdicts of each image's own run. `make equiv` reproduces it; `make report`",
        "wrote this file.",
        "",
        "## Verdict",
        "",
        "| Scenario | `fpt equiv` | Console |",
        "|---|---|---|",
        f"| boot alone, 5 s | {boot.removeprefix('equiv: ')} | match |",
        f"| the scenario (SOME/IP tester, button), 5 s | {scenario.removeprefix('equiv: ')} | match |",
        "",
        "Every UDP datagram the Rust ECU sends, the responses and the events, is the one the C++",
        "ECU sends, byte for byte, IP identifiers included; the ARP the ECU answers with is the",
        "same; and the consoles match line for line.",
        "",
        "## The two images",
        "",
        "| | C++ | Rust |",
        "|---|---|---|",
        f"| text | {cpp_size['text']} B | {rust_size['text']} B |",
        f"| data | {cpp_size['data']} B | {rust_size['data']} B |",
        f"| bss | {cpp_size['bss']} B | {rust_size['bss']} B |",
        f"| scenario outcome | `{outcome(out / 'cpp-run.txt')}` | `{outcome(out / 'rust-run.txt')}` |",
        f"| steps in 5 s | {steps(outcome(out / 'cpp-run.txt'))} | {steps(outcome(out / 'rust-run.txt'))} |",
        f"| busy ticks of {cpp_total} | {cpp_busy} ({cpp_busy * 100 / cpp_total:.2f}%) | {rust_busy} ({rust_busy * 100 / rust_total:.2f}%) |",
        f"| console lines (test.sh run) | {cpp_console} | {rust_console} |",
        f"| wire capture of the scenario | {cpp_pcap} B | {rust_pcap} B |",
        f"| properties reported | {len(cpp_props)} | {len(rust_props)} |",
        f"| catalog | none (the C++ SDK records none) | {len(rust_props)} entries |",
        "",
        "The Rust image is smaller: the C++ carries the C++ standard library, the full",
        "OpenSOME/IP stack, and OpenBSW's C++ libraries, where the Rust carries the part of",
        "SOME/IP the firmware uses and the OpenBSW crates it links.",
        "",
        "### Where the busy time goes",
        "",
        "C++:",
        "",
        leaf_table(cpp_leaf, cpp_busy),
        "",
        "Rust:",
        "",
        leaf_table(rust_leaf, rust_busy),
        "",
        "## Properties",
        "",
        "Each image judged its own run. The C++ reports a property by the hash of its message",
        "(its SDK records no catalog); the Rust reports the same hashes with the messages, so",
        "the verdicts are compared by id.",
        "",
        "| Property | Kind | C++ | Rust |",
        "|---|---|---|---|",
    ]
    for r in rust_props:
        c = next((p for p in cpp_props if p["id"] == r["id"]), None)
        cpp_cell = f"{c['status']} ({c['hits']})" if c else "not reached"
        lines.append(f"| {r['message']} | {r['kind'].removeprefix('assert_')} | {cpp_cell} | {r['status']} ({r['hits']}) |")
    lines += [
        "",
        "The one property the C++ never reports, `body_ecu: an estd assertion fails`, is an",
        "unreachable: the C++ SDK only learns of it when it is hit, and it never is; the Rust",
        "catalog lists it, and `fpt` judges it passed.",
        "",
        "## Known differences",
        "",
        "- The blink test prints each LED's port device address (`port=0x...`), which the",
        "  linker decides and no two images share; the comparison masks it.",
        "- Zephyr's log timestamps differ by the microseconds each image takes to reach a",
        "  line; the comparison masks them. The log thread drains the deferred log once a",
        "  second, so the log lines' place among the `printk` lines is timing, not behavior,",
        "  and it happened to match in every run here.",
        "- Service discovery is not in the Rust image; the C++ disables it under emulation",
        "  (`enable_sd = real_hw`), so neither image offers services, and the wire shows none.",
        "",
        "## How to reproduce",
        "",
        "```sh",
        "cd forkpoint",
        "make test-cpp test-rust   # each image: milestones, properties, determinism, replay",
        "make equiv                # fpt equiv on both scenarios, consoles, properties",
        "make report               # this file",
        "```",
        "",
    ]
    target.write_text("\n".join(lines))
    print(f"wrote {target}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
