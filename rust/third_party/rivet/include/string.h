// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* The string functions of rivet, a freestanding C library, src/string/. */

#ifndef STRING_H
#define STRING_H

#define __RIVET_RESERVED_ONLY
#include <rivet_compiler.h>
#undef __RIVET_RESERVED_ONLY
#include <stddef.h>

__BEGIN_CDECLS

void *memcpy(void *__restrict dst, const void *__restrict src, size_t n);
void *memmove(void *dst, const void *src, size_t n);
void *memset(void *dst, int c, size_t n);
__RIVET_PURE int memcmp(const void *a, const void *b, size_t n);
__RIVET_PURE void *memchr(const void *s, int c, size_t n);
__RIVET_PURE size_t strlen(const char *s);
/* POSIX, not ISO C: strlen that stops at n bytes. */
__RIVET_PURE size_t strnlen(const char *s, size_t n);
__RIVET_PURE int strcmp(const char *a, const char *b);
__RIVET_PURE int strncmp(const char *a, const char *b, size_t n);
char *strcpy(char *__restrict dst, const char *__restrict src);
/* POSIX, not ISO C: strcpy that returns the address of the NUL it wrote. */
char *stpcpy(char *__restrict dst, const char *__restrict src);
char *strncpy(char *__restrict dst, const char *__restrict src, size_t n);
char *strcat(char *__restrict dst, const char *__restrict src);
char *strncat(char *__restrict dst, const char *__restrict src, size_t n);
__RIVET_PURE char *strchr(const char *s, int c);
__RIVET_PURE char *strrchr(const char *s, int c);
__RIVET_PURE char *strstr(const char *haystack, const char *needle);

__END_CDECLS

#endif /* STRING_H */
