#include "liblayout.h"

long long padded_sum(const Padded *p) {
    // a (char) + b (int) + c (short) read through the real C layout.
    return (long long)p->a + p->b + p->c;
}

int u_as_int(const U *u) {
    return u->i;
}

int flags_ready(const Flags *f) {
    return (int)f->ready;
}

int flags_mode(const Flags *f) {
    return (int)f->mode;
}

int wrapper_tag(const Wrapper *w) {
    return w->tag;
}
