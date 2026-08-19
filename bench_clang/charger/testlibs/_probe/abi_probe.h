#include <stddef.h>

// 1. Bitfield
struct Bitfield {
    int a : 3;
    unsigned b : 5;
    int normal;
};

// 2. Flexible array member
struct Buffer {
    size_t len;
    char data[];
};

// 3. Packed
#pragma pack(push, 1)
struct Packed {
    char a;
    int b;
};
#pragma pack(pop)

// 4. Anonymous union
struct AnonUnion {
    int kind;
    union {
        int i;
        float f;
    };
};

// 5. Callback table (fn ptr fields)
struct CallbackTable {
    int (*open)(void*);
    void (*close)(int);
};
