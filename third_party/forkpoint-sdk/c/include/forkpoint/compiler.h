// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

// Macros that unify the differences between C/C++ compiler extensions and give
// older C++ standards the newer attributes: the Forkpoint SDK's counterpart of
// rivet's rivet_compiler.h, with the same macros and meanings.
//
// May be included by C, C++, and assembly files. Every macro is defined only
// when nothing defined it first, so firmware can include this header next to
// rivet_compiler.h or CMSIS's cmsis_gcc.h, which define some of the same
// names, without redefinition warnings; whichever header comes first wins, and
// their meanings agree.

#ifndef FORKPOINT_COMPILER_H
#define FORKPOINT_COMPILER_H

// Ensure we are using a known compiler.
#if !defined(__GNUC__) && !defined(__clang__)
#error "Unrecognized compiler!"
#endif

// Feature checking macros.
//
// If feature checking macros are not provided by the compiler, we assume that the
// checked features are unavailable.

// C++11 attribute checking.
#ifndef __has_cpp_attribute
#define __has_cpp_attribute(x) 0
#endif

// clang feature checking.
#ifndef __has_feature
#define __has_feature(x) 0
#endif

#if !defined(__ASSEMBLER__)

// C++ header guards.
#ifdef __cplusplus
#ifndef __BEGIN_CDECLS
#define __BEGIN_CDECLS extern "C" {
#endif
#ifndef __END_CDECLS
#define __END_CDECLS }
#endif
#else
#ifndef __BEGIN_CDECLS
#define __BEGIN_CDECLS
#endif
#ifndef __END_CDECLS
#define __END_CDECLS
#endif
#endif

//
// Function and data attributes.
//

// Function inlining directives.
#ifndef __NO_INLINE
#define __NO_INLINE __attribute__((__noinline__))
#endif
#ifndef __ALWAYS_INLINE
#define __ALWAYS_INLINE __attribute__((__always_inline__))
#endif

// Avoid issuing a warning if the given variable/function is unused.
#ifndef __cplusplus
#ifndef __UNUSED
#define __UNUSED __attribute__((__unused__))
#endif
#endif

// Pack the given structure or class, omitting padding between fields.
#ifndef __PACKED
#define __PACKED __attribute__((packed))
#endif

// Place the variable in thread-local storage.
#ifndef __THREAD
#define __THREAD __thread
#endif

// Align the variable or type to at least `x` bytes. `x` must be a power of two.
#ifndef __ALIGNED
#define __ALIGNED(x) __attribute__((aligned(x)))
#endif

// Declare the given function will never return. (such as `exit`, `abort`, etc).
#ifndef __NO_RETURN
#define __NO_RETURN __attribute__((__noreturn__))
#endif

// Warn if the result of this function is ignored by the caller.
#ifndef __WARN_UNUSED_RESULT
#define __WARN_UNUSED_RESULT __attribute__((__warn_unused_result__))
#endif

// Declare that the given stack variable may be left uninitialized if it does
// not have a specified initial value.
#if defined(__clang__)
#ifndef __UNINITIALIZED
#define __UNINITIALIZED __attribute__((uninitialized))
#endif
#else
#ifndef __UNINITIALIZED
#define __UNINITIALIZED
#endif
#endif

// Warn if a constructed objected is not used by the caller.
//
// Generates a warning on code like this:
//
//   {
//     AutoLock(&mutex);  // error: the AutoLock goes out of scope immediately.
//     locked_variable = 42;
//   }
//
// C++17 defines [[nodiscard]] to do this, but some older but supported
// versions of GCC support [[nodiscard]] on functions but not constructors.
// Having a separate __WARN_UNUSED_CONSTRUCTOR allows us to hide the attribute
// only on constructors on old versions of GCC.
#if defined(__cplusplus)
#if defined(__clang__) || (defined(__GNUC__) && (__GNUC__ >= 10))
#ifndef __WARN_UNUSED_CONSTRUCTOR
#define __WARN_UNUSED_CONSTRUCTOR [[nodiscard]]
#endif
#else
#ifndef __WARN_UNUSED_CONSTRUCTOR
#define __WARN_UNUSED_CONSTRUCTOR
#endif
#endif
#endif

