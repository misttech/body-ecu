// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* struct timeval, in the header newlib keeps it in: Zephyr's socket types
 * (socket_types.h) and POSIX sys/time.h include <sys/_timeval.h> by that
 * name, and Zephyr ships its own only with its minimal libc. The struct is
 * guarded as newlib guards it (_TIMEVAL_DEFINED), as Zephyr's libc headers do
 * (_TIMEVAL_DECLARED), and as glibc does (__timeval_defined), so whichever
 * header reaches it first defines it, and the others skip it. No function
 * takes it here: a firmware's clock is its own. */

#ifndef _SYS__TIMEVAL_H_
#define _SYS__TIMEVAL_H_

#include <sys/types.h>

#if !defined(_TIMEVAL_DEFINED) && !defined(_TIMEVAL_DECLARED) && !defined(__timeval_defined)
#define _TIMEVAL_DEFINED
#define _TIMEVAL_DECLARED
#define __timeval_defined
struct timeval {
  time_t tv_sec;       /* Whole seconds. */
  suseconds_t tv_usec; /* Microseconds, 0 to 999999. */
};
#endif

#endif /* _SYS__TIMEVAL_H_ */
