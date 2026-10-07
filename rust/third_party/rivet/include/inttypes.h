// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* The printf and scanf length modifiers for stdint.h's types. Which modifier a
 * type needs depends on the type the compiler gave it: int32_t is long under
 * GCC for Arm and int under clang, and a build may redefine the types first,
 * as Zephyr does. So each modifier is worked out here from the type's
 * __*_TYPE__ macro: while the keywords below stand for numbers, the macro
 * becomes a sum that names the type, and the sum picks the modifier.
 *
 * Only the macros are here: imaxabs, imaxdiv, strtoimax, strtoumax, and the
 * wide conversions are not provided. */

#ifndef INTTYPES_H
#define INTTYPES_H

#include <stdint.h>

#ifdef __clang__
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wkeyword-macro"
#endif
#pragma push_macro("signed")
#pragma push_macro("unsigned")
#pragma push_macro("char")
#pragma push_macro("short")
#pragma push_macro("int")
#pragma push_macro("long")
#undef signed
#undef unsigned
#undef char
#undef short
#undef int
#undef long
/* char is 1, short 2, int 4, and each long 8, so signed char sums to 1, short
 * or short int to 2 or 6, int to 4, long or long int to 8 or 12, and long long
 * or long long int to 16 or 20. */
#define signed +0
#define unsigned +0
#define char +1
#define short +2
#define int +4
#define long +8

