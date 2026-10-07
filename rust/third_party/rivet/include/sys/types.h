// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* The POSIX scalar types a freestanding program meets: ssize_t, off_t, pid_t,
 * mode_t, time_t, suseconds_t, useconds_t, and clock_t. Each is guarded both
 * the way newlib's headers guard it (_X_DECLARED) and the way glibc's and
 * Zephyr's do (__x_defined), so whichever header a build reaches first defines
 * it, and the other skips it. pid_t is int, as Zephyr's posix_types.h defines
 * it without a guard. */

#ifndef SYS_TYPES_H
#define SYS_TYPES_H

#include <stddef.h>

/* ssize_t is the signed type of size_t's width: while unsigned stands for
 * signed, __SIZE_TYPE__ names it. */
#if !defined(_SSIZE_T_DECLARED) && !defined(__ssize_t_defined)
#define _SSIZE_T_DECLARED
#define __ssize_t_defined
#ifdef __clang__
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wkeyword-macro"
#endif
#pragma push_macro("unsigned")
#undef unsigned
#define unsigned signed
typedef __SIZE_TYPE__ ssize_t;
#undef unsigned
#pragma pop_macro("unsigned")
#ifdef __clang__
#pragma clang diagnostic pop
#endif
#endif

#if !defined(_OFF_T_DECLARED) && !defined(__off_t_defined)
#define _OFF_T_DECLARED
#define __off_t_defined
typedef long off_t;
#endif

#if !defined(_PID_T_DECLARED) && !defined(__pid_t_defined)
#define _PID_T_DECLARED
#define __pid_t_defined
typedef int pid_t;
#endif

#if !defined(_MODE_T_DECLARED) && !defined(__mode_t_defined)
#define _MODE_T_DECLARED
#define __mode_t_defined
typedef unsigned int mode_t;
#endif

/* time_t is 64 bits, so it outlasts 2038. */
#if !defined(_TIME_T_DECLARED) && !defined(__time_t_defined)
#define _TIME_T_DECLARED
#define __time_t_defined
typedef __INT64_TYPE__ time_t;
#endif

#if !defined(_SUSECONDS_T_DECLARED) && !defined(__suseconds_t_defined)
#define _SUSECONDS_T_DECLARED
#define __suseconds_t_defined
typedef long suseconds_t;
#endif

#if !defined(_USECONDS_T_DECLARED) && !defined(__useconds_t_defined)
#define _USECONDS_T_DECLARED
#define __useconds_t_defined
typedef unsigned long useconds_t;
#endif

#if !defined(_CLOCK_T_DECLARED) && !defined(__clock_t_defined)
#define _CLOCK_T_DECLARED
#define __clock_t_defined
typedef unsigned long clock_t;
#endif

#endif /* SYS_TYPES_H */
