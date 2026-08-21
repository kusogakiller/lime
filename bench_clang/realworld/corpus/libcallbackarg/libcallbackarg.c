// Synthetic fixture implementation: store a registered callback and invoke it.
#include "libcallbackarg.h"

static void (*g_cb)(int) = 0;

void reg_cb(void (*cb)(int)) {
    g_cb = cb;
}

int trigger(int x) {
    if (g_cb) {
        g_cb(x);
        return x * 2;
    }
    return 0;
}

void invoke_direct(void (*cb)(int), int x) {
    if (cb) cb(x);
}
