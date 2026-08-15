#ifndef LIBPTR_H
#define LIBPTR_H

// Pointer / opaque-handle vertical-slice library for Charger #2.
// `Counter` is a heap-allocated struct passed around by pointer so the Lime
// side can observe mutation through the pointer (not a by-value copy).

typedef struct {
    long long v;
} Counter;

// Returns a pointer (opaque handle on the Lime side). Allocates on the C heap.
Counter* make_counter();

// Mutates the pointed-to struct through the pointer.
void counter_inc(Counter* c);

// Reads the pointed-to struct's field through the pointer.
long long counter_get(Counter* c);

// Frees the allocation.
void counter_free(Counter* c);

#endif
