#ifndef ITER8_H
#define ITER8_H

#ifdef __cplusplus
extern "C" {
#endif

// Iteration 8 regression fixture for Charger (C-only, general mechanism).
// Encodes the exact bug classes fixed in Iteration 8:
//   1. scalar-typedef POINTER (pre-pass fix):
//        `typedef double coord_t;` then `coord_t* scale(coord_t*, int)`
//        must normalize to Pointer(Double) [Opaque(ScalarPtr)], NOT a bare
//        scalar. A scalar-typedef pointee must not have its pointer dropped.
//   2. `double*` STRUCT FIELD (sqlite3_rtree_dbl* collapse fix):
//        Geo.aParam is `double*` and must stay a pointer handle, not collapse
//        to a 4-byte scalar (which previously crashed the accessor shim).
//   3. `#else`-guarded `main()` TU (shell.c / crc32.c filter fix):
//        iter8_cli.c defines `main` behind an `#else` (real entry point) AND
//        contains a NESTED #if/#else inside an outer #ifdef; the preprocessor
//        depth tracker must not corrupt the nesting when it sees the #else.
//   4. header auto-selection: the single public header (this file) is picked
//        without a charger.toml (select_api_header root-header heuristic).

// (1) scalar typedef whose pointee is used as a POINTER.
typedef double coord_t;

// Returns coord_t* — exercises the scalar-typedef pointer path.
coord_t* scale(coord_t* p, int n);

// Plain scalar-typedef VALUE param (sanity: already worked).
coord_t coord_sum(coord_t a, coord_t b);

// (2) struct carrying a `double*` field — the sqlite rtree-dbl collapse class.
// Named-tag typedef so Charger's RecordDecl path captures it (anonymous-tag
// typedefs are not captured by design).
typedef struct Geo {
    int    n;
    double* aParam; // scalar-pointee pointer -> Opaque(ScalarPtr), never Float
} Geo;

Geo* geo_make(int n);
int  geo_count(Geo* g);
void geo_free(Geo* g);

#ifdef __cplusplus
}
#endif

#endif
