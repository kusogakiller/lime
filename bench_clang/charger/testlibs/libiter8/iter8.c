#include "iter8.h"
#include <stdlib.h>

// (1) scalar-typedef pointer return.
coord_t* scale(coord_t* p, int n) {
    coord_t* out = (coord_t*)malloc(sizeof(coord_t) * (n > 0 ? n : 1));
    for (int i = 0; i < n; i++) out[i] = p[i] * 2.0;
    return out;
}

// (1) scalar-typedef value param.
coord_t coord_sum(coord_t a, coord_t b) { return a + b; }

// (2) struct with a `double*` field.
Geo* geo_make(int n) {
    Geo* g = (Geo*)malloc(sizeof(Geo));
    g->n = n;
    g->aParam = (double*)malloc(sizeof(double) * (n > 0 ? n : 1));
    for (int i = 0; i < n; i++) g->aParam[i] = (double)(i + 1);
    return g;
}

int geo_count(Geo* g) { return g ? g->n : 0; }

void geo_free(Geo* g) {
    if (g) { free(g->aParam); free(g); }
}
