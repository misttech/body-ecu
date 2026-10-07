// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* The fixed-width integer types, from the compiler's predefined macros, so a
 * build without -ffreestanding, whose compiler stdint.h would defer to the C
 * library's, gets them too. Every type comes from a __*_TYPE__ macro, and the
 * constant macros from __INTn_C where the compiler defines it, so a build that
 * redefines those before any header, as Zephyr's zephyr_stdint.h does, gets
 * types, constants, and limits that agree. */

#ifndef STDINT_H
#define STDINT_H

typedef __INT8_TYPE__ int8_t;
typedef __INT16_TYPE__ int16_t;
typedef __INT32_TYPE__ int32_t;
typedef __INT64_TYPE__ int64_t;
typedef __UINT8_TYPE__ uint8_t;
typedef __UINT16_TYPE__ uint16_t;
typedef __UINT32_TYPE__ uint32_t;
typedef __UINT64_TYPE__ uint64_t;

typedef __INT_LEAST8_TYPE__ int_least8_t;
typedef __INT_LEAST16_TYPE__ int_least16_t;
typedef __INT_LEAST32_TYPE__ int_least32_t;
typedef __INT_LEAST64_TYPE__ int_least64_t;
typedef __UINT_LEAST8_TYPE__ uint_least8_t;
typedef __UINT_LEAST16_TYPE__ uint_least16_t;
typedef __UINT_LEAST32_TYPE__ uint_least32_t;
typedef __UINT_LEAST64_TYPE__ uint_least64_t;
typedef __INT_FAST8_TYPE__ int_fast8_t;
typedef __INT_FAST16_TYPE__ int_fast16_t;
typedef __INT_FAST32_TYPE__ int_fast32_t;
typedef __INT_FAST64_TYPE__ int_fast64_t;
typedef __UINT_FAST8_TYPE__ uint_fast8_t;
typedef __UINT_FAST16_TYPE__ uint_fast16_t;
typedef __UINT_FAST32_TYPE__ uint_fast32_t;
typedef __UINT_FAST64_TYPE__ uint_fast64_t;

typedef __INTPTR_TYPE__ intptr_t;
typedef __UINTPTR_TYPE__ uintptr_t;
typedef __INTMAX_TYPE__ intmax_t;
typedef __UINTMAX_TYPE__ uintmax_t;

/* The constant macros: the compiler's __INTn_C where it has one, as GCC does,
 * else the literal with the compiler's suffix for the type, as clang has. */
#define __RIVET_PASTE(c, suffix) c##suffix
#define __RIVET_SUFFIX(c, suffix) __RIVET_PASTE(c, suffix)

#ifdef __INT8_C
#define INT8_C(c) __INT8_C(c)
#else
#define INT8_C(c) __RIVET_SUFFIX(c, __INT8_C_SUFFIX__)
#endif
#ifdef __INT16_C
#define INT16_C(c) __INT16_C(c)
#else
#define INT16_C(c) __RIVET_SUFFIX(c, __INT16_C_SUFFIX__)
#endif
#ifdef __INT32_C
#define INT32_C(c) __INT32_C(c)
#else
#define INT32_C(c) __RIVET_SUFFIX(c, __INT32_C_SUFFIX__)
#endif
#ifdef __INT64_C
#define INT64_C(c) __INT64_C(c)
#else
#define INT64_C(c) __RIVET_SUFFIX(c, __INT64_C_SUFFIX__)
#endif
#ifdef __UINT8_C
#define UINT8_C(c) __UINT8_C(c)
#else
#define UINT8_C(c) __RIVET_SUFFIX(c, __UINT8_C_SUFFIX__)
#endif
#ifdef __UINT16_C
#define UINT16_C(c) __UINT16_C(c)
#else
#define UINT16_C(c) __RIVET_SUFFIX(c, __UINT16_C_SUFFIX__)
#endif
#ifdef __UINT32_C
#define UINT32_C(c) __UINT32_C(c)
#else
#define UINT32_C(c) __RIVET_SUFFIX(c, __UINT32_C_SUFFIX__)
#endif
#ifdef __UINT64_C
#define UINT64_C(c) __UINT64_C(c)
#else
#define UINT64_C(c) __RIVET_SUFFIX(c, __UINT64_C_SUFFIX__)
#endif
#ifdef __INTMAX_C
#define INTMAX_C(c) __INTMAX_C(c)
#else
#define INTMAX_C(c) __RIVET_SUFFIX(c, __INTMAX_C_SUFFIX__)
#endif
#ifdef __UINTMAX_C
#define UINTMAX_C(c) __UINTMAX_C(c)
#else
#define UINTMAX_C(c) __RIVET_SUFFIX(c, __UINTMAX_C_SUFFIX__)
#endif

/* The limits of the exact-width types, through the constant macros, so each
 * has its type's promoted type. */
