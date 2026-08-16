#ifndef LIBVARIADIC_H
#define LIBVARIADIC_H

// Phase 1 Iteration 5: C variadic functions — generic ABI test fixture.
// These are deliberately ordinary, library-agnostic variadic APIs (no
// printf-family, no library-specific special casing). Charger's general
// variadic engine must surface all of them.

#include <stdarg.h>

// Opaque handle type (named so Charger surfaces it as `Opaque(Dummy)` rather
// than a bare `void*` => `Unit`).
typedef struct Dummy { int marker; } *Handle;

// Returns a stable opaque pointer (used to exercise passing an opaque handle
// as a variadic argument). Not a variadic function itself.
Handle var_handle(void);

// Sum `count` variadic int arguments. (default variadic shape: Int)
int var_sum(int count, ...);

// Sum `count` variadic long long arguments.
long long var_sum_i64(int count, ...);

// Sum `count` variadic double arguments.
double var_sum_double(int count, ...);

// Sum the lengths of `count` variadic C-string arguments.
int var_strlen_sum(int count, ...);

// Echo a fixed opaque pointer through one variadic opaque-pointer slot.
// Returns 1 if the variadic pointer equals the fixed one.
int var_echo_ptr(void *base, ...);

// Mixed fixed + variadic: one int, one double, one opaque pointer.
// Returns (i > 0) + (d > 0 ? 1 : 0) + (p != 0 ? 1 : 0) — i.e. a 3-bit
// signature proving each slot was received in the correct register class
// (GP for int/ptr, FP for double).
int var_mixed(int n, ...);

// Default-argument-promotion probe: even slots read as int (char/short
// promote to int), odd slots read as double (float promotes to double).
int var_promote(int count, ...);

// Enum passed as a variadic argument (its underlying int value is passed).
typedef enum { COLOR_RED = 10, COLOR_GREEN = 20, COLOR_BLUE = 30 } Color;
int var_sum_enum(int count, ...);

#endif
