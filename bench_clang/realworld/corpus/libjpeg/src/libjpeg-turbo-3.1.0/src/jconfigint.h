/* Minimal jconfigint.h for Charger build (substitute for CMake-generated file).
 * Only the macros the library sources actually consume are defined; values are
 * the standard 64-bit Windows/clang defaults. This is corpus scaffolding, not
 * Charger logic. */
#ifndef JCONFIGINT_H
#define JCONFIGINT_H

#define BUILD "20240829"

#define HIDDEN
#define INLINE inline
#define THREAD_LOCAL

#define PACKAGE_NAME "libjpeg-turbo"
#define VERSION "3.1.0"

#define SIZEOF_SIZE_T 8

/* clang supports the fallthrough attribute. */
#if defined(__has_attribute)
#if __has_attribute(fallthrough)
#define FALLTHROUGH  __attribute__((fallthrough));
#else
#define FALLTHROUGH
#endif
#else
#define FALLTHROUGH
#endif

/* 8-bit sample precision is the standard libjpeg-turbo setting. */
#ifndef BITS_IN_JSAMPLE
#define BITS_IN_JSAMPLE 8
#endif

/* clang on Windows has __builtin_ctzl and a working intrin.h */
#define HAVE_BUILTIN_CTZL
#define HAVE_INTRIN_H

/* No libjpeg-turbo SIMD / arithmetic-coding / getenv build-time knobs are
 * required for a plain C build. */
#undef C_ARITH_CODING_SUPPORTED
#undef D_ARITH_CODING_SUPPORTED
#undef WITH_SIMD
#define NO_GETENV

#endif /* JCONFIGINT_H */