#if (__INT8_TYPE__) == 1
#define __RIVET_LEN_8 "hh"
#elif (__INT8_TYPE__) == 2 || (__INT8_TYPE__) == 6
#define __RIVET_LEN_8 "h"
#elif (__INT8_TYPE__) == 4
#define __RIVET_LEN_8 ""
#elif (__INT8_TYPE__) == 8 || (__INT8_TYPE__) == 12
#define __RIVET_LEN_8 "l"
#elif (__INT8_TYPE__) == 16 || (__INT8_TYPE__) == 20
#define __RIVET_LEN_8 "ll"
#else
#error "inttypes.h: no length modifier for int8_t"
#endif
#if (__INT16_TYPE__) == 1
#define __RIVET_LEN_16 "hh"
#elif (__INT16_TYPE__) == 2 || (__INT16_TYPE__) == 6
#define __RIVET_LEN_16 "h"
#elif (__INT16_TYPE__) == 4
#define __RIVET_LEN_16 ""
#elif (__INT16_TYPE__) == 8 || (__INT16_TYPE__) == 12
#define __RIVET_LEN_16 "l"
#elif (__INT16_TYPE__) == 16 || (__INT16_TYPE__) == 20
#define __RIVET_LEN_16 "ll"
#else
#error "inttypes.h: no length modifier for int16_t"
#endif
#if (__INT32_TYPE__) == 1
#define __RIVET_LEN_32 "hh"
#elif (__INT32_TYPE__) == 2 || (__INT32_TYPE__) == 6
#define __RIVET_LEN_32 "h"
#elif (__INT32_TYPE__) == 4
#define __RIVET_LEN_32 ""
#elif (__INT32_TYPE__) == 8 || (__INT32_TYPE__) == 12
#define __RIVET_LEN_32 "l"
#elif (__INT32_TYPE__) == 16 || (__INT32_TYPE__) == 20
#define __RIVET_LEN_32 "ll"
#else
#error "inttypes.h: no length modifier for int32_t"
#endif
#if (__INT64_TYPE__) == 1
#define __RIVET_LEN_64 "hh"
#elif (__INT64_TYPE__) == 2 || (__INT64_TYPE__) == 6
#define __RIVET_LEN_64 "h"
#elif (__INT64_TYPE__) == 4
#define __RIVET_LEN_64 ""
#elif (__INT64_TYPE__) == 8 || (__INT64_TYPE__) == 12
#define __RIVET_LEN_64 "l"
#elif (__INT64_TYPE__) == 16 || (__INT64_TYPE__) == 20
#define __RIVET_LEN_64 "ll"
#else
#error "inttypes.h: no length modifier for int64_t"
#endif
#if (__INT_LEAST8_TYPE__) == 1
#define __RIVET_LEN_LEAST8 "hh"
#elif (__INT_LEAST8_TYPE__) == 2 || (__INT_LEAST8_TYPE__) == 6
#define __RIVET_LEN_LEAST8 "h"
#elif (__INT_LEAST8_TYPE__) == 4
#define __RIVET_LEN_LEAST8 ""
#elif (__INT_LEAST8_TYPE__) == 8 || (__INT_LEAST8_TYPE__) == 12
#define __RIVET_LEN_LEAST8 "l"
#elif (__INT_LEAST8_TYPE__) == 16 || (__INT_LEAST8_TYPE__) == 20
#define __RIVET_LEN_LEAST8 "ll"
#else
#error "inttypes.h: no length modifier for int_least8_t"
#endif
#if (__INT_LEAST16_TYPE__) == 1
#define __RIVET_LEN_LEAST16 "hh"
#elif (__INT_LEAST16_TYPE__) == 2 || (__INT_LEAST16_TYPE__) == 6
#define __RIVET_LEN_LEAST16 "h"
#elif (__INT_LEAST16_TYPE__) == 4
#define __RIVET_LEN_LEAST16 ""
#elif (__INT_LEAST16_TYPE__) == 8 || (__INT_LEAST16_TYPE__) == 12
#define __RIVET_LEN_LEAST16 "l"
#elif (__INT_LEAST16_TYPE__) == 16 || (__INT_LEAST16_TYPE__) == 20
#define __RIVET_LEN_LEAST16 "ll"
#else
#error "inttypes.h: no length modifier for int_least16_t"
#endif
#if (__INT_LEAST32_TYPE__) == 1
#define __RIVET_LEN_LEAST32 "hh"
#elif (__INT_LEAST32_TYPE__) == 2 || (__INT_LEAST32_TYPE__) == 6
#define __RIVET_LEN_LEAST32 "h"
#elif (__INT_LEAST32_TYPE__) == 4
#define __RIVET_LEN_LEAST32 ""
#elif (__INT_LEAST32_TYPE__) == 8 || (__INT_LEAST32_TYPE__) == 12
#define __RIVET_LEN_LEAST32 "l"
#elif (__INT_LEAST32_TYPE__) == 16 || (__INT_LEAST32_TYPE__) == 20
#define __RIVET_LEN_LEAST32 "ll"
#else
#error "inttypes.h: no length modifier for int_least32_t"
#endif
#if (__INT_LEAST64_TYPE__) == 1
#define __RIVET_LEN_LEAST64 "hh"
#elif (__INT_LEAST64_TYPE__) == 2 || (__INT_LEAST64_TYPE__) == 6
#define __RIVET_LEN_LEAST64 "h"
#elif (__INT_LEAST64_TYPE__) == 4
#define __RIVET_LEN_LEAST64 ""
#elif (__INT_LEAST64_TYPE__) == 8 || (__INT_LEAST64_TYPE__) == 12
#define __RIVET_LEN_LEAST64 "l"
#elif (__INT_LEAST64_TYPE__) == 16 || (__INT_LEAST64_TYPE__) == 20
#define __RIVET_LEN_LEAST64 "ll"
#else
#error "inttypes.h: no length modifier for int_least64_t"
#endif
#if (__INT_FAST8_TYPE__) == 1
#define __RIVET_LEN_FAST8 "hh"
#elif (__INT_FAST8_TYPE__) == 2 || (__INT_FAST8_TYPE__) == 6
#define __RIVET_LEN_FAST8 "h"
#elif (__INT_FAST8_TYPE__) == 4
#define __RIVET_LEN_FAST8 ""
#elif (__INT_FAST8_TYPE__) == 8 || (__INT_FAST8_TYPE__) == 12
#define __RIVET_LEN_FAST8 "l"
#elif (__INT_FAST8_TYPE__) == 16 || (__INT_FAST8_TYPE__) == 20
#define __RIVET_LEN_FAST8 "ll"
#else
#error "inttypes.h: no length modifier for int_fast8_t"
#endif
#if (__INT_FAST16_TYPE__) == 1
#define __RIVET_LEN_FAST16 "hh"
#elif (__INT_FAST16_TYPE__) == 2 || (__INT_FAST16_TYPE__) == 6
#define __RIVET_LEN_FAST16 "h"
#elif (__INT_FAST16_TYPE__) == 4
#define __RIVET_LEN_FAST16 ""
#elif (__INT_FAST16_TYPE__) == 8 || (__INT_FAST16_TYPE__) == 12
#define __RIVET_LEN_FAST16 "l"
#elif (__INT_FAST16_TYPE__) == 16 || (__INT_FAST16_TYPE__) == 20
#define __RIVET_LEN_FAST16 "ll"
#else
#error "inttypes.h: no length modifier for int_fast16_t"
#endif
#if (__INT_FAST32_TYPE__) == 1
#define __RIVET_LEN_FAST32 "hh"
#elif (__INT_FAST32_TYPE__) == 2 || (__INT_FAST32_TYPE__) == 6
#define __RIVET_LEN_FAST32 "h"
#elif (__INT_FAST32_TYPE__) == 4
#define __RIVET_LEN_FAST32 ""
#elif (__INT_FAST32_TYPE__) == 8 || (__INT_FAST32_TYPE__) == 12
#define __RIVET_LEN_FAST32 "l"
#elif (__INT_FAST32_TYPE__) == 16 || (__INT_FAST32_TYPE__) == 20
#define __RIVET_LEN_FAST32 "ll"
#else
#error "inttypes.h: no length modifier for int_fast32_t"
#endif
#if (__INT_FAST64_TYPE__) == 1
#define __RIVET_LEN_FAST64 "hh"
#elif (__INT_FAST64_TYPE__) == 2 || (__INT_FAST64_TYPE__) == 6
#define __RIVET_LEN_FAST64 "h"
#elif (__INT_FAST64_TYPE__) == 4
#define __RIVET_LEN_FAST64 ""
#elif (__INT_FAST64_TYPE__) == 8 || (__INT_FAST64_TYPE__) == 12
#define __RIVET_LEN_FAST64 "l"
#elif (__INT_FAST64_TYPE__) == 16 || (__INT_FAST64_TYPE__) == 20
#define __RIVET_LEN_FAST64 "ll"
#else
#error "inttypes.h: no length modifier for int_fast64_t"
#endif
#if (__INTMAX_TYPE__) == 1
#define __RIVET_LEN_MAX "hh"
#elif (__INTMAX_TYPE__) == 2 || (__INTMAX_TYPE__) == 6
#define __RIVET_LEN_MAX "h"
#elif (__INTMAX_TYPE__) == 4
#define __RIVET_LEN_MAX ""
#elif (__INTMAX_TYPE__) == 8 || (__INTMAX_TYPE__) == 12
#define __RIVET_LEN_MAX "l"
#elif (__INTMAX_TYPE__) == 16 || (__INTMAX_TYPE__) == 20
#define __RIVET_LEN_MAX "ll"
#else
#error "inttypes.h: no length modifier for intmax_t"
#endif
#if (__INTPTR_TYPE__) == 1
#define __RIVET_LEN_PTR "hh"
#elif (__INTPTR_TYPE__) == 2 || (__INTPTR_TYPE__) == 6
#define __RIVET_LEN_PTR "h"
#elif (__INTPTR_TYPE__) == 4
#define __RIVET_LEN_PTR ""
#elif (__INTPTR_TYPE__) == 8 || (__INTPTR_TYPE__) == 12
#define __RIVET_LEN_PTR "l"
#elif (__INTPTR_TYPE__) == 16 || (__INTPTR_TYPE__) == 20
#define __RIVET_LEN_PTR "ll"
#else
#error "inttypes.h: no length modifier for intptr_t"
#endif

