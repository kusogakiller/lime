#include "libcallbacktable.h"

int cbt_invoke(CallbackTable *t, int x) {
    int a = t->op(x);
    int b = t->cl(x);
    return a + b;
}
