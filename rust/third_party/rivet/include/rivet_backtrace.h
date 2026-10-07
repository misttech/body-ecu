// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* Backtraces, with rivet's backtrace feature, from src/rivet_backtrace/.
 *
 * abort, a failed assert, and a panic print the backtrace from their call site
 * to standard error, and rivet_backtrace prints one on request to stderr, as
 * symbolizer markup:
 *
 *   {{{reset}}}
 *   {{{module:0:firmware:elf:<build ID>}}}
 *   {{{mmap:<code start>:<code size>:load:0:rx:<code start>}}}
 *   {{{bt:0:0x80001234:ra}}}
 *   ...
 *
 * which `llvm-symbolizer --filter-markup` turns into function names and lines.
 * Frame 0 is the call site. Frames past it come from the frame records of the
 * code that called it, so that code must keep frame pointers
 * (-fno-omit-frame-pointer); when the records stop early, as GCC's Thumb code
 * makes them do, stack words that point into the code range follow as
 * "rivet: stack scan" candidates. Without hooks, only frame 0 prints. */

#ifndef RIVET_BACKTRACE_H
#define RIVET_BACKTRACE_H

#define __RIVET_RESERVED_ONLY
#include <rivet_compiler.h>
#undef __RIVET_RESERVED_ONLY
#include <stddef.h>

__BEGIN_CDECLS

/* What a kernel tells rivet's backtraces. Each may be NULL or 0. */
struct rivet_backtrace_hooks {
  /* Nonzero if the word at addr lies in the running thread's stack. rivet asks
   * for each word it reads, both words of a frame record included. Without it
   * no frame record or stack word is read. It must be safe to call from a
   * fault or a panic. */
  int (*is_on_stack)(const void *addr);
  /* The image's GNU build ID note, as the linker emits .note.gnu.build-id
   * (link with --build-id), and its size in bytes. */
  const void *build_id_note;
  size_t build_id_note_size;
  /* The image's code, as loaded, and its size in bytes. */
  const void *code_start;
  size_t code_size;
  /* The module's name in the markup; NULL for "firmware". */
  const char *module_name;
};

/* Registers `hooks`, which must stay valid for the rest of the program, with
 * everything it points at. NULL keeps the current ones. */
void rivet_backtrace_set_hooks(const struct rivet_backtrace_hooks *hooks);

/* Prints the backtrace from the call site to stderr. */
void rivet_backtrace(void);

__END_CDECLS

#endif /* RIVET_BACKTRACE_H */
