#include "libptr.h"
#include <stdlib.h>

Counter* make_counter() {
    Counter* c = (Counter*)malloc(sizeof(Counter));
    c->v = 0;
    return c;
}

void counter_inc(Counter* c) {
    c->v = c->v + 1;
}

long long counter_get(Counter* c) {
    return c->v;
}

void counter_free(Counter* c) {
    free(c);
}
