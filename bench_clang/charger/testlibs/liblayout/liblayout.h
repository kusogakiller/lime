#ifndef LIBLAYOUT_H
#define LIBLAYOUT_H

// Phase 1 ABI introspection test library: exact struct layout, union ABI,
// bitfield ABI. Charger must extract sizeof/alignof/offsetof from clang's
// record-layout dump and surface layouts that match the C ABI exactly (no
// silent layout guessing).

// Padded struct: char a; int b; short c;  -> sizeof 12, align 4 on SysV/Win64
typedef struct {
    char a;
    int b;
    short c;
} Padded;

// Nested struct
typedef struct {
    Padded p;
    int tag;
} Wrapper;

// Union: all members overlap at offset 0; sizeof == max member
typedef union {
    int i;
    float f;
    char bytes[4];
    long long ll;
} U;

// Bitfield
typedef struct {
    unsigned ready:1;
    unsigned error:1;
    unsigned mode:3;
    unsigned reserved:27;
} Flags;

// API using the above (proves exact layout is honored at the ABI level)
long long padded_sum(const Padded *p);   // returns a + b + c (byte-reinterpreted)
int u_as_int(const U *u);                 // returns u->i
int flags_ready(const Flags *f);          // returns f->ready
int flags_mode(const Flags *f);           // returns f->mode
int wrapper_tag(const Wrapper *w);        // returns w->tag

#endif
