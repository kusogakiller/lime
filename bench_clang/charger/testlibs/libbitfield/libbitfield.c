#include "libbitfield.h"

int bf_demo(Bitfield *v) {
    v->a = 2;
    v->b = 7;
    v->normal = 5;
    volatile int na = v->a;
    volatile int nb = v->b;
    volatile int nn = v->normal;
    return nn * 100 + na * 10 + nb;   // 5*100 + 2*10 + 7 = 527
}
