// Category B: Floating point — equivalent workload to float_loop.lime.
// Prints the float directly with %.16g to match Lime's float formatting exactly.
#include <stdio.h>
int main(void) {
    double s = 0.0;
    double i = 1.0;
    while (i < 2000000.0) {
        s = s + i * 0.5;
        s = s / 1.000001;
        i = i + 1.0;
    }
    printf("%.16g\n", s);
    return 0;
}
