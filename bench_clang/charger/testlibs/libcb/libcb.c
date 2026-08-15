#include "libcb.h"

// Test B: invokes all three callbacks through the table
int register_mathops(const MathOps *ops, int a, int b) {
    return ops->add(a, b) + ops->mul(a, b) + ops->sub(a, b);
}

// Test C: C passes userdata from the table into the callback
int run_processor(const Processor *p) {
    return p->process(p->userdata, 41);
}

// Test D: nullable callback — must NOT call destroy if NULL
int run_lifecycle(const Lifecycle *lc, int v) {
    int r = lc->run(v);
    if (lc->destroy != 0) {
        lc->destroy();
    }
    return r;
}

static const MathOps *g_retained = 0;

// Test H: C retains the table pointer and invokes it on a later call
int install_and_run(const MathOps *ops) {
    g_retained = ops;
    return g_retained->add(10, 20) + g_retained->mul(10, 20);
}
