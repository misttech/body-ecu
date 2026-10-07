// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* The BSD declaration brackets, for headers that include sys/cdefs.h for them.
 * Nothing else: its other names, such as __packed and __unused, collide with
 * the ones toolchain headers like Zephyr's define. rivet's own headers use
 * __BEGIN_CDECLS from rivet_compiler.h. */

#ifndef SYS_CDEFS_H
#define SYS_CDEFS_H

#ifndef __BEGIN_DECLS
#ifdef __cplusplus
#define __BEGIN_DECLS extern "C" {
#define __END_DECLS }
#else
#define __BEGIN_DECLS
#define __END_DECLS
#endif
#endif

#endif /* SYS_CDEFS_H */
