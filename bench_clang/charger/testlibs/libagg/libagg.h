#ifndef LIBAGG_H
#define LIBAGG_H

// Phase 1 Iteration 3: arrays + nested aggregates.
// Charger must surface fixed-size array fields, nested struct fields, and
// flexible array members through the existing Opaque + accessor-shim
// mechanism. No new Lime type categories (Architecture Gate respected).

// A struct with a nested struct field, a fixed-size array field, and a scalar.
typedef struct {
    int x;
    int y;
} Point2;

typedef struct {
    Point2 origin;        // nested struct field
    int count;            // scalar
    int vals[4];          // fixed-size array field
    char name[16];        // fixed-size char array field
} Aggregate;

// Flexible array member (must be last; allocated with extra room).
typedef struct {
    int id;
    int len;
    double data[];        // flexible array member
} FlexBuf;

// C-side accessors to prove ABI round-trip (these read/write through the real
// C layout, which Charger's shims must match).
int agg_origin_x(const Aggregate *a);
void agg_set_val(Aggregate *a, int idx, int v);
int agg_get_val(const Aggregate *a, int idx);
int agg_name_len(const Aggregate *a);
void flex_set(FlexBuf *f, int idx, double v);
double flex_get(const FlexBuf *f, int idx);
FlexBuf *flex_make(int len);

#endif