#undef signed
#undef unsigned
#undef char
#undef short
#undef int
#undef long
#pragma pop_macro("signed")
#pragma pop_macro("unsigned")
#pragma pop_macro("char")
#pragma pop_macro("short")
#pragma pop_macro("int")
#pragma pop_macro("long")
#ifdef __clang__
#pragma clang diagnostic pop
#endif

#define PRId8 __RIVET_LEN_8 "d"
#define PRId16 __RIVET_LEN_16 "d"
#define PRId32 __RIVET_LEN_32 "d"
#define PRId64 __RIVET_LEN_64 "d"
#define PRIdLEAST8 __RIVET_LEN_LEAST8 "d"
#define PRIdLEAST16 __RIVET_LEN_LEAST16 "d"
#define PRIdLEAST32 __RIVET_LEN_LEAST32 "d"
#define PRIdLEAST64 __RIVET_LEN_LEAST64 "d"
#define PRIdFAST8 __RIVET_LEN_FAST8 "d"
#define PRIdFAST16 __RIVET_LEN_FAST16 "d"
#define PRIdFAST32 __RIVET_LEN_FAST32 "d"
#define PRIdFAST64 __RIVET_LEN_FAST64 "d"
#define PRIdMAX __RIVET_LEN_MAX "d"
#define PRIdPTR __RIVET_LEN_PTR "d"

