// Category H: Set ops — equivalent LINEAR set to set_ops.lime (mirrors Lime's HashSet).
#include <stdio.h>
#include <stdlib.h>
static long long *s; static long long n;
static void ls_add(long long item) {
    long long i = 0;
    while (i < n) { if (s[i] == item) return; i = i + 1; }
    s[n] = item; n = n + 1;
}
static int ls_contains(long long item) {
    long long i = 0;
    while (i < n) { if (s[i] == item) return 1; i = i + 1; }
    return 0;
}
int main(void) {
    s = malloc(30000 * sizeof(long long));
    long long i = 0;
    while (i < 20000LL) { ls_add((i * 31) % 100000); i = i + 1; }
    long long total = 0; long long k = 0;
    while (k < 10000LL) { if (ls_contains(k)) total = total + k; k = k + 1; }
    printf("%lld\n", total);
    printf("%lld\n", n);
    free(s);
    return 0;
}
