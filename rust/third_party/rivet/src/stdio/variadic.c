// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* The variadic entry points of stdio.h: printf, vprintf, fprintf, vfprintf,
 * snprintf, vsnprintf, sprintf, and vsprintf.
 *
 * They are C because stable Rust can neither define a C variadic function nor
 * read a va_list. Each wraps its va_list in a struct rivet_va_list and hands
 * that to the Rust formatter, rust_vsnprintf or rust_vfprintf (src/stdio/),
 * which asks for each argument through the cpp_va_* calls below as its format
 * names them. src/build.rs compiles this file into the archive for a
 * bare-metal target; a host build has no need of it. */

#include "variadic.h"

#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

struct rivet_va_list {
  va_list ap;
};

/* The kinds of integer argument the formatter asks for; src/support/va_list.rs
 * numbers them the same. hh and h arguments arrive promoted to int. */
enum {
  RIVET_VA_INT,
  RIVET_VA_LONG,
  RIVET_VA_LONG_LONG,
  RIVET_VA_INTMAX,
  RIVET_VA_SIZE,
  RIVET_VA_PTRDIFF,
};

/* intmax_t is long long on some ABIs, which makes two branches alike. */
// NOLINTBEGIN(bugprone-branch-clone)
intmax_t cpp_va_int(struct rivet_va_list *va, int kind) {
  switch (kind) {
    case RIVET_VA_INT:
      return va_arg(va->ap, int);
    case RIVET_VA_LONG:
      return va_arg(va->ap, long);
    case RIVET_VA_LONG_LONG:
      return va_arg(va->ap, long long);
    case RIVET_VA_INTMAX:
      return va_arg(va->ap, intmax_t);
    case RIVET_VA_SIZE:
      return (intmax_t)va_arg(va->ap, size_t);
    case RIVET_VA_PTRDIFF:
      return va_arg(va->ap, ptrdiff_t);
    default:
      return 0;
  }
}

uintmax_t cpp_va_uint(struct rivet_va_list *va, int kind) {
  switch (kind) {
    case RIVET_VA_INT:
      return va_arg(va->ap, unsigned);
    case RIVET_VA_LONG:
      return va_arg(va->ap, unsigned long);
    case RIVET_VA_LONG_LONG:
      return va_arg(va->ap, unsigned long long);
    case RIVET_VA_INTMAX:
      return va_arg(va->ap, uintmax_t);
    case RIVET_VA_SIZE:
      return va_arg(va->ap, size_t);
    case RIVET_VA_PTRDIFF:
      return (uintmax_t)va_arg(va->ap, ptrdiff_t);
    default:
      return 0;
  }
}

// NOLINTEND(bugprone-branch-clone)

const void *cpp_va_pointer(struct rivet_va_list *va) { return va_arg(va->ap, const void *); }

int vsnprintf(char *restrict buf, size_t n, const char *restrict format, va_list ap) {
  struct rivet_va_list va;
  va_copy(va.ap, ap);
  int result = rust_vsnprintf(buf, n, format, &va);
  va_end(va.ap);
  return result;
}

int snprintf(char *restrict buf, size_t n, const char *restrict format, ...) {
  va_list ap;
  va_start(ap, format);
  int result = vsnprintf(buf, n, format, ap);
  va_end(ap);
  return result;
}

int vsprintf(char *restrict buf, const char *restrict format, va_list ap) {
  return vsnprintf(buf, SIZE_MAX, format, ap);
}

int sprintf(char *restrict buf, const char *restrict format, ...) {
  va_list ap;
  va_start(ap, format);
  int result = vsnprintf(buf, SIZE_MAX, format, ap);
  va_end(ap);
  return result;
}

int vfprintf(FILE *restrict stream, const char *restrict format, va_list ap) {
  struct rivet_va_list va;
  va_copy(va.ap, ap);
  int result = rust_vfprintf(stream, format, &va);
  va_end(va.ap);
  return result;
}

int fprintf(FILE *restrict stream, const char *restrict format, ...) {
  va_list ap;
  va_start(ap, format);
  int result = vfprintf(stream, format, ap);
  va_end(ap);
  return result;
}

int vprintf(const char *restrict format, va_list ap) { return vfprintf(stdout, format, ap); }

int printf(const char *restrict format, ...) {
  va_list ap;
  va_start(ap, format);
  int result = vprintf(format, ap);
  va_end(ap);
  return result;
}
