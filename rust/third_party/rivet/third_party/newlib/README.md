# newlib

Tests from newlib's testsuite, ported into rivet's test suite. They live here,
under `test/<header>/`, and `test/build.rs` builds them with rivet's own C
tests. Each ported file keeps its upstream copyright and license notice;
`COPYING.NEWLIB` collects the licenses newlib is distributed under.

- Upstream: https://sourceware.org/newlib/
- Release: 4.5.0 (`newlib-4.5.0.20241231.tar.gz`)
- License: the BSD-style licenses each file states, collected in
  `COPYING.NEWLIB`

## Ported tests

| Upstream file | Rivet file | License |
|---|---|---|
| `newlib/testsuite/newlib.string/memcpy-1.c` | `third_party/newlib/test/string/memcpy-1.c` | BSD-3-Clause, ARM Ltd |
| `newlib/testsuite/newlib.string/memmove1.c` | `third_party/newlib/test/string/memmove1.c` | BSD-3-Clause, Axis Communications |
| `newlib/testsuite/newlib.string/strcmp-1.c` | `third_party/newlib/test/string/strcmp-1.c` | BSD-3-Clause, ARM Ltd |
| `newlib/testsuite/newlib.string/tstring.c` | `third_party/newlib/test/string/tstring.c` | Red Hat permissive notice |

## Changes

The tests run together in one firmware image, with no C library beyond rivet,
so each ported file has these mechanical changes and no others:

- `main` becomes `RIVET_TEST_NAME`, the function `build.rs` names after the
  file; `abort ()` becomes `return 1;` and `exit (0)` becomes `return 0;`.
- `#include "rivet_test.h"` is added. `printf`, `fprintf (stderr, ...)`,
  `vfprintf (stderr, ...)`, `srand`, and `rand` become the harness's
  `rivet_test_printf`, `rivet_test_vprintf`, `rivet_test_srand`, and
  `rivet_test_rand`, which rivet does not provide. The harness's `rand` is
  newlib's generator, so the tests see the same data.
- File-scope variables and helper functions (`errors`, `testname`,
  `print_error`, `printbuf`, `mymemmove`, `xmemmove`, `fill`, `eprintf`,
  `mycopy`, `myset`) become `static`, so the tests do not collide when linked
  together.
- `memcpy-1.c` no longer prints a `.` per copy.
- A comment after the license notice names the upstream file.