#define INT8_MAX 127
#define INT16_MAX 32767
#define INT32_MAX INT32_C(2147483647)
#define INT64_MAX INT64_C(9223372036854775807)
#define INT8_MIN (-INT8_MAX - 1)
#define INT16_MIN (-INT16_MAX - 1)
#define INT32_MIN (-INT32_MAX - 1)
#define INT64_MIN (-INT64_MAX - 1)
#define UINT8_MAX 255
#define UINT16_MAX 65535
#define UINT32_MAX UINT32_C(4294967295)
#define UINT64_MAX UINT64_C(18446744073709551615)

/* Every target with these types has the exact widths, so each least type is
 * its exact type. */
#define INT_LEAST8_MIN INT8_MIN
#define INT_LEAST8_MAX INT8_MAX
#define UINT_LEAST8_MAX UINT8_MAX
#define INT_LEAST16_MIN INT16_MIN
#define INT_LEAST16_MAX INT16_MAX
#define UINT_LEAST16_MAX UINT16_MAX
#define INT_LEAST32_MIN INT32_MIN
#define INT_LEAST32_MAX INT32_MAX
#define UINT_LEAST32_MAX UINT32_MAX
#define INT_LEAST64_MIN INT64_MIN
#define INT_LEAST64_MAX INT64_MAX
#define UINT_LEAST64_MAX UINT64_MAX

/* The fast types are as wide as the compiler makes them. */
#define INT_FAST8_MAX __INT_FAST8_MAX__
#define INT_FAST8_MIN (-INT_FAST8_MAX - 1)
#define UINT_FAST8_MAX __UINT_FAST8_MAX__
#define INT_FAST16_MAX __INT_FAST16_MAX__
#define INT_FAST16_MIN (-INT_FAST16_MAX - 1)
#define UINT_FAST16_MAX __UINT_FAST16_MAX__
#define INT_FAST32_MAX __INT_FAST32_MAX__
#define INT_FAST32_MIN (-INT_FAST32_MAX - 1)
#define UINT_FAST32_MAX __UINT_FAST32_MAX__
#define INT_FAST64_MAX __INT_FAST64_MAX__
#define INT_FAST64_MIN (-INT_FAST64_MAX - 1)
#define UINT_FAST64_MAX __UINT_FAST64_MAX__

#define INTPTR_MAX __INTPTR_MAX__
#define INTPTR_MIN (-INTPTR_MAX - 1)
#define UINTPTR_MAX __UINTPTR_MAX__
#define INTMAX_MAX __INTMAX_MAX__
#define INTMAX_MIN (-INTMAX_MAX - 1)
#define UINTMAX_MAX __UINTMAX_MAX__

#define PTRDIFF_MAX __PTRDIFF_MAX__
#define PTRDIFF_MIN (-PTRDIFF_MAX - 1)
#define SIZE_MAX __SIZE_MAX__
#define SIG_ATOMIC_MAX __SIG_ATOMIC_MAX__
#define SIG_ATOMIC_MIN (-SIG_ATOMIC_MAX - 1)
#define WCHAR_MAX __WCHAR_MAX__
#if defined(__WCHAR_MIN__)
#define WCHAR_MIN __WCHAR_MIN__
#elif defined(__WCHAR_UNSIGNED__)
#define WCHAR_MIN 0U
#else
#define WCHAR_MIN (-WCHAR_MAX - 1)
#endif
#define WINT_MAX __WINT_MAX__
#if defined(__WINT_MIN__)
#define WINT_MIN __WINT_MIN__
#elif defined(__WINT_UNSIGNED__)
#define WINT_MIN 0U
#else
#define WINT_MIN (-WINT_MAX - 1)
#endif

/* C23's widths, or on request, for the types whose width the compiler names. */
#if defined(__STDC_WANT_IEC_60559_BFP_EXT__) || \
    (defined(__STDC_VERSION__) && __STDC_VERSION__ > 201710L)
#define INT8_WIDTH 8
#define UINT8_WIDTH 8
#define INT_LEAST8_WIDTH 8
#define UINT_LEAST8_WIDTH 8
#define INT16_WIDTH 16
#define UINT16_WIDTH 16
#define INT_LEAST16_WIDTH 16
#define UINT_LEAST16_WIDTH 16
#define INT32_WIDTH 32
#define UINT32_WIDTH 32
#define INT_LEAST32_WIDTH 32
#define UINT_LEAST32_WIDTH 32
#define INT64_WIDTH 64
#define UINT64_WIDTH 64
#define INT_LEAST64_WIDTH 64
#define UINT_LEAST64_WIDTH 64
#define INTPTR_WIDTH __INTPTR_WIDTH__
#define UINTPTR_WIDTH __INTPTR_WIDTH__
#define INTMAX_WIDTH __INTMAX_WIDTH__
#define UINTMAX_WIDTH __INTMAX_WIDTH__
#define PTRDIFF_WIDTH __PTRDIFF_WIDTH__
#define SIZE_WIDTH __SIZE_WIDTH__
#define SIG_ATOMIC_WIDTH __SIG_ATOMIC_WIDTH__
#define WCHAR_WIDTH __WCHAR_WIDTH__
#define WINT_WIDTH __WINT_WIDTH__
#endif

#endif /* STDINT_H */
