#include <stdio.h>
long long noop(long long x) { return x; }
int main(int argc, char **argv) {
    long long sum = argc;
    for (long long i = 0; i < 100000000; i++)
        sum = noop(sum) + sum / (i + 1);
    printf("sum = %lld\n", sum);
    return 0;
}