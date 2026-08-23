#include "libfreshprobe.h"
#include <stdlib.h>
#include <stdarg.h>

int fresh_add(int x) {
    return x + 1;
}

FreshProbe* fresh_make(void) {
    FreshProbe* p = (FreshProbe*)malloc(sizeof(FreshProbe));
    p->value = 41;
    return p;
}

int fresh_get(FreshProbe* p) {
    return p->value;
}

FreshHandle fresh_hmake(void) {
    static struct FreshHandle_ g = { 7 };
    return &g;
}

int fresh_vdummy(int count, ...) {
    va_list ap; va_start(ap, count);
    int s = 0; int i;
    for (i = 0; i < count; i++) s += va_arg(ap, int);
    va_end(ap);
    return s;
}
