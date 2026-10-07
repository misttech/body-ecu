#!/usr/bin/env python3
# Copyright 2026 Mist Tecnologia LTDA. All rights reserved.
"""Compare the consoles of two Body ECU images that `fpt equiv --console DIR` wrote.

Two images of the same program differ on the console only where the text names
something that is not the program's behavior: Zephyr's log stamps every line
from the log with the time (`[00:00:01.252,000]`), which the two builds reach a
few microseconds apart, and the blink test prints the address of each LED's port
device, which the linker decides. Both are masked; everything else, every line
in order, must match byte for byte.

    compare_consoles.py out/console/a.txt out/console/b.txt

Exits 0 when the masked consoles match, and 1 at the first differing line,
which it prints. Standard library only.
"""

from __future__ import annotations

import re
import sys

TIMESTAMP = re.compile(r"^\[\d\d:\d\d:\d\d\.\d{3},\d{3}\] ")
PORT = re.compile(r"port=0x[0-9a-f]+")


def mask(line: str) -> str:
    return PORT.sub("port=*", TIMESTAMP.sub("[*] ", line))


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("usage: compare_consoles.py A B", file=sys.stderr)
        return 2
    sides = []
    for path in argv:
        with open(path, encoding="utf-8", errors="replace") as console:
            sides.append([mask(line) for line in console.read().splitlines()])
    a, b = sides
    for index in range(max(len(a), len(b))):
        left = a[index] if index < len(a) else "(ended)"
        right = b[index] if index < len(b) else "(ended)"
        if left != right:
            print(f"console line {index + 1}: A {left!r} | B {right!r}")
            return 1
    print(f"{len(a)} console lines match")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