#define PRIi8 __RIVET_LEN_8 "i"
#define PRIi16 __RIVET_LEN_16 "i"
#define PRIi32 __RIVET_LEN_32 "i"
#define PRIi64 __RIVET_LEN_64 "i"
#define PRIiLEAST8 __RIVET_LEN_LEAST8 "i"
#define PRIiLEAST16 __RIVET_LEN_LEAST16 "i"
#define PRIiLEAST32 __RIVET_LEN_LEAST32 "i"
#define PRIiLEAST64 __RIVET_LEN_LEAST64 "i"
#define PRIiFAST8 __RIVET_LEN_FAST8 "i"
#define PRIiFAST16 __RIVET_LEN_FAST16 "i"
#define PRIiFAST32 __RIVET_LEN_FAST32 "i"
#define PRIiFAST64 __RIVET_LEN_FAST64 "i"
#define PRIiMAX __RIVET_LEN_MAX "i"
#define PRIiPTR __RIVET_LEN_PTR "i"

#define PRIo8 __RIVET_LEN_8 "o"
#define PRIo16 __RIVET_LEN_16 "o"
#define PRIo32 __RIVET_LEN_32 "o"
#define PRIo64 __RIVET_LEN_64 "o"
#define PRIoLEAST8 __RIVET_LEN_LEAST8 "o"
#define PRIoLEAST16 __RIVET_LEN_LEAST16 "o"
#define PRIoLEAST32 __RIVET_LEN_LEAST32 "o"
#define PRIoLEAST64 __RIVET_LEN_LEAST64 "o"
#define PRIoFAST8 __RIVET_LEN_FAST8 "o"
#define PRIoFAST16 __RIVET_LEN_FAST16 "o"
#define PRIoFAST32 __RIVET_LEN_FAST32 "o"
#define PRIoFAST64 __RIVET_LEN_FAST64 "o"
#define PRIoMAX __RIVET_LEN_MAX "o"
#define PRIoPTR __RIVET_LEN_PTR "o"

