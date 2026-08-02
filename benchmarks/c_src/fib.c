#include <stdio.h>
long long fib_recursive(int n) {
    if (n <= 1) return n;
    return fib_recursive(n - 1) + fib_recursive(n - 2);
}
long long fib_iterative(int n) {
    if (n <= 1) return n;
    long long a = 0, b = 1;
    for (int i = 2; i <= n; i++) {
        long long tmp = a + b;
        a = b;
        b = tmp;
    }
    return b;
}
int main() {
    long long r = fib_recursive(30);
    printf("recursive fib(30) = %lld\n", r);
    long long r2 = fib_iterative(100000);
    printf("iterative fib(100000) = %lld\n", r2);
    return 0;
}