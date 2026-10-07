// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* Heaps for memory domains, with rivet's cmpctmalloc feature. Each heap owns the
 * memory it is given and never grows: it hands out nothing outside that memory,
 * and every call takes bounded time. malloc() and its family allocate from the
 * running domain's heap, which the kernel names through rivet_heap_set_hooks(),
 * or from the default heap, which rivet_heap_add() gives memory to. */

#ifndef RIVET_HEAP_H
#define RIVET_HEAP_H

#define __RIVET_RESERVED_ONLY
#include <rivet_compiler.h>
#undef __RIVET_RESERVED_ONLY
#include <stddef.h>

__BEGIN_CDECLS

/* A heap. Its state lives at the start of its arena. */
typedef struct rivet_heap rivet_heap;

/* Bytes of an arena that hold its heap's own state rather than allocations. */
#define RIVET_HEAP_OVERHEAD (144 * sizeof(void *) + 32)

/* Declares `name`, memory for a heap of about `bytes` bytes, in the linker section
 * `.bss.rivet_heap.<name>`. Firmware's usual `*(.bss .bss.*)` rule zeroes and
 * places it with the rest of .bss; a kernel that gives a domain its own RAM range
 * places `*(.bss.rivet_heap.<name>)` there instead, ahead of that rule, and
 * covers it with one MPU region. Such a section may be NOLOAD: rivet_heap_init()
 * writes everything the heap reads. */
#define RIVET_HEAP_ARENA(name, bytes)                                   \
  static unsigned char name[(bytes) + RIVET_HEAP_OVERHEAD] __ALIGNED(8) \
      __SECTION(".bss.rivet_heap." #name) __ALWAYS_EMIT

/* Builds a heap in the `size` bytes at `arena` and returns it, or NULL when they
 * cannot hold it. Called again on the same arena, it frees every allocation, as
 * restarting a domain needs. */
__WARN_UNUSED_RESULT rivet_heap *rivet_heap_init(void *arena, size_t size);

/* Gives the `size` bytes at `base` to the default heap. Returns 0, or -1 when the
 * default heap already has four regions or the memory is too small to use. */
int rivet_heap_add(void *base, size_t size);

/* Allocates `size` bytes aligned to `align`, a power of two or 0 for malloc's
 * alignment, from `heap`, or the running domain's heap when it is NULL. */
__MALLOC __ALLOC_SIZE(2) void *rivet_heap_alloc(rivet_heap *heap, size_t size, size_t align);

/* Frees `ptr`, an allocation of `heap`, or of the running domain's heap when it
 * is NULL. */
void rivet_heap_free(rivet_heap *heap, void *ptr);

/* The calls a kernel makes available to rivet's heaps. Each may be NULL. */
struct rivet_heap_hooks {
  /* The running domain's heap, or NULL for the default heap. */
  rivet_heap *(*current)(void);
  /* Take and release a heap's lock: a domain mutex, or a critical section.
   * Without them, each heap must be used from one thread at a time. */
  void (*lock)(rivet_heap *heap);
  void (*unlock)(rivet_heap *heap);
};

/* Registers `hooks`, which must stay valid for the rest of the program: a call
 * already in progress may still use hooks that a later call replaced. */
void rivet_heap_set_hooks(const struct rivet_heap_hooks *hooks);

/* A heap's counters. */
struct rivet_heap_stats {
  size_t size;      /* Bytes the heap's memory holds. */
  size_t free;      /* Bytes free, headers included. */
  size_t peak_used; /* The most bytes in use at once since the heap was built. */
  size_t failures;  /* Allocations that found no free area large enough. */
};

/* Writes the counters of `heap`, or of the running domain's heap when it is NULL. */
void rivet_heap_stats(rivet_heap *heap, struct rivet_heap_stats *stats);

__END_CDECLS

#endif /* RIVET_HEAP_H */