#define PRIu8 __RIVET_LEN_8 "u"
#define PRIu16 __RIVET_LEN_16 "u"
#define PRIu32 __RIVET_LEN_32 "u"
#define PRIu64 __RIVET_LEN_64 "u"
#define PRIuLEAST8 __RIVET_LEN_LEAST8 "u"
#define PRIuLEAST16 __RIVET_LEN_LEAST16 "u"
#define PRIuLEAST32 __RIVET_LEN_LEAST32 "u"
#define PRIuLEAST64 __RIVET_LEN_LEAST64 "u"
#define PRIuFAST8 __RIVET_LEN_FAST8 "u"
#define PRIuFAST16 __RIVET_LEN_FAST16 "u"
#define PRIuFAST32 __RIVET_LEN_FAST32 "u"
#define PRIuFAST64 __RIVET_LEN_FAST64 "u"
#define PRIuMAX __RIVET_LEN_MAX "u"
#define PRIuPTR __RIVET_LEN_PTR "u"

#define PRIx8 __RIVET_LEN_8 "x"
#define PRIx16 __RIVET_LEN_16 "x"
#define PRIx32 __RIVET_LEN_32 "x"
#define PRIx64 __RIVET_LEN_64 "x"
#define PRIxLEAST8 __RIVET_LEN_LEAST8 "x"
#define PRIxLEAST16 __RIVET_LEN_LEAST16 "x"
#define PRIxLEAST32 __RIVET_LEN_LEAST32 "x"
#define PRIxLEAST64 __RIVET_LEN_LEAST64 "x"
#define PRIxFAST8 __RIVET_LEN_FAST8 "x"
#define PRIxFAST16 __RIVET_LEN_FAST16 "x"
#define PRIxFAST32 __RIVET_LEN_FAST32 "x"
#define PRIxFAST64 __RIVET_LEN_FAST64 "x"
#define PRIxMAX __RIVET_LEN_MAX "x"
#define PRIxPTR __RIVET_LEN_PTR "x"

#define PRIX8 __RIVET_LEN_8 "X"
#define PRIX16 __RIVET_LEN_16 "X"
#define PRIX32 __RIVET_LEN_32 "X"
#define PRIX64 __RIVET_LEN_64 "X"
#define PRIXLEAST8 __RIVET_LEN_LEAST8 "X"
#define PRIXLEAST16 __RIVET_LEN_LEAST16 "X"
#define PRIXLEAST32 __RIVET_LEN_LEAST32 "X"
#define PRIXLEAST64 __RIVET_LEN_LEAST64 "X"
#define PRIXFAST8 __RIVET_LEN_FAST8 "X"
#define PRIXFAST16 __RIVET_LEN_FAST16 "X"
#define PRIXFAST32 __RIVET_LEN_FAST32 "X"
#define PRIXFAST64 __RIVET_LEN_FAST64 "X"
#define PRIXMAX __RIVET_LEN_MAX "X"
#define PRIXPTR __RIVET_LEN_PTR "X"

#define SCNd8 __RIVET_LEN_8 "d"
#define SCNd16 __RIVET_LEN_16 "d"
#define SCNd32 __RIVET_LEN_32 "d"
#define SCNd64 __RIVET_LEN_64 "d"
#define SCNdLEAST8 __RIVET_LEN_LEAST8 "d"
#define SCNdLEAST16 __RIVET_LEN_LEAST16 "d"
#define SCNdLEAST32 __RIVET_LEN_LEAST32 "d"
#define SCNdLEAST64 __RIVET_LEN_LEAST64 "d"
#define SCNdFAST8 __RIVET_LEN_FAST8 "d"
#define SCNdFAST16 __RIVET_LEN_FAST16 "d"
#define SCNdFAST32 __RIVET_LEN_FAST32 "d"
#define SCNdFAST64 __RIVET_LEN_FAST64 "d"
#define SCNdMAX __RIVET_LEN_MAX "d"
#define SCNdPTR __RIVET_LEN_PTR "d"

