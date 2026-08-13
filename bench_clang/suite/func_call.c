// Category C: Function calls — equivalent workload to func_call.lime
#include <stdio.h>
static long long sq(long long a) { return a * a; }
static long long add(long long a, long long b) { return a + b; }
static long long nested(long long a) { return add(sq(a), sq(a + 1)); }
int main(void) {
    long long total = 0;
    long long i = 0;
    while (i < 20000000LL) {
        total = total + nested(i % 1000);
        i = i + 1;
    }
    printf("%lld\n", total);
    return 0;
}
