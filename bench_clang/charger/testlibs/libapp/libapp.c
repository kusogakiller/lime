#include "libapp.h"
#include "libc_common.h"

long long app_compute(long long x) {
    // Delegates to the dependency library (libc_common).
    return common_add(x, x);
}
