// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* Output, from src/stdio/, through streams. A stream (FILE) is a write
 * callback and the context it takes; there is no file system, buffering, or
 * input. stdout and stderr are two streams whose callbacks call the
 * firmware's write (see unistd.h) to file descriptors 1 and 2. Firmware sends
 * either elsewhere, a log buffer or another UART, by assigning it a stream
 * before any thread prints:
 *
 *   *stdout = (FILE){uart_write, &uart1};
 *
 * Every output function holds the standard I/O lock (see rivet_stdio.h) for
 * the whole of its output, whatever the stream. snprintf and its siblings
 * write to the caller's buffer and take no lock.
 *
 * The printf family takes the flags - + space # 0, a width and precision as
 * digits or *, the lengths hh h l ll j z t, and the conversions d i u o x X c
 * s p %. The floating-point conversions, %n, the wide conversions, and L are
 * not supported: they stop the formatting with -1 and errno EINVAL. A result
 * past INT_MAX bytes gives -1 and EOVERFLOW. */

#ifndef STDIO_H
#define STDIO_H

#define __RIVET_RESERVED_ONLY
#include <rivet_compiler.h>
#undef __RIVET_RESERVED_ONLY
#include <stdarg.h>
#include <stddef.h>

__BEGIN_CDECLS

#define EOF (-1)

/* A stream: write writes the n bytes at s, given ctx, and returns n, or a
 * negative value when it failed. rivet never passes it more than INT_MAX
 * bytes at once. A stream whose write is NULL fails every write. */
#if !defined(__FILE_defined)
#define __FILE_defined
struct __rivet_file {
  int (*write)(void *ctx, const char *s, size_t n);
  void *ctx;
};
typedef struct __rivet_file FILE;
#endif

extern FILE __rivet_stdout;
extern FILE __rivet_stderr;
#define stdout (&__rivet_stdout)
#define stderr (&__rivet_stderr)

/* Each returns EOF, or 0 items for fwrite, with errno set to EIO when the
 * stream fails. A stream writes all it is given or fails, so fwrite returns
 * every item or none. */
int fputc(int c, FILE *stream);
int putc(int c, FILE *stream);
int fputs(const char *__restrict s, FILE *__restrict stream);
size_t fwrite(const void *__restrict buf, size_t size, size_t count, FILE *__restrict stream);
int putchar(int c);
int puts(const char *s);

int printf(const char *__restrict format, ...) __PRINTFLIKE(1, 2);
int vprintf(const char *__restrict format, va_list ap) __PRINTFLIKE(1, 0);
int fprintf(FILE *__restrict stream, const char *__restrict format, ...) __PRINTFLIKE(2, 3);
int vfprintf(FILE *__restrict stream, const char *__restrict format, va_list ap) __PRINTFLIKE(2, 0);
int snprintf(char *__restrict buf, size_t n, const char *__restrict format, ...) __PRINTFLIKE(3, 4);
int vsnprintf(char *__restrict buf, size_t n, const char *__restrict format, va_list ap)
    __PRINTFLIKE(3, 0);
int sprintf(char *__restrict buf, const char *__restrict format, ...) __PRINTFLIKE(2, 3);
int vsprintf(char *__restrict buf, const char *__restrict format, va_list ap) __PRINTFLIKE(2, 0);

__END_CDECLS

#endif /* STDIO_H */
