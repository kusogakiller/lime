#ifndef LIBCB_H
#define LIBCB_H

// Phase 1 Iteration 2: function-pointer tables (CALLBACK TABLES general
// mechanism — no library-specific special cases). Charger must detect
// function-pointer fields inside any C struct and surface them as settable
// native function pointers so Lime callbacks round-trip through a C struct.

// Test B: three callback fields
typedef struct {
    int (*add)(int, int);
    int (*mul)(int, int);
    int (*sub)(int, int);
} MathOps;

// Test C + F + G: callback + userdata, pointer/opaque/const-ptr params
typedef struct {
    void *userdata;
    int (*process)(void *userdata, int value);
} Processor;

// Test D: nullable callback
typedef struct {
    int (*run)(int);
    void (*destroy)(void);
} Lifecycle;

// registration / runner functions
int register_mathops(const MathOps *ops, int a, int b);
int run_processor(const Processor *p);
int run_lifecycle(const Lifecycle *lc, int v);
// retains a pointer to the table and invokes later (Test H)
int install_and_run(const MathOps *ops);

#endif
