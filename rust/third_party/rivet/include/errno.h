// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* errno, from src/errno/. There is one cell for the program until a kernel
 * with threads names the running thread's own through rivet_errno.h.
 *
 * The codes are newlib's, with the values its bare-metal Arm build uses, plus
 * ENOTBLK, ESHUTDOWN, and ESOCKTNOSUPPORT from its Linux set: code built
 * against newlib's errno.h, such as a vendor library, and code built against
 * this one agree on every value. Rivet's own functions set EIO when the
 * firmware's write fails, EINVAL for a bad strtol base or printf conversion,
 * ERANGE for a strtol result out of range, and EOVERFLOW for a printf result
 * longer than INT_MAX bytes. */

#ifndef ERRNO_H
#define ERRNO_H

#define __RIVET_RESERVED_ONLY
#include <rivet_compiler.h>
#undef __RIVET_RESERVED_ONLY

__BEGIN_CDECLS

int *__errno_location(void);
#define errno (*__errno_location())

#define EPERM 1
#define ENOENT 2
#define ESRCH 3
#define EINTR 4
#define EIO 5
#define ENXIO 6
#define E2BIG 7
#define ENOEXEC 8
#define EBADF 9
#define ECHILD 10
#define EAGAIN 11
#define ENOMEM 12
#define EACCES 13
#define EFAULT 14
#define ENOTBLK 15
#define EBUSY 16
#define EEXIST 17
#define EXDEV 18
#define ENODEV 19
#define ENOTDIR 20
#define EISDIR 21
#define EINVAL 22
#define ENFILE 23
#define EMFILE 24
#define ENOTTY 25
#define ETXTBSY 26
#define EFBIG 27
#define ENOSPC 28
#define ESPIPE 29
#define EROFS 30
#define EMLINK 31
#define EPIPE 32
#define EDOM 33
#define ERANGE 34
#define ENOMSG 35
#define EIDRM 36
#define EDEADLK 45
#define ENOLCK 46
#define ENOSTR 60
#define ENODATA 61
#define ETIME 62
#define ENOSR 63
#define ENOLINK 67
#define EPROTO 71
#define EMULTIHOP 74
#define EBADMSG 77
#define EFTYPE 79
#define ENOSYS 88
#define ENOTEMPTY 90
#define ENAMETOOLONG 91
#define ELOOP 92
#define EOPNOTSUPP 95
#define EPFNOSUPPORT 96
#define ECONNRESET 104
#define ENOBUFS 105
#define EAFNOSUPPORT 106
#define EPROTOTYPE 107
#define ENOTSOCK 108
#define ENOPROTOOPT 109
#define ESHUTDOWN 110
#define ECONNREFUSED 111
#define EADDRINUSE 112
#define ECONNABORTED 113
#define ENETUNREACH 114
#define ENETDOWN 115
#define ETIMEDOUT 116
#define EHOSTDOWN 117
#define EHOSTUNREACH 118
#define EINPROGRESS 119
#define EALREADY 120
#define EDESTADDRREQ 121
#define EMSGSIZE 122
#define EPROTONOSUPPORT 123
#define ESOCKTNOSUPPORT 124
#define EADDRNOTAVAIL 125
#define ENETRESET 126
#define EISCONN 127
#define ENOTCONN 128
#define ETOOMANYREFS 129
#define EDQUOT 132
#define ESTALE 133
#define ENOTSUP 134
#define EILSEQ 138
#define EOVERFLOW 139
#define ECANCELED 140
#define ENOTRECOVERABLE 141
#define EOWNERDEAD 142
#define EWOULDBLOCK EAGAIN

__END_CDECLS

#endif /* ERRNO_H */
