// Category E: Control flow — equivalent to control_flow.lime
#include <stdio.h>
int main(void) {
    long long total = 0;
    long long i = 0;
    while (i < 50000000LL) {
        if (i % 3 == 0) {
            total = total + i;
        } else if (i % 3 == 1) {
            total = total - i;
        } else if (i % 5 == 0) {
            total = total + (i * 2);
        } else {
            total = total + 1;
        }
        i = i + 1;
    }
    printf("%lld\n", total);
    return 0;
}
