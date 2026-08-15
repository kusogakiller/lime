#include "libmathx.h"

long long add(long long a, long long b) {
    return a + b;
}

Point make_point(long long x, long long y) {
    Point p;
    p.x = x;
    p.y = y;
    return p;
}

long long point_sum(Point p) {
    return p.x + p.y;
}

long long apply(long long (*fn)(long long, long long), long long a, long long b) {
    return fn(a, b);
}

long long square(long long n) {
    return n * n;
}
