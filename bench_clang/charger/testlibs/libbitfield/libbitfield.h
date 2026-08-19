#ifndef LIBBITFIELD_H
#define LIBBITFIELD_H

// Iteration 9 fixture: a struct with real C bitfields. Charger must detect the
// bitfield members (clang emits `isBitfield`, not `bitWidth`) and surface the
// struct as an opaque handle, skipping accessors for the sub-byte (unrepresentable)
// bitfield fields while still laying the struct out correctly (verify-abi).
typedef struct {
    int a : 3;        // signed 3-bit field
    unsigned b : 5;   // unsigned 5-bit field
    int normal;       // ordinary scalar member (no bit-width)
} Bitfield;

// Fill the struct's members (exercises the bitfield storage) and return
// n*100 + a*10 + b, so a Lime driver observes the in-memory layout round-trip.
int bf_demo(Bitfield *v);

#endif
