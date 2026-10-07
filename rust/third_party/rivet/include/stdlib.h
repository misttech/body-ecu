// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

#ifndef STDLIB_H
#define STDLIB_H

#define __RIVET_RESERVED_ONLY
#include <rivet_compiler.h>
#undef __RIVET_RESERVED_ONLY
#include <stddef.h>

__BEGIN_CDECLS

/* Writes "abort()" to standard error and ends the program with _exit(134). */
__NO_RETURN void abort(void);

/* The initial decimal number in s, as (int)strtol(s, NULL, 10). */
__RIVET_PURE int atoi(const char *s);

/* The initial number in s: white space, an optional sign, and digits in base,
 * which is 0 or 2 to 36. Base 16 allows a 0x prefix; base 0 takes the base
 * from the prefix (0x is 16, 0 is 8, else 10). *endptr, if endptr is not
 * NULL, receives the address past the number, or s when there is none. A
 * number outside the type gives its limit with errno set to ERANGE; an
 * invalid base gives 0 with EINVAL. The unsigned ones negate a - value in
 * their type, so "-1" gives the maximum. */
long strtol(const char *__restrict s, char **__restrict endptr, int base);
unsigned long strtoul(const char *__restrict s, char **__restrict endptr, int base);
long long strtoll(const char *__restrict s, char **__restrict endptr, int base);
unsigned long long strtoull(const char *__restrict s, char **__restrict endptr, int base);

/* The absolute value. The most negative value has none; these return it
 * unchanged. */
__RIVET_CONST int abs(int n);
__RIVET_CONST long labs(long n);
__RIVET_CONST long long llabs(long long n);

/* The sorted-array functions: cmp(a, b) is negative, zero, or positive as a
 * orders before, equal to, or after b. qsort sorts in place without
 * allocating, in O(n log n) comparisons at worst; equal elements may end in
 * any order. bsearch returns an element of the sorted array that cmp orders
 * equal to key, or NULL. */
void qsort(void *base, size_t nel, size_t width, int (*cmp)(const void *, const void *));
void *bsearch(const void *key, const void *base, size_t nel, size_t width,
              int (*cmp)(const void *, const void *));

/* With rivet's cmpctmalloc feature, these allocate from the running domain's
 * heap, or the default heap (see rivet_heap.h); without it, the firmware's own
 * allocator provides them. malloc(0) returns NULL. */
__MALLOC __ALLOC_SIZE(1) void *malloc(size_t size);
__MALLOC __ALLOC_SIZE(1, 2) void *calloc(size_t count, size_t size);
__ALLOC_SIZE(2) void *realloc(void *ptr, size_t size);
__MALLOC __ALLOC_SIZE(2) void *aligned_alloc(size_t alignment, size_t size);
void free(void *ptr);
void free_sized(void *ptr, size_t size);

__END_CDECLS

#endif /* STDLIB_H */
