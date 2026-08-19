#include "libpacked.h"

int packed_demo(Packed *v) {
    v->a = (char)10;
    v->b = 200;
    volatile char ca = v->a;
    volatile int cb = v->b;
    return (int)ca + cb;   // 10 + 200 = 210
}
