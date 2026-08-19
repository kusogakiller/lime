#ifndef LIBPACKED_H
#define LIBPACKED_H

// Iteration 9 fixture: a struct under `#pragma pack(push,1)`. Charger must detect
// the packed layout (clang emits `MaxFieldAlignmentAttr`) and verify the packed
// size/alignment against a clang probe in verify-abi.
#pragma pack(push, 1)
typedef struct {
    char a;
    int b;
} Packed;
#pragma pack(pop)

// Fill both members and return a + b so a Lime driver observes the packed layout
// round-trip (size must be 5, align 1 — not the natural 8/4).
int packed_demo(Packed *v);

#endif
