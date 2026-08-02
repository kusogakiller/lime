#include <stdio.h>
static inline long long identity_long(long long x) { return x; }
int main(int argc, char **argv) {
    long long sum = argc;
    for (long long i = 0; i < 50000000; i++)
        sum = identity_long(sum) + sum / (i + 1);
    printf("generic identity sum = %lld\n", sum);
    return 0;
}