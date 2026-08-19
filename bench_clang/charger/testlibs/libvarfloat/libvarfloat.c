#include "libvarfloat.h"
#include <stdarg.h>

float vf_sumf(int n, ...) {
    va_list ap; va_start(ap, n);
    float total = 0.0f;
    for (int i = 0; i < n; i++) total += (float)va_arg(ap, double); // float promotes to double in ...
    va_end(ap);
    return total;
}

double vf_sumd(int n, ...) {
    va_list ap; va_start(ap, n);
    double total = 0.0;
    for (int i = 0; i < n; i++) total += va_arg(ap, double);
    va_end(ap);
    return total;
}

double vf_mixed8(int n, ...) {
    va_list ap; va_start(ap, n);
    int a = va_arg(ap, int);
    double b = va_arg(ap, double);
    long long c = va_arg(ap, long long);
    float d = (float)va_arg(ap, double);
    int e = va_arg(ap, int);
    double f = va_arg(ap, double);
    long long g = va_arg(ap, long long);
    float h = (float)va_arg(ap, double);
    va_end(ap);
    return (double)a + b + (double)c + (double)d + (double)e + f + (double)g + (double)h;
}

float vf_sixf(int n, ...) {
    va_list ap; va_start(ap, n);
    float t = 0.0f;
    for (int i = 0; i < n; i++) t += (float)va_arg(ap, double);
    va_end(ap);
    return t;
}

double vf_sixd(int n, ...) {
    va_list ap; va_start(ap, n);
    double t = 0.0;
    for (int i = 0; i < n; i++) t += va_arg(ap, double);
    va_end(ap);
    return t;
}
