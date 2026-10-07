// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* Startup: the call a C runtime's start-up code makes before main. */

#ifndef RIVET_INIT_H
#define RIVET_INIT_H

#define __RIVET_RESERVED_ONLY
#include <rivet_compiler.h>
#undef __RIVET_RESERVED_ONLY

__BEGIN_CDECLS
/* Calls the constructors the program was linked with: every function in
 * .preinit_array, then every function in .init_array, each array in order.
 * Call it once, after .data and .bss are set up and before main. It does not
 * call _init, which comes from crti.o: firmware linked without the C start
 * files has none.
 *
 * The linker script bounds the two arrays with these symbols, and keeps them:
 *
 *   .preinit_array : ALIGN(4) {
 *     __preinit_array_start = .;
 *     KEEP(*(.preinit_array))
 *     __preinit_array_end = .;
 *   }
 *   .init_array : ALIGN(4) {
 *     __init_array_start = .;
 *     KEEP(*(SORT_BY_INIT_PRIORITY(.init_array.*)))
 *     KEEP(*(.init_array))
 *     __init_array_end = .;
 *   }
 */
void __libc_init_array(void);

__END_CDECLS

#endif /* RIVET_INIT_H */
