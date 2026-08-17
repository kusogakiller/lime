#ifndef ITER8B_H
#define ITER8B_H

#ifdef __cplusplus
extern "C" {
#endif

// Iteration 8 regression fixture for the cross-library DEPENDENCY resolver
// (the fix that made libpng resolve zlib's header dir via find_header_dir /
// find_artifact_entry version-suffix fallback). libiter8b depends on libiter8
// and calls one of its functions; Charger must locate libiter8's header dir
// through the `deps = ["libiter8"]` manifest entry and link BOTH archives.
// Self-contained for AST extraction: coord_t / coord_sum come from libiter8,
// which the deps mechanism places on the include path.
#include "iter8.h"

coord_t iter8b_use(coord_t a, coord_t b); // forwards to coord_sum from libiter8

#ifdef __cplusplus
}
#endif

#endif
