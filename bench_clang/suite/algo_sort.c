// Category J (algorithm): in-place quicksort — equivalent to algo_sort.lime
#include <stdio.h>
#include <stdlib.h>
static long long *A;
static void qsort_(long long lo, long long hi) {
    if (lo >= hi) return;
    long long pivot = A[(lo + hi) / 2];
    long long l = lo, r = hi;
    while (l <= r) {
        while (A[l] < pivot) l = l + 1;
        while (A[r] > pivot) r = r - 1;
        if (l <= r) {
            long long t = A[l]; A[l] = A[r]; A[r] = t;
            l = l + 1; r = r - 1;
        }
    }
    qsort_(lo, r); qsort_(l, hi);
}
int main(void) {
    long long n = 5000;
    A = malloc(n * sizeof(long long));
    long long i = 0;
    while (i < n) { A[i] = (i * 2654435761LL) % 1000000; i = i + 1; }
    qsort_(0, n - 1);
    long long total = 0; long long k = 0;
    while (k < n) { total = total + A[k]; k = k + 1; }
    printf("%lld\n", total);
    free(A);
    return 0;
}
