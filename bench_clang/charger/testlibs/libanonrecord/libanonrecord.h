#ifndef LIBANONRECORD_H
#define LIBANONRECORD_H

// Iteration 9 fixture: a struct containing an *anonymous union* as a member.
// clang emits the union body inline (unnamed RecordDecl) plus an implicit field
// `union Parent::(anonymous at ...)`. Charger must flatten the union's members
// into the parent struct (keeping the widest member) and surface them as opaque
// accessors — not drop them or reuse a stale anonymous-record body.
typedef struct {
    int kind;
    union {
        int i;
        float f;
    };
} AnonUnion;

// Fill kind and the union's `i` member and return kind*1000 + i. Exercises the
// flattened anonymous-union member through the real struct layout.
int anon_demo(AnonUnion *v);

#endif