// Mark a function or variable as deprecated, warning about any callers.
#if !defined(__DEPRECATE)
#define __DEPRECATE __attribute__((__deprecated__))
#endif

// Mark a function as having no visible side-effects: It may read memory, but
// will not modify it.
//
// Multiple calls to the function may be optimized away by the compiler.
#ifndef __PURE
#define __PURE __attribute__((__pure__))
#endif

// Mark a function has having an output determined solely on its input
// parameters (but not memory).
//
// Multiple calls to the function may be optimized away by the compiler, even
// if memory is modified between calls.
#ifndef __CONST
#define __CONST __attribute__((__const__))
#endif

// The given function is malloc-like, returning a pointer to new, unused
// memory.
//
// The compiler can assume that the returned pointer does not alias any other
// pointer, which may help the compiler optimize the program.
#ifndef __MALLOC
#define __MALLOC __attribute__((__malloc__))
#endif

// The given function allocates memory of size argument |x| or |x| * |y| and returns
// a pointer to that memory. Argument indexes are one-based. The compiler can use this
// information to support compile-time __builtin_object_size checks.
#if defined(__clang__)
#ifndef __ALLOC_SIZE
#define __ALLOC_SIZE(x, ...) __attribute__((__alloc_size__(x, ##__VA_ARGS__)))
#endif
#else
// GCC warns incorrectly on constructs that could result in large allocations
// for type reasons but cannot for other (checked) reasons. Disable the attribute
// there.
#ifndef __ALLOC_SIZE
#define __ALLOC_SIZE(x, ...)
#endif
#endif

// Indicate that the given function takes a printf/scanf-style format string.
//
// "__fmt" is the argument number of the format string, indexed from 1.
// "__varargs" is the argument number of the variable args "..." argument
// counting from 1.
//
// If applied to a class method, the implicit "this" parameter counts as the
// first argument.
#ifndef __PRINTFLIKE
#define __PRINTFLIKE(__fmt, __varargs) __attribute__((__format__(__printf__, __fmt, __varargs)))
#endif
#ifndef __SCANFLIKE
#define __SCANFLIKE(__fmt, __varargs) __attribute__((__format__(__scanf__, __fmt, __varargs)))
#endif

// Indicate that the `n`th argument to a function is non-null.
//
// The compiler will emit warnings if it can prove an argument is null, and
// may optimise assuming that the values are non-null.
#ifndef __NONNULL
#define __NONNULL(n) __attribute__((__nonnull__ n))
#endif

// The given function is a "leaf", and won't call further functions.
//
// Leaf functions must only return directly, and not call back into the
// current compilation unit (either via direct calls, or function pointers).
//
// May help the compiler optimize calls to the function in some cases.
#ifndef __LEAF_FN
#define __LEAF_FN __attribute__((__leaf__))
#endif

// Mark the given function or variable as `constexpr`.
//
// Used in code included by both C and C++. Code that is pure C++ should use
// `constexpr` directly.
#ifdef __cplusplus
#ifndef __CONSTEXPR
#define __CONSTEXPR constexpr
#endif
#else
#ifndef __CONSTEXPR
#define __CONSTEXPR
#endif
#endif

// Optimize the given function using custom flags.
//
// For example,
//
//  __OPTIMIZE("O3") int myfunction() { ... }
//
// will cause GCC to optimize the function at the O3 level, independent
// of what the compiler optimization flags are.
#if !defined(__clang__)
#ifndef __OPTIMIZE
#define __OPTIMIZE(x) __attribute__((__optimize__(x)))
#endif
#else
#ifndef __OPTIMIZE
#define __OPTIMIZE(x)
#endif
#endif

