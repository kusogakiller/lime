// Category D: Recursion (tree form) — equivalent to recursion_tree.lime
#include <stdio.h>
static long long rsum(long long n) {
    if (n <= 1) return 1;
    return n + rsum(n - 1);
}
int main(void) {
    printf("%lld\n", rsum(20000LL));
    return 0;
}
