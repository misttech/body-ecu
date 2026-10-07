# Rivet

Rivet is a C library for embedded firmware, written in Rust. It is
`#![no_std]` with no heap of its own, exports the C functions firmware and the
C code it links call, and builds as a static archive that any C toolchain can
link.

Rivet is deliberately small. It grows by porting C library functions as
firmware needs them, each with its upstream tests.

## What it provides

| Header | Functions |
|---|---|
| `string.h` | `memcpy`, `memmove`, `memset`, `memcmp`, `memchr`, `strlen`, `strnlen`, `strcmp`, `strncmp`, `strcpy`, `stpcpy`, `strncpy`, `strcat`, `strncat`, `strchr`, `strrchr`, `strstr` |
| `ctype.h` | `isalnum`, `isalpha`, `isblank`, `iscntrl`, `isdigit`, `isgraph`, `islower`, `isprint`, `ispunct`, `isspace`, `isupper`, `isxdigit`, `tolower`, `toupper` |
| `stdio.h` | Streams (`FILE`, `stdout`, `stderr`) and `fputc`, `putc`, `fputs`, `fwrite`, `putchar`, `puts`, `printf`, `vprintf`, `fprintf`, `vfprintf`, `snprintf`, `vsnprintf`, `sprintf`, `vsprintf`: the integer, character, string, and pointer conversions with their flags, width, and precision; no floating point yet. See Streams |
| `stdlib.h` | `abort`, `atoi`, `strtol`, `strtoul`, `strtoll`, `strtoull`, `abs`, `labs`, `llabs`, `qsort`, `bsearch`; with the `cmpctmalloc` feature, `malloc`, `calloc`, `realloc`, `aligned_alloc`, `free`, `free_sized` |
| `time.h` | `time_t` (64 bits), `clock_t`, `struct tm`, and `struct timespec`; no functions |
| `sys/types.h` | `ssize_t`, `off_t`, `pid_t`, `mode_t`, `time_t`, `suseconds_t`, `useconds_t`, and `clock_t`, each guarded as newlib and glibc guard them |
| `sys/_timeval.h` | `struct timeval`, in the header newlib keeps it in, which Zephyr's socket and POSIX time headers include; guarded as newlib, Zephyr, and glibc guard it |
| `sys/cdefs.h` | `__BEGIN_DECLS` and `__END_DECLS` |
| `errno.h` | `errno`, through `__errno_location`, and newlib's error codes with its values, from `EPERM` to `EOWNERDEAD` |
| `rivet_stdio.h` | `rivet_stdio_set_hooks`, through which a kernel gives the streams a lock, so concurrent output calls do not interleave |
| `rivet_backtrace.h` | With the `backtrace` feature: `rivet_backtrace` and `rivet_backtrace_set_hooks`; `abort`, a failed `assert`, and a panic print a backtrace (see Backtraces) |
| `rivet_errno.h` | `rivet_errno_set_hooks`, through which a kernel names the running thread's `errno` (see Per-thread errno) |
| `rivet_heap.h` | With the `cmpctmalloc` feature: `RIVET_HEAP_ARENA`, `rivet_heap_init`, `rivet_heap_add`, `rivet_heap_alloc`, `rivet_heap_free`, `rivet_heap_set_hooks`, `rivet_heap_stats` |
| `math.h` | The types `float_t` and `double_t`, the constants `INFINITY`, `NAN`, and `HUGE_VAL`, and the classification macros `isnan`, `isinf`, `isfinite`, `isnormal`, and `signbit`, all through the compiler; no functions yet |
| `stdint.h` | The fixed-width, least, fast, pointer, and maximum integer types, their limits, and the constant macros, from the compiler's type macros, so a build without `-ffreestanding` has them too |
| `inttypes.h` | The `PRI*` and `SCN*` format macros, matched to the types the compiler, or the build, gives `stdint.h`; not `imaxabs`, `imaxdiv`, `strtoimax`, or `strtoumax` |
| `limits.h` | `CHAR_BIT`, `INT_MAX`, `LLONG_MAX`, and the rest of C11's integer limits, from the compiler; `MB_LEN_MAX` is 1 |
| `rivet_compiler.h` | Compiler attribute and builtin macros for C, C++, and assembly: `__PACKED`, `__ALIGNED`, `__NO_RETURN`, `__PRINTFLIKE`, `__SECTION`, `__WEAK`, `likely`, `add_overflow`, `__FALLTHROUGH`, the `__TA_*` thread-safety annotations, and the rest; nothing in the archive. The unreserved names, `likely`, `unlikely`, and the `*_overflow` macros, come only from including it directly: the C library's headers bring in reserved names alone |
| `assert.h` | `assert`, through `__assert_func` |
| `rivet_init.h` | `__libc_init_array`, for startup code that expects the C runtime's: it runs the constructors in `.preinit_array` and `.init_array` |

