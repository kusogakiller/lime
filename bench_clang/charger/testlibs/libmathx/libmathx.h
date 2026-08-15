#ifndef LIBMATHX_H
#define LIBMATHX_H

#ifdef __cplusplus
extern "C" {
#endif

// NOTE: fields/params use `long long` so the C struct/ABI layout matches
// Lime's `Int` (which lowers to LLVM i64). This keeps the vertical-slice ABI
// check honest: Lime `struct Point { int, int }` is 16 bytes (2xi64) and the
// C `Point` must be the same size/order for by-value params and returns.
typedef struct {
    long long x;
    long long y;
} Point;

long long add(long long a, long long b);
Point make_point(long long x, long long y);
long long point_sum(Point p);
long long apply(long long (*fn)(long long, long long), long long a, long long b);
long long square(long long n);

#ifdef __cplusplus
}
#endif

#endif
