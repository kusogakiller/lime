#ifndef LIBVARIANT_H
#define LIBVARIANT_H

// Phase 1 C ABI completeness test library: union, enum, constants, macros.
// Charger must extract these from the header and surface them as Lime-legal
// constructs (union -> struct-sized layout, enum -> Int + const values,
// macros/constants -> const values) WITHOUT changing Lime's type system.

// ---- union ----
// A union's size is its largest member. Charger should model it with the
// maximum-member layout so pass-by-value has the right ABI size.
typedef union {
    int i;
    float f;
    long long ll;
} Variant;

// ---- enum ----
// Enums are ABI-compatible with int. Charger should surface the type as Int
// and the enumerators as named constants.
typedef enum {
    COLOR_RED = 1,
    COLOR_GREEN = 2,
    COLOR_BLUE = 4
} Color;

// ---- constants / macros ----
#define LIBVARIANT_MAX 1024
static const int LIBVARIANT_VERSION = 3;

// ---- API using the above ----
// Set the int member of a Variant and return it (proves union layout / size).
long long variant_set_int(int v);
// Tagged helper: return the float reinterpretation of an int (proves union
// pass-by-value ABI carries the full union width).
float variant_as_float(long long bits);
// Return the OR of two Color values (proves enum -> Int mapping).
int color_or(Color a, Color b);
// Return LIBVARIANT_MAX (proves macro/constant extraction).
int get_max();
// Return LIBVARIANT_VERSION (proves static const extraction).
int get_version();

#endif
