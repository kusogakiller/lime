#include "libvariant.h"

long long variant_set_int(int v) {
    Variant x;
    x.i = v;
    // Return as long long to prove the union carries at least int width.
    return (long long)x.i;
}

float variant_as_float(long long bits) {
    Variant x;
    x.ll = bits;
    // Reinterpret the low 32 bits as float (union pass-by-value ABI).
    return x.f;
}

int color_or(Color a, Color b) {
    return (int)a | (int)b;
}

int get_max() {
    return LIBVARIANT_MAX;
}

int get_version() {
    return LIBVARIANT_VERSION;
}
