#include "libvariadic.h"
#include <string.h>

static struct Dummy s_dummy; // stable address for var_handle()

Handle var_handle(void) {
    return &s_dummy;
}

int var_sum(int count, ...) {
    va_list ap;
    va_start(ap, count);
    int total = 0;
    for (int i = 0; i < count; i++) {
        total += va_arg(ap, int);
    }
    va_end(ap);
    return total;
}

long long var_sum_i64(int count, ...) {
    va_list ap;
    va_start(ap, count);
    long long total = 0;
    for (int i = 0; i < count; i++) {
        total += va_arg(ap, long long);
    }
    va_end(ap);
    return total;
}

double var_sum_double(int count, ...) {
    va_list ap;
    va_start(ap, count);
    double total = 0.0;
    for (int i = 0; i < count; i++) {
        total += va_arg(ap, double);
    }
    va_end(ap);
    return total;
}

int var_strlen_sum(int count, ...) {
    va_list ap;
    va_start(ap, count);
    int total = 0;
    for (int i = 0; i < count; i++) {
        const char *s = va_arg(ap, const char *);
        total += (int)strlen(s);
    }
    va_end(ap);
    return total;
}

int var_echo_ptr(void *base, ...) {
    va_list ap;
    va_start(ap, base);
    void *p = va_arg(ap, void *);
    va_end(ap);
    return (p == base) ? 1 : 0;
}

int var_mixed(int n, ...) {
    va_list ap;
    va_start(ap, n);
    int i = va_arg(ap, int);
    double d = va_arg(ap, double);
    void *p = va_arg(ap, void *);
    va_end(ap);
    int bits = 0;
    if (i > 0) bits |= 1;
    if (d > 0.0) bits |= 2;
    if (p != 0) bits |= 4;
    return bits;
}

int var_promote(int count, ...) {
    va_list ap;
    va_start(ap, count);
    int sig = 0;
    for (int i = 0; i < count; i++) {
        if (i % 2 == 0) {
            int v = va_arg(ap, int);      // char/short promote to int
            if (v > 0) sig |= (1 << i);
        } else {
            double v = va_arg(ap, double); // float promotes to double
            if (v > 0.0) sig |= (1 << i);
        }
    }
    va_end(ap);
    return sig;
}

int var_sum_enum(int count, ...) {
    va_list ap;
    va_start(ap, count);
    int total = 0;
    for (int i = 0; i < count; i++) {
        total += va_arg(ap, int); // enum's underlying int value
    }
    va_end(ap);
    return total;
}
