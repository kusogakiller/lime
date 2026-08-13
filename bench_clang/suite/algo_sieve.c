// Category J (algorithm): Sieve of Eratosthenes — equivalent to algo_sieve.lime
#include <stdio.h>
#include <stdlib.h>
int main(void) {
    long long n = 5000;
    char *sieve = malloc((n + 1) * sizeof(char));
    long long i = 0;
    while (i <= n) { sieve[i] = 1; i = i + 1; }
    long long p = 2;
    while (p * p <= n) {
        if (sieve[p]) {
            long long j = p * p;
            while (j <= n) { sieve[j] = 0; j = j + p; }
        }
        p = p + 1;
    }
    long long count = 0; long long k = 2;
    while (k <= n) { if (sieve[k]) count = count + 1; k = k + 1; }
    printf("%lld\n", count);
    free(sieve);
    return 0;
}
