// Category A: Integer arithmetic — equivalent workload to int_loop.lime
#include <stdio.h>
int main(void) {
    long long acc = 0;
    long long i = 0;
    while (i < 100000000LL) {
        acc = acc + (i * 3) % 17;
        i = i + 1;
    }
    printf("%lld\n", acc);
    return 0;
}
