// Category H: Map ops — equivalent LINEAR map (parallel int arrays) to map_ops.lime.
// Remove phase omitted to match the Lime benchmark (no List pop/truncate in Lime build).
#include <stdio.h>
#include <stdlib.h>
static long long *keys; static long long *vals; static long long n;
static void lm_insert(long long key, long long val) { keys[n] = key; vals[n] = val; n = n + 1; }
static long long lm_get(long long key) {
    long long i = 0;
    while (i < n) { if (keys[i] == key) return vals[i]; i = i + 1; }
    return 0;
}
int main(void) {
    keys = malloc(10000 * sizeof(long long));
    vals = malloc(10000 * sizeof(long long));
    long long i = 0;
    while (i < 5000LL) { lm_insert(i, i * 3); i = i + 1; }
    long long total = 0; long long k = 0;
    while (k < 5000LL) { total = total + lm_get(k); k = k + 1; }
    printf("%lld\n", total);
    printf("%lld\n", n);
    free(keys); free(vals);
    return 0;
}
