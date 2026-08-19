#include "libanonrecord.h"

int anon_demo(AnonUnion *v) {
    v->kind = 2;
    v->i = 55;
    volatile int vk = v->kind;
    volatile int vi = v->i;
    return vk * 1000 + vi;   // 2*1000 + 55 = 2055
}
