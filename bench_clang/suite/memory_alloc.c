// Category I: Memory ops — equivalent to memory_alloc.lime
#include <stdio.h>
typedef struct { long long x; long long y; } Node;
int main(void) {
    long long total = 0;
    long long i = 0;
    while (i < 1000000LL) {
        Node n = {i, i + 1};
        total = total + n.x + n.y;
        i = i + 1;
    }
    printf("%lld\n", total);
    return 0;
}
