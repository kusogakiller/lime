typedef struct {
    char a;
    int b;
    short c;
} Padded;
typedef union {
    int i;
    float f;
    char bytes[4];
    long long ll;
} U;
typedef struct {
    unsigned ready:1;
    unsigned error:1;
    unsigned mode:3;
} Flags;