// Indicate the given function should not use LLVM's stack hardening features,
// but instead put all local variables on the standard stack.
//
// c.f. https://clang.llvm.org/docs/SafeStack.html
#if defined(__clang__)
#ifndef __NO_SAFESTACK
#define __NO_SAFESTACK __attribute__((__no_sanitize__("safe-stack", "shadow-call-stack")))
#endif
#else
#ifndef __NO_SAFESTACK
#define __NO_SAFESTACK
#endif
#endif

// The given C++ class or struct need not have a unique address when part of
// a larger struct or class, but can be safely collapsed into a zero-byte
// object by the compiler.
//
// Only C++ has these attributes. In C, GCC still defines __has_cpp_attribute,
// but cannot parse the scoped name `msvc::no_unique_address`, even in an #elif
// it never takes, so the tests sit inside the C++ branch.
#ifdef __cplusplus
#if __has_cpp_attribute(no_unique_address)
#ifndef __NO_UNIQUE_ADDRESS
#define __NO_UNIQUE_ADDRESS [[no_unique_address]]
#endif
#elif __has_cpp_attribute(msvc::no_unique_address)
// msvc targets, as used by efi, have a different attribute name, even when
// targeting C++20.
#ifndef __NO_UNIQUE_ADDRESS
#define __NO_UNIQUE_ADDRESS [[msvc::no_unique_address]]
#endif
#else
#ifndef __NO_UNIQUE_ADDRESS
#define __NO_UNIQUE_ADDRESS
#endif
#endif
#else
#ifndef __NO_UNIQUE_ADDRESS
#define __NO_UNIQUE_ADDRESS
#endif
#endif

// Indicate that the given function should be treated by the Clang static
// analyzer as if it doesn't return.
//
// A workaround to help static analyzer identify assertion failures
#if defined(__clang__)
#ifndef __ANALYZER_CREATE_SINK
#define __ANALYZER_CREATE_SINK __attribute__((analyzer_noreturn))
#endif
#else
#ifndef __ANALYZER_CREATE_SINK
#define __ANALYZER_CREATE_SINK
#endif
#endif

// Mark the given function as externally visible, and shouldn't be optimized
// away by link-time optimizations or whole-program optimizations.
#if !defined(__clang__)
#ifndef __EXTERNALLY_VISIBLE
#define __EXTERNALLY_VISIBLE __attribute__((__externally_visible__))
#endif
#else
#ifndef __EXTERNALLY_VISIBLE
#define __EXTERNALLY_VISIBLE
#endif
#endif

// Declare that this function declaration should be emitted as an alias for
// another function.
#ifndef __ALIAS
#define __ALIAS(x) __attribute__((__alias__(x)))
#endif

// Place the given global into a particular linker section.
#ifndef __SECTION
#define __SECTION(x) __attribute__((__section__(x)))
#endif

// The given function or global should be given a weak symbol, or a weak
// alias to another symbol.
#ifndef __WEAK
#define __WEAK __attribute__((__weak__))
#endif
#ifndef __WEAK_ALIAS
#define __WEAK_ALIAS(x) __attribute__((__weak__, __alias__(x)))
#endif

// The given static variable should still be emitted by the compiler, even if it
// appears unused to the compiler.
//
// Rarely needed. Not to be confused with "__UNUSED", which avoids the
// compiler warning if a variable appears unused.
#ifndef __ALWAYS_EMIT
#define __ALWAYS_EMIT __attribute__((__used__))
#endif

// Declare this object's ELF symbol visibility.
#ifndef __EXPORT
#define __EXPORT __attribute__((__visibility__("default")))
#endif
#ifndef __LOCAL
#define __LOCAL __attribute__((__visibility__("hidden")))
#endif

//
// Builtin functions.
//

// Provide a hint to the compiler that the given expression is likely/unlikely
// to be true.
#ifndef likely
#define likely(x) __builtin_expect(!!(x), 1)
#endif

#ifndef unlikely
#define unlikely(x) __builtin_expect(!!(x), 0)
#endif

// Return the program counter of the calling function.
#ifndef __GET_CALLER
#define __GET_CALLER(x) __builtin_return_address(0)
#endif

