#include "libptrtypedef.h"
#include <stdlib.h>

bytep ptd_make(byte_t init) {
    bytep p = (bytep)malloc(sizeof(byte_t));
    if (p) { *p = init; }
    return p;
}

byte_t ptd_get(const bytep p) { return p ? *p : 0; }

void ptd_set(bytep p, byte_t v) { if (p) { *p = v; } }

int ptd_sum(cbytep p, int n) {
    if (!p || n <= 0) { return 0; }
    int s = 0;
    for (int i = 0; i < n; i++) { s += (int)p[i]; }
    return s;
}

PTObj ptd_obj_make(int v) {
    PTObj o = (PTObj)malloc(sizeof(struct PTObj_));
    if (o) { o->v = v; }
    return o;
}

int ptd_obj_get(PTObj o) { return o ? o->v : -1; }

int ptd_link_anchor(void) { return 42; }
