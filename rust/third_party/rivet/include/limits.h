// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* The integer limits of limits.h. Each one is a compiler builtin for this
 * target, so nothing here needs the archive. _LIBC_LIMITS_H_ is the macro
 * GCC's own limits.h checks before it searches for a C library header;
 * defining it here is that header, whether or not the build defined it
 * already. MB_LEN_MAX is 1, the "C" locale. */

#ifndef LIMITS_H
#define LIMITS_H

#ifndef _LIBC_LIMITS_H_
#define _LIBC_LIMITS_H_
#endif

#define CHAR_BIT __CHAR_BIT__

#define MB_LEN_MAX 1

#define SCHAR_MAX __SCHAR_MAX__
#define SCHAR_MIN (-SCHAR_MAX - 1)
#if __SCHAR_MAX__ == __INT_MAX__
#define UCHAR_MAX (SCHAR_MAX * 2U + 1U)
#else
#define UCHAR_MAX (SCHAR_MAX * 2 + 1)
#endif

#ifdef __CHAR_UNSIGNED__
#if __SCHAR_MAX__ == __INT_MAX__
#define CHAR_MIN 0U
#else
#define CHAR_MIN 0
#endif
#define CHAR_MAX UCHAR_MAX
#else
#define CHAR_MIN SCHAR_MIN
#define CHAR_MAX SCHAR_MAX
#endif

#define SHRT_MAX __SHRT_MAX__
#define SHRT_MIN (-SHRT_MAX - 1)
#if __SHRT_MAX__ == __INT_MAX__
#define USHRT_MAX (SHRT_MAX * 2U + 1U)
#else
#define USHRT_MAX (SHRT_MAX * 2 + 1)
#endif

#define INT_MAX __INT_MAX__
#define INT_MIN (-INT_MAX - 1)
#define UINT_MAX (INT_MAX * 2U + 1U)

#define LONG_MAX __LONG_MAX__
#define LONG_MIN (-LONG_MAX - 1L)
#define ULONG_MAX (LONG_MAX * 2UL + 1UL)

#if defined(__STDC_VERSION__) && __STDC_VERSION__ >= 199901L
#define LLONG_MAX __LONG_LONG_MAX__
#define LLONG_MIN (-LLONG_MAX - 1LL)
#define ULLONG_MAX (LLONG_MAX * 2ULL + 1ULL)
#endif

/* The GNU names, which GCC's limits.h also defines outside strict ISO C. */
#if !defined(__STRICT_ANSI__)
#define LONG_LONG_MAX __LONG_LONG_MAX__
#define LONG_LONG_MIN (-LONG_LONG_MAX - 1LL)
#define ULONG_LONG_MAX (LONG_LONG_MAX * 2ULL + 1ULL)
#endif

/* The widths, for C23 or on request. Every char type is CHAR_BIT wide; long
 * long's builtin is __LLONG_WIDTH__ in clang and __LONG_LONG_WIDTH__ in GCC. */
#if defined(__STDC_WANT_IEC_60559_BFP_EXT__) || \
    (defined(__STDC_VERSION__) && __STDC_VERSION__ > 201710L)
#define CHAR_WIDTH CHAR_BIT
#define SCHAR_WIDTH CHAR_BIT
#define UCHAR_WIDTH CHAR_BIT
#define SHRT_WIDTH __SHRT_WIDTH__
#define USHRT_WIDTH __SHRT_WIDTH__
#define INT_WIDTH __INT_WIDTH__
#define UINT_WIDTH __INT_WIDTH__
#define LONG_WIDTH __LONG_WIDTH__
#define ULONG_WIDTH __LONG_WIDTH__
#ifdef __LLONG_WIDTH__
#define LLONG_WIDTH __LLONG_WIDTH__
#else
#define LLONG_WIDTH __LONG_LONG_WIDTH__
#endif
#define ULLONG_WIDTH LLONG_WIDTH
#endif

/* C23's bool and _BitInt limits. The header claims C23's version only where
 * the compiler has _BitInt, which BITINT_MAXWIDTH describes. */
#if defined(__STDC_VERSION__) && __STDC_VERSION__ > 201710L
#define BOOL_MAX 1
#define BOOL_WIDTH 1
#ifdef __BITINT_MAXWIDTH__
#define BITINT_MAXWIDTH __BITINT_MAXWIDTH__
#define __STDC_VERSION_LIMITS_H__ 202311L
#endif
#endif

#endif /* LIMITS_H */
