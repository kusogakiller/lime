// Category F: Struct ops — equivalent to struct_ops.lime
#include <stdio.h>
typedef struct { long long x; long long y; } Point;
static Point make(long long a, long long b) { return (Point){a, b}; }
static Point addp(Point p, Point q) { return (Point){p.x + q.x, p.y + q.y}; }
int main(void) {
    long long total = 0;
    long long i = 0;
    while (i < 5000000LL) {
        Point a = make(i, i + 1);
        Point b = make(i + 2, i + 3);
        Point c = addp(a, b);
        total = total + c.x + c.y;
        i = i + 1;
    }
    printf("%lld\n", total);
    return 0;
}
