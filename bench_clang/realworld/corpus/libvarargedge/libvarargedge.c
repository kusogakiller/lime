#include "libvarargedge.h"
#include <stdarg.h>
#include <stdlib.h>

VarHandle vae_handle(void) {
    static struct VarDummy g = { 77 };
    return &g;
}

int vae_sum(int n, ...) {
    va_list ap;
    int total = 0;
    int i;
    va_start(ap, n);
    for (i = 0; i < n; i++) {
        total += va_arg(ap, int);
    }
    va_end(ap);
    return total;
}

int vae_ptr_int(VarHandle a, int b) {
    return a->marker + b;
}

double vae_dbl_ptr(double a, double b, VarHandle c) {
    return a + b + (double)c->marker;
}
