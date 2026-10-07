// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* The standard I/O lock, from src/rivet_stdio/. printf writes its result
 * through a stream in several calls, so two threads printing at once would
 * interleave their bytes. A kernel with threads registers a lock, and every
 * output function, printf through fwrite, holds it for the whole of its
 * output, whatever the stream. snprintf and its siblings write to the
 * caller's buffer and take no lock. */

#ifndef RIVET_STDIO_H
#define RIVET_STDIO_H

#define __RIVET_RESERVED_ONLY
#include <rivet_compiler.h>
#undef __RIVET_RESERVED_ONLY

__BEGIN_CDECLS

/* The calls a kernel makes available for the streams. Each may be NULL.
 * The lock is not taken recursively. An interrupt handler that prints needs a
 * lock that masks interrupts, or it deadlocks against the thread it interrupted. */
struct rivet_stdio_hooks {
  void (*lock)(void);
  void (*unlock)(void);
};

/* Registers `hooks`, which must stay valid for the rest of the program: a call
 * already in progress may still use hooks that a later call replaced. NULL
 * keeps the current ones. */
void rivet_stdio_set_hooks(const struct rivet_stdio_hooks *hooks);

__END_CDECLS

#endif /* RIVET_STDIO_H */