#define SCNi8 __RIVET_LEN_8 "i"
#define SCNi16 __RIVET_LEN_16 "i"
#define SCNi32 __RIVET_LEN_32 "i"
#define SCNi64 __RIVET_LEN_64 "i"
#define SCNiLEAST8 __RIVET_LEN_LEAST8 "i"
#define SCNiLEAST16 __RIVET_LEN_LEAST16 "i"
#define SCNiLEAST32 __RIVET_LEN_LEAST32 "i"
#define SCNiLEAST64 __RIVET_LEN_LEAST64 "i"
#define SCNiFAST8 __RIVET_LEN_FAST8 "i"
#define SCNiFAST16 __RIVET_LEN_FAST16 "i"
#define SCNiFAST32 __RIVET_LEN_FAST32 "i"
#define SCNiFAST64 __RIVET_LEN_FAST64 "i"
#define SCNiMAX __RIVET_LEN_MAX "i"
#define SCNiPTR __RIVET_LEN_PTR "i"

#define SCNu8 __RIVET_LEN_8 "u"
#define SCNu16 __RIVET_LEN_16 "u"
#define SCNu32 __RIVET_LEN_32 "u"
#define SCNu64 __RIVET_LEN_64 "u"
#define SCNuLEAST8 __RIVET_LEN_LEAST8 "u"
#define SCNuLEAST16 __RIVET_LEN_LEAST16 "u"
#define SCNuLEAST32 __RIVET_LEN_LEAST32 "u"
#define SCNuLEAST64 __RIVET_LEN_LEAST64 "u"
#define SCNuFAST8 __RIVET_LEN_FAST8 "u"
#define SCNuFAST16 __RIVET_LEN_FAST16 "u"
#define SCNuFAST32 __RIVET_LEN_FAST32 "u"
#define SCNuFAST64 __RIVET_LEN_FAST64 "u"
#define SCNuMAX __RIVET_LEN_MAX "u"
#define SCNuPTR __RIVET_LEN_PTR "u"

#define SCNo8 __RIVET_LEN_8 "o"
#define SCNo16 __RIVET_LEN_16 "o"
#define SCNo32 __RIVET_LEN_32 "o"
#define SCNo64 __RIVET_LEN_64 "o"
#define SCNoLEAST8 __RIVET_LEN_LEAST8 "o"
#define SCNoLEAST16 __RIVET_LEN_LEAST16 "o"
#define SCNoLEAST32 __RIVET_LEN_LEAST32 "o"
#define SCNoLEAST64 __RIVET_LEN_LEAST64 "o"
#define SCNoFAST8 __RIVET_LEN_FAST8 "o"
#define SCNoFAST16 __RIVET_LEN_FAST16 "o"
#define SCNoFAST32 __RIVET_LEN_FAST32 "o"
#define SCNoFAST64 __RIVET_LEN_FAST64 "o"
#define SCNoMAX __RIVET_LEN_MAX "o"
#define SCNoPTR __RIVET_LEN_PTR "o"

#define SCNx8 __RIVET_LEN_8 "x"
#define SCNx16 __RIVET_LEN_16 "x"
#define SCNx32 __RIVET_LEN_32 "x"
#define SCNx64 __RIVET_LEN_64 "x"
#define SCNxLEAST8 __RIVET_LEN_LEAST8 "x"
#define SCNxLEAST16 __RIVET_LEN_LEAST16 "x"
#define SCNxLEAST32 __RIVET_LEN_LEAST32 "x"
#define SCNxLEAST64 __RIVET_LEN_LEAST64 "x"
#define SCNxFAST8 __RIVET_LEN_FAST8 "x"
#define SCNxFAST16 __RIVET_LEN_FAST16 "x"
#define SCNxFAST32 __RIVET_LEN_FAST32 "x"
#define SCNxFAST64 __RIVET_LEN_FAST64 "x"
#define SCNxMAX __RIVET_LEN_MAX "x"
#define SCNxPTR __RIVET_LEN_PTR "x"

#endif /* INTTYPES_H */
