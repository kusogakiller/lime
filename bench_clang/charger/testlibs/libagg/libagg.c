#include "libagg.h"
#include <string.h>
#include <stdlib.h>

int agg_origin_x(const Aggregate *a) {
    return a->origin.x;
}

void agg_set_val(Aggregate *a, int idx, int v) {
    if (idx >= 0 && idx < 4) a->vals[idx] = v;
}

int agg_get_val(const Aggregate *a, int idx) {
    if (idx >= 0 && idx < 4) return a->vals[idx];
    return -1;
}

int agg_name_len(const Aggregate *a) {
    return (int)strlen(a->name);
}

void flex_set(FlexBuf *f, int idx, double v) {
    if (idx >= 0 && idx < f->len) f->data[idx] = v;
}

double flex_get(const FlexBuf *f, int idx) {
    if (idx >= 0 && idx < f->len) return f->data[idx];
    return -1.0;
}

FlexBuf *flex_make(int len) {
    FlexBuf *f = (FlexBuf *)calloc(1, sizeof(FlexBuf) + (size_t)len * sizeof(double));
    f->len = len;
    return f;
}