The headers are in `include/`.

## What the firmware provides

Rivet calls into its platform only through these symbols, which the firmware
defines:

| Symbol | Declared in | Used for |
|---|---|---|
| `ssize_t write(int fd, const void *buf, size_t count)` | `unistd.h` | `stdout` and `stderr` until firmware gives them other streams (fds 1 and 2), and assertion and panic messages |
| `void _exit(int status)` | `unistd.h` | `abort`, failed assertions, and panics |
| `__preinit_array_start`, `__preinit_array_end`, `__init_array_start`, `__init_array_end` | The linker script (see `rivet_init.h`) | `__libc_init_array`: the bounds of the constructor arrays, which the script also keeps |
| `malloc`, `free`, and the rest | `stdlib.h` | Without the `cmpctmalloc` feature, the firmware's allocator; with it, rivet's, over memory the firmware gives it (see Domain heaps) |

## Forkpoint properties

Built with the `forkpoint` feature, rivet reports properties to
[Forkpoint](https://github.com/misttech/forkpoint)'s emulator through its SDK
(`third_party/forkpoint-sdk`): the C contract each function relies on, such as
non-null pointers, no overlap for `memcpy`, and `ctype` arguments in range, and
that `abort`, a failed `assert`, or a panic is never reached. Forkpoint judges
them after every run, and lists each one in the image's catalog, reached or
not. Set `FPT_HOSTCALL_BASE` to the board's hostcall address when building.

`forkpoint-coverage` adds properties for edge paths, such as `memmove` over an
overlap, that a test suite should reach at least once. With `automemcpy`, they
include each size and alignment pattern the memory functions take on that
target. Firmware that never takes them would report them as failed, so turn it
on only for a test run.

Without either feature nothing is compiled in, not even the conditions.

The headers declare the functions that only read, such as `strlen` and
`isdigit`, `__PURE` or `__CONST`, so the compiler may drop a call whose result
goes unused. For a Forkpoint run, build the C code with `-DRIVET_FORKPOINT`:
the attributes then go away, and every call stays to check its properties.

## Build

Rivet needs Rust 1.85 or newer and the Rust target matching the firmware's C
ABI:

```sh
rustup target add riscv32imafc-unknown-none-elf
make staticlib TARGET=riscv32imafc-unknown-none-elf
```

This writes
`out/default/riscv32imafc-unknown-none-elf/release/librivet_libc.a`. Add
`include/` to the C include path and link the archive. A firmware build also
needs `clang` (or `CLANG=<path>`): the `printf` family's entry points are C,
since stable Rust cannot define a variadic function, and `src/build.rs`
compiles them into the archive. The Rust
target must match the C ABI: `riscv32imafc-unknown-none-elf` pairs with
`-march=rv32imafc -mabi=ilp32f`, and `thumbv7em-none-eabihf` with a
hard-float Cortex-M4F.

### Memory functions

By default, `memcpy`, `memmove`, `memset`, and `memcmp` choose an access
pattern by size, smallest first: overlapping blocks where the target loads
unaligned words in one instruction (Arm from ARMv7), and aligned words where it
does not (RV32). This is the `automemcpy` Cargo feature, on by default, named
after the paper the patterns come from, [automemcpy: A Framework for Automatic
Generation of Fundamental Memory Operations][automemcpy] (ISMM '21). Turn it
off for byte loops, the smallest code:

```sh
make staticlib TARGET=riscv32imafc-unknown-none-elf AUTOMEMCPY=0
```

`AUTOMEMCPY=0` also applies to `make test` and `make test-target(s)`. A Cargo
build turns the feature off with `--no-default-features`.

| Target | Feature on | Feature off |
|---|---|---|
| `thumbv7em-none-eabihf` | 1558 bytes | 514 bytes |
| `riscv32imafc-unknown-none-elf` | 770 bytes | 134 bytes |

The table gives the four functions' combined code size. With the feature on, a
copy of 16 bytes or more takes 2.5 to 7 times fewer instructions, counted in
QEMU, on both targets. On RV32 the gain is for copies between equally aligned
pointers; other copies, and those under 16 bytes, stay within a few
instructions of the byte loops. `make bench` measures this on each QEMU
platform and on the host; see
[the test suite's README](/test/README.md#benchmark).

Run `make` to list the other targets, and `make check` before sending a
change. See [AGENTS.md](/AGENTS.md) and the guides under
[docs/](/docs/development/languages/rust/README.md).

On bare-metal Arm the compiler turns copies and fills, in C and in rivet's
own Rust, into calls to the run-time ABI's `__aeabi_memcpy`,
`__aeabi_memmove`, `__aeabi_memset`, `__aeabi_memclr`, and their `4` and `8`
forms. Rivet defines them over these same functions, so every copy in the
firmware takes these patterns rather than the compiler runtime's generic
byte loops.

[automemcpy]: https://storage.googleapis.com/gweb-research2023-media/pubtools/6156.pdf

### Firmware core

By default rivet is built for its target's generic CPU, so one archive suits
every core of that target. `CPU=<name>` builds it for one core instead, and the
compiler then schedules, and may unroll, for that core's pipeline:

```sh
make staticlib TARGET=riscv32imafc-unknown-none-elf CPU=sifive-e76
```

The name is one of `rustc --print target-cpus --target <rust-target>`, and it
must be the core the firmware runs on: it can enable that core's own
extensions. `CPU=` also applies to `make test-target` and `make bench`. Cores
the compiler has no model for, such as most vendor RISC-V cores, gain nothing
from it.

It trades code size for speed, unevenly:

| Target and core | Instructions per call | Rivet's code size |
|---|---|---|
| `thumbv7em-none-eabihf`, `cortex-m4` | 1% fewer | 3% larger |
| `thumbv7em-none-eabihf`, `cortex-m7` | not measured | 8% larger |
| `riscv32imafc-unknown-none-elf`, `sifive-e76` | 18% fewer | 55% larger |

The instruction counts are `make bench`'s for the memory functions, geometric
means over every size and alignment it runs, against the generic build. On
RISC-V the gain comes mostly from loop unrolling, which the generic CPU leaves
off. QEMU counts instructions, not cycles, so time saved by scheduling for the
pipeline is not measured here.

### Domain heaps

The `cmpctmalloc` feature, off by default, builds an allocator ported from
Fuchsia's `cmpctmalloc`, and the `malloc` family over it.
Firmware that brings its own allocator leaves it off and links without
duplicate symbols.

Each heap owns the memory it is given and never grows. It cannot hand out
memory outside that memory, and every call takes a bounded time: `make bench
CMPCTMALLOC=1` measures a `malloc` and `free` pair at about 300 instructions on
Cortex-M4 and 470 on RV32, the same on an empty heap and a fragmented one.

A kernel that partitions RAM into domains gives each domain its own heap:

```c
#include <rivet_heap.h>

RIVET_HEAP_ARENA(ui_arena, 16 * 1024); /* in section .bss.rivet_heap.ui_arena */

rivet_heap *ui = rivet_heap_init(ui_arena, sizeof ui_arena);
```

```rust
rivet_libc::rivet_heap_arena!(UI, 16 * 1024); // in section .bss.rivet_heap.UI

let ui = UI.take().unwrap(); // the only handle, moved to the domain
```

- **Placement:** an arena's section is `.bss.rivet_heap.<name>`. Firmware's
  usual `*(.bss .bss.*)` rule zeroes it with the rest of `.bss`. To give a
  domain its own RAM range, a linker script places `*(.bss.rivet_heap.<name>)`
  there, ahead of that rule; one MPU region then covers the heap and its state.
- **Restart:** after a fault, `rivet_heap_init` on the same arena, or
  `DomainHeap::reset`, frees every allocation at once.
- **Which heap `malloc` uses:** the kernel registers `struct rivet_heap_hooks`
  with `rivet_heap_set_hooks`. Its `current` call names the running domain's
  heap, and its `lock` and `unlock` calls make a heap safe to share between
  threads. Without hooks, `malloc` uses the default heap, which
  `rivet_heap_add(base, size)` gives memory to, from one thread.
- **Overhead:** each heap keeps its state at the start of its arena:
  `RIVET_HEAP_OVERHEAD` bytes, 608 on 32-bit targets.

Build and test with it with `CMPCTMALLOC=1`:

```sh
make staticlib TARGET=thumbv7em-none-eabihf CMPCTMALLOC=1
make test test-targets CMPCTMALLOC=1
```

### Per-thread errno

`errno` is the `int` that `__errno_location` points at. Until the kernel says
otherwise that is one cell for the program, which is right for firmware with
one thread, and wrong as soon as two threads can be inside `strtol` or
`printf` at once. A kernel with threads registers the call that names the
running thread's own cell, and from then on every `errno` read and write goes
there, with nothing to save or restore on a switch:

```c
#include <rivet_errno.h>

static int *current_errno(void) { return &current_thread()->errno_cell; }
static const struct rivet_errno_hooks hooks = {current_errno};

rivet_errno_set_hooks(&hooks); /* once, before the threads start */
```

The cell can live in the thread's control block, or at an aligned base of its
stack, where `current_errno` is one mask of the stack pointer. The hook runs
wherever `errno` is set, so it must be safe in an interrupt handler if handlers
call functions that set `errno`; a null result falls back to the program's
cell. This is how newlib's `__errno` and NuttX's `__errno` work, with rivet's
`rivet_heap_set_hooks` shape instead of a symbol the firmware must define.

### Streams

A stream, `FILE`, is a write callback and the context it takes: there is no
file system, buffering, or input. Every output function ends in a stream's
callback. `stdout` and `stderr` start as streams over the firmware's `write`
to file descriptors 1 and 2, and firmware points either elsewhere, a log
buffer or another UART, by assigning it a stream before any thread prints:

```c
static int uart_write(void *uart, const char *s, size_t n) {
  uart_send(uart, s, n);
  return (int)n; /* or a negative value when it failed */
}

*stdout = (FILE){uart_write, &uart1};
fprintf(stderr, "boot: %d\n", reason);
```

The callback writes all `n` bytes and returns `n`, or returns a negative
value; rivet then reports `EOF`, or -1, with `errno` set to `EIO`. Rust code
writes to the same streams with `rivet_libc::print` and `eprint`, or through
`FILE`'s `fmt::Write`.

### Output from several threads

`printf` writes its result through a stream in several calls, so two threads
printing at once would interleave their bytes. A kernel with threads gives the
streams a lock, and every output function, `printf` through `fwrite`, holds it
for the whole of its output, whatever the stream:

```c
#include <rivet_stdio.h>

static void lock(void) { mutex_lock(&console_mutex); }
static void unlock(void) { mutex_unlock(&console_mutex); }
static const struct rivet_stdio_hooks hooks = {lock, unlock};

rivet_stdio_set_hooks(&hooks); /* once, before the threads start */
```

The lock is not taken recursively, and an interrupt handler that prints while
the thread it interrupted holds a mutex would deadlock: print from a handler
only with a lock that masks interrupts, or not at all. `snprintf` and its
siblings write to the caller's buffer and take no lock.

### Backtraces

With the `backtrace` feature, off by default, `abort`, a failed `assert`, and
a panic print the backtrace from their call site to standard error before the
program ends, and `rivet_backtrace()` prints one on request to `stderr`. They
print it as symbolizer markup, which `llvm-symbolizer` turns into function
names:

```sh
make staticlib TARGET=thumbv7em-none-eabihf BACKTRACE=1
llvm-symbolizer --filter-markup --debug-file-directory=<dir> < console.log
```

where `<dir>` holds the firmware's ELF file as
`.build-id/<first two hex digits>/<the rest>.debug`. Without the feature,
nothing of it is compiled, and the archive is the same as without it.

Frame 0 is the call site. The frames past it come from the frame records of
the code that called it, so that code keeps frame pointers
(`-fno-omit-frame-pointer`); `FRAME_POINTERS=1` keeps them in rivet's own code
too. GCC's code for Thumb does not keep its frame pointer at the frame record,
so the walk stops at the first such frame; stack words that point into the code
follow then, as `rivet: stack scan` candidates, which the reader judges.

The kernel tells rivet what it cannot know: which addresses are on the running
thread's stack, the image's build ID note, and its code range. The firmware
links with `--build-id` and its linker script keeps `.note.gnu.build-id`:

```c
#include <rivet_backtrace.h>

static struct rivet_backtrace_hooks hooks; /* kept, not copied */

hooks.is_on_stack = thread_stack_contains;
hooks.build_id_note = __build_id_start;
hooks.build_id_note_size = __build_id_end - __build_id_start;
hooks.code_start = __text_start;
hooks.code_size = __text_end - __text_start;
rivet_backtrace_set_hooks(&hooks);
```

Without hooks, only frame 0 prints.

## Changes

[CHANGELOG.md](/CHANGELOG.md) lists what each release changed for code that
builds against rivet, and what changed for firmware that pinned a commit.

## License

Copyright 2026 Mist Tecnologia LTDA. All rights reserved. Vendored code under
`third_party/` keeps its own license.