// Return the address of the current stack frame.
#ifndef __GET_FRAME
#define __GET_FRAME(x) __builtin_frame_address(0)
#endif

// Return true if the given expression is a known compile-time constant.
#ifndef __ISCONSTANT
#define __ISCONSTANT(x) __builtin_constant_p(x)
#endif

// Assume this branch of code cannot be reached.
#ifndef __UNREACHABLE
#define __UNREACHABLE __builtin_unreachable()
#endif

// Get the offset of `field` from the beginning of the struct or class `type`.
#ifndef __offsetof
#define __offsetof(type, field) __builtin_offsetof(type, field)
#endif

// Perform an arithmetic operation, returning "true" if the operation overflowed.
//
// Equivalent to: { *result = a + b; return _overflow_occurred; }
#ifndef add_overflow
#define add_overflow(a, b, result) __builtin_add_overflow(a, b, result)
#endif
#ifndef sub_overflow
#define sub_overflow(a, b, result) __builtin_sub_overflow(a, b, result)
#endif
#ifndef mul_overflow
#define mul_overflow(a, b, result) __builtin_mul_overflow(a, b, result)
#endif

// Indicate the given case of a switch statement is intended to fall through
// to the next case, and avoid generating a compiler warning.
//
//   switch (n) {
//     case 0:
//     case 1:
//       handle_zero_and_one_case();
//       __FALLTHROUGH;
//     default:
//       handle_all_cases();
//       break;
//   }
//
// Clang C++ takes its own attribute first, since the standard one is a C++17
// extension that -Wpedantic rejects in C++11 and C++14.
#if defined(__cplusplus) && defined(__clang__)
#ifndef __FALLTHROUGH
#define __FALLTHROUGH [[clang::fallthrough]]
#endif
#elif defined(__cplusplus) && __cplusplus >= 201703L
#ifndef __FALLTHROUGH
#define __FALLTHROUGH [[fallthrough]]
#endif
// The GNU style attribute is supported by Clang for C code, but __GNUC__ for
// clang right now is 4.
#elif __GNUC__ >= 7 || (!defined(__cplusplus) && defined(__clang__))
#ifndef __FALLTHROUGH
#define __FALLTHROUGH __attribute__((__fallthrough__))
#endif
#else
#ifndef __FALLTHROUGH
#define __FALLTHROUGH \
  do {                \
  } while (0)
#endif
#endif

// Locking annotations.
//
// The following annotations allow compile-time checking that annotated data
// is only accessed while holding the correct locks.
//
// The annotations are only supported by Clang, and frequently used with C++
// standard library types in userspace, so only enable in Clang when we know
// that the C++ standard library types are annotated or if we're in kernel
// code.
#if defined(__clang__) && (defined(_LIBCPP_ENABLE_THREAD_SAFETY_ANNOTATIONS) || defined(_KERNEL))
#ifndef __THREAD_ANNOTATION
#define __THREAD_ANNOTATION(x) __attribute__((x))
#endif
#else
#ifndef __THREAD_ANNOTATION
#define __THREAD_ANNOTATION(x)
#endif
#endif
#ifndef __TA_CAPABILITY
#define __TA_CAPABILITY(x) __THREAD_ANNOTATION(__capability__(x))
#endif
#ifndef __TA_GUARDED
#define __TA_GUARDED(x) __THREAD_ANNOTATION(__guarded_by__(x))
#endif
#ifndef __TA_ACQUIRE
#define __TA_ACQUIRE(...) __THREAD_ANNOTATION(__acquire_capability__(__VA_ARGS__))
#endif
#ifndef __TA_ACQUIRE_SHARED
#define __TA_ACQUIRE_SHARED(...) __THREAD_ANNOTATION(__acquire_shared_capability__(__VA_ARGS__))
#endif
#ifndef __TA_TRY_ACQUIRE
#define __TA_TRY_ACQUIRE(...) __THREAD_ANNOTATION(__try_acquire_capability__(__VA_ARGS__))
#endif
#ifndef __TA_TRY_ACQUIRE_SHARED
#define __TA_TRY_ACQUIRE_SHARED(...) \
  __THREAD_ANNOTATION(__try_acquire_shared_capability__(__VA_ARGS__))
