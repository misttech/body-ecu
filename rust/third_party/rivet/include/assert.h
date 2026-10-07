// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

#ifndef ASSERT_H
#define ASSERT_H

#define __RIVET_RESERVED_ONLY
#include <rivet_compiler.h>
#undef __RIVET_RESERVED_ONLY

__BEGIN_CDECLS
__NO_RETURN void __assert_func(const char *file, int line, const char *function,
                               const char *expression);

#ifdef NDEBUG
#define assert(e) ((void)0)
#else
#define assert(e) ((e) ? (void)0 : __assert_func(__FILE__, __LINE__, __func__, #e))
#endif

__END_CDECLS

#endif /* ASSERT_H */
