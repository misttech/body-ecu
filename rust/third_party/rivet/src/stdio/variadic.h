// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* The calls between variadic.c and the Rust formatter. Rust defines the
 * rust_* functions (src/stdio/vsnprintf.rs and vfprintf.rs) and variadic.c
 * the cpp_* ones, which src/support/va_list.rs declares on the Rust side. */

#ifndef RIVET_SRC_STDIO_VARIADIC_H
#define RIVET_SRC_STDIO_VARIADIC_H

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

struct rivet_va_list;

int rust_vsnprintf(char *buf, size_t n, const char *format, struct rivet_va_list *va);
int rust_vfprintf(FILE *stream, const char *format, struct rivet_va_list *va);

intmax_t cpp_va_int(struct rivet_va_list *va, int kind);
uintmax_t cpp_va_uint(struct rivet_va_list *va, int kind);
const void *cpp_va_pointer(struct rivet_va_list *va);

#endif /* RIVET_SRC_STDIO_VARIADIC_H */