#endif
// Neither ACQUIRED_BEFORE nor ACQUIRED_AFTER are implemented in clang. Users of these macros must
// not rely upon them to catch lock ordering bugs, and must treat them as documentation only.
// See:
// https://clang.llvm.org/docs/ThreadSafetyAnalysis.html#acquired-before-and-acquired-after-are-currently-unimplemented
#ifndef __TA_ACQUIRED_BEFORE
#define __TA_ACQUIRED_BEFORE(...) __THREAD_ANNOTATION(__acquired_before__(__VA_ARGS__))
#endif
#ifndef __TA_ACQUIRED_AFTER
#define __TA_ACQUIRED_AFTER(...) __THREAD_ANNOTATION(__acquired_after__(__VA_ARGS__))
#endif
#ifndef __TA_RELEASE
#define __TA_RELEASE(...) __THREAD_ANNOTATION(__release_capability__(__VA_ARGS__))
#endif
#ifndef __TA_RELEASE_SHARED
#define __TA_RELEASE_SHARED(...) __THREAD_ANNOTATION(__release_shared_capability__(__VA_ARGS__))
#endif
#ifndef __TA_REQUIRES
#define __TA_REQUIRES(...) __THREAD_ANNOTATION(__requires_capability__(__VA_ARGS__))
#endif
#ifndef __TA_REQUIRES_SHARED
#define __TA_REQUIRES_SHARED(...) __THREAD_ANNOTATION(__requires_shared_capability__(__VA_ARGS__))
#endif
#ifndef __TA_EXCLUDES
#define __TA_EXCLUDES(...) __THREAD_ANNOTATION(__locks_excluded__(__VA_ARGS__))
#endif
#ifndef __TA_ASSERT
#define __TA_ASSERT(...) __THREAD_ANNOTATION(__assert_capability__(__VA_ARGS__))
#endif
#ifndef __TA_ASSERT_SHARED
#define __TA_ASSERT_SHARED(...) __THREAD_ANNOTATION(__assert_shared_capability__(__VA_ARGS__))
#endif
#ifndef __TA_RETURN_CAPABILITY
#define __TA_RETURN_CAPABILITY(x) __THREAD_ANNOTATION(__lock_returned__(x))
#endif
#ifndef __TA_SCOPED_CAPABILITY
#define __TA_SCOPED_CAPABILITY __THREAD_ANNOTATION(__scoped_lockable__)
#endif
#ifndef __TA_NO_THREAD_SAFETY_ANALYSIS
#define __TA_NO_THREAD_SAFETY_ANALYSIS __THREAD_ANNOTATION(__no_thread_safety_analysis__)
#endif

// Experimental lifetime analysis annotations.
#ifdef __clang__
#ifndef __OWNER
#define __OWNER(x) [[gsl::Owner(x)]]
#endif
#ifndef __POINTER
#define __POINTER(x) [[gsl::Pointer(x)]]
#endif
#else
#ifndef __OWNER
#define __OWNER(x)
#endif
#ifndef __POINTER
#define __POINTER(x)
#endif
#endif

#if defined(__cpp_constinit) && __cpp_constinit >= 201907L
#ifndef __CONSTINIT
#define __CONSTINIT constinit
#endif
#elif defined(__clang__)
#ifndef __CONSTINIT
#define __CONSTINIT [[clang::require_constant_initialization]]
#endif
#elif defined(__GNUC__) && __GNUC__ >= 13
#ifndef __CONSTINIT
#define __CONSTINIT __constinit
#endif
#else
#ifndef __CONSTINIT
#define __CONSTINIT
#endif
#endif

#endif  // !defined(__ASSEMBLER__)

#endif  // FORKPOINT_COMPILER_H
