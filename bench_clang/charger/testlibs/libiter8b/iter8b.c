#include "iter8b.h"
#include "iter8.h"   // self-contained: pulls coord_t + coord_sum from libiter8

// Forward to libiter8::coord_sum — proves the dependency header dir is on the
// include path and that BOTH archives are linked (no undefined symbol).
coord_t iter8b_use(coord_t a, coord_t b) {
    return coord_sum(a, b);
}
