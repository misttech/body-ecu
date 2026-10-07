// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* Where errno lives, from src/rivet_errno/. errno (see errno.h) is the int
 * that __errno_location() points at: one cell for the program, until a
 * kernel with threads registers the call that names the running thread's
 * own, in its control block or at the base of its stack. From then on every
 * errno read and write goes there, and nothing is saved or restored on a
 * switch. */

#ifndef RIVET_ERRNO_H
#define RIVET_ERRNO_H

#define __RIVET_RESERVED_ONLY
#include <rivet_compiler.h>
#undef __RIVET_RESERVED_ONLY

__BEGIN_CDECLS
/* The calls a kernel makes available for errno. Each may be NULL. */
struct rivet_errno_hooks {
  /* The running thread's errno, or NULL for the program's one cell. It runs
   * wherever errno is set, so it must be safe in an interrupt handler if
   * handlers call functions that set errno. */
  int *(*current)(void);
};

/* Registers `hooks`, which must stay valid for the rest of the program: a call
 * already in progress may still use hooks that a later call replaced. NULL
 * keeps the current ones. */
void rivet_errno_set_hooks(const struct rivet_errno_hooks *hooks);

__END_CDECLS

#endif /* RIVET_ERRNO_H */
