// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* The two calls this C library makes into the firmware, its board support
 * package. The firmware implements both. */

#ifndef UNISTD_H
#define UNISTD_H

#define __RIVET_RESERVED_ONLY
#include <rivet_compiler.h>
#undef __RIVET_RESERVED_ONLY
#include <stddef.h>
#include <sys/types.h>

__BEGIN_CDECLS

#define STDOUT_FILENO 1
#define STDERR_FILENO 2

/* Writes count bytes to a console; returns the number written. */
ssize_t write(int fd, const void *buf, size_t count);

/* Ends the program. It never returns. */
__NO_RETURN void _exit(int status);

__END_CDECLS

#endif /* UNISTD_H */
