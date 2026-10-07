#!/usr/bin/env bash
# Copyright 2026 Mist Tecnologia LTDA. All rights reserved.
#
# rivet is the whole C library: the link took no member of the toolchain's newlib (libc,
# libc_nano, libm). MAP names the linker map of the build. libgcc, the compiler's own
# arithmetic helpers, is listed but allowed: openbsw_rust_application() links it ahead of
# the Rust archive on purpose, so Zephyr's C and the Rust code share one set of helpers.
set -euo pipefail

map=${MAP:?set MAP to the linker map}
included=$(awk '/^Archive member included/ { f = 1 } /^(Discarded input sections|Allocating common|Memory Configuration)/ { f = 0 } f' "$map")
if grep -qE '/lib(c|c_nano|m)\.a\(' <<<"$included"; then
    echo "the link took members of newlib:" >&2
    grep -E '/lib(c|c_nano|m)\.a\(' <<<"$included" >&2
    exit 1
fi
gcc=$(grep -cE '/libgcc\.a\(' <<<"$included" || true)
echo "OK: the image links rivet and nothing from newlib; libgcc members: $gcc"
