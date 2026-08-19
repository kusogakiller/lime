#ifndef LIBFLEXARRAY_H
#define LIBFLEXARRAY_H

#include <stddef.h>

// Iteration 9 fixture: a struct with a flexible array member (`char data[]`).
// Charger must NOT generate an invalid fixed-size accessor and must preserve the
// struct layout (the FAM carries no size in the struct's sizeof).
typedef struct {
    size_t len;
    char data[];        // flexible array member
} Buffer;

// Return the sum of the first `b->len` bytes of the flexible array member. The
// Lime driver allocates via `lime_make_Buffer_flex` and fills data[] through the
// element accessors, exercising the FAM through the real ABI.
int flex_sum(Buffer *b);

#endif
