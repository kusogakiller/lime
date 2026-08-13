// Category D: Recursion (tail form) — equivalent to recursion_tail.lime
#include <stdio.h>
static long long tsum(long long n, long long acc) {
    if (n <= 0) return acc;
    return tsum(n - 1, acc + n);
}
int main(void) {
    printf("%lld\n", tsum(200000LL, 0LL));
    return 0;
}
