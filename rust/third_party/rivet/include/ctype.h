// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

/* Character classification in the "C" locale, from src/ctype/. Each takes an
 * unsigned char value or EOF; the predicates return 1 or 0. */

#ifndef CTYPE_H
#define CTYPE_H

#define __RIVET_RESERVED_ONLY
#include <rivet_compiler.h>
#undef __RIVET_RESERVED_ONLY

__BEGIN_CDECLS
__RIVET_CONST int isalnum(int c);
__RIVET_CONST int isalpha(int c);
__RIVET_CONST int isblank(int c);
__RIVET_CONST int iscntrl(int c);
__RIVET_CONST int isdigit(int c);
__RIVET_CONST int isgraph(int c);
__RIVET_CONST int islower(int c);
__RIVET_CONST int isprint(int c);
__RIVET_CONST int ispunct(int c);
__RIVET_CONST int isspace(int c);
__RIVET_CONST int isupper(int c);
__RIVET_CONST int isxdigit(int c);
__RIVET_CONST int tolower(int c);
__RIVET_CONST int toupper(int c);

__END_CDECLS

#endif /* CTYPE_H */
