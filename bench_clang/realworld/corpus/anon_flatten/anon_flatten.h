#ifndef ANON_FLATTEN_H
#define ANON_FLATTEN_H

// Test cases for Iteration 18: generic anonymous record flattening.

// direct anonymous struct
struct DirectAnonStruct {
    int a;
    struct {
        int b;
        long c;
    };
    int d;
};

// nested anonymous union (union field with anon struct member)
struct NestedAnonUnion {
    int a;
    union {
        int x;
        long y;
        struct {
            int z;
        };
    };
    int f;
};

// anonymous union inside anonymous struct
struct AnonUnionInAnonStruct {
    int a;
    struct {
        int b;
        union {
            int p;
            long q;
        };
    };
    int c;
};

// anonymous union whose widest member is itself an inner struct
// (priority: named scalar beats anon-struct leaf)
struct AnonUnionWidestStruct {
    int a;
    union {
        int x;
        struct {
            int z1;
            int z2;
        };
    };
    int b;
};

// Anchor symbol so the prepared artifact links even for a struct-only corpus
// (manifest `symbols` is otherwise empty and `lime build` would skip the store).
int anon_flatten_link_anchor(void);

#endif
