#include <stdio.h>
typedef struct { long long x, y, z; } Point;
Point make_point(long long x, long long y, long long z) {
    Point p = {x, y, z};
    return p;
}
int main(int argc, char **argv) {
    long long sum = argc;
    for (long long i = 0; i < 5000000; i++) {
        Point p = make_point(i, i * 2, i * 3);
        sum = sum + p.x + p.y + p.z + sum / (i + 1);
    }
    printf("sum = %lld\n", sum);
    return 0;
}