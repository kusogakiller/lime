// Category H: List push — equivalent to list_push.lime (N capped at 20000).
#include <stdio.h>
#include <stdlib.h>
int main(void) {
    long long *xs = malloc(20000 * sizeof(long long));
    long long i = 0;
    long long n = 0;
    while (i < 20000LL) {
        xs[n] = i % 1000;
        n = n + 1;
        i = i + 1;
    }
    printf("%lld\n", n);
    printf("%lld\n", xs[n - 1]);
    free(xs);
    return 0;
}
