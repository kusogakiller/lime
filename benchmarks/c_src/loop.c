#include <stdio.h>
int main(int argc, char **argv) {
    long long sum = argc;
    for (long long i = 0; i < 50000000; i++)
        sum = sum + i * 3 - 1 + sum / (i + 1);
    printf("sum = %lld\n", sum);
    return 0;
}