// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* The types and constants of math.h, with no functions yet: rivet grows them
 * as firmware needs them. float_t and double_t are the types the compiler
 * evaluates float and double arithmetic in, which it reports through
 * FLT_EVAL_METHOD; the constants and classification macros are the
 * compiler's builtins, so nothing here needs the archive. */

#ifndef MATH_H
#define MATH_H

#if defined(__FLT_EVAL_METHOD__) && __FLT_EVAL_METHOD__ == 2
typedef long double float_t;
typedef long double double_t;
#elif defined(__FLT_EVAL_METHOD__) && __FLT_EVAL_METHOD__ == 1
typedef double float_t;
typedef double double_t;
#else
typedef float float_t;
typedef double double_t;
#endif

#define HUGE_VALF __builtin_huge_valf()
#define HUGE_VAL __builtin_huge_val()
#define HUGE_VALL __builtin_huge_vall()
#define INFINITY __builtin_inff()
#define NAN __builtin_nanf("")

#define isnan(x) __builtin_isnan(x)
#define isinf(x) __builtin_isinf(x)
#define isfinite(x) __builtin_isfinite(x)
#define isnormal(x) __builtin_isnormal(x)
#define signbit(x) __builtin_signbit(x)

#endif /* MATH_H */
