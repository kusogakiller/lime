// Category H: List iterate — equivalent to list_iter.lime (N capped at 20000).
#include <stdio.h>
#include <stdlib.h>
int main(void) {
    long long *xs = malloc(20000 * sizeof(long long));
    long long n = 0;
    long long i = 0;
    while (i < 20000LL) {
        xs[n] = (i * 7) % 1000;
        n = n + 1;
        i = i + 1;
    }
    long long total = 0;
    long long k = 0;
    while (k < n) {
        total = total + xs[k];
        k = k + 1;
    }
    printf("%lld\n", total);
    free(xs);
    return 0;
}
