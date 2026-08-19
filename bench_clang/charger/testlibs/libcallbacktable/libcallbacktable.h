#ifndef LIBCALLBACKTABLE_H
#define LIBCALLBACKTABLE_H

// Iteration 9 fixture: a struct whose members are C function pointers ("callback
// table"). Charger must extract the function-pointer fields, surface them as
// `Callback`-typed setters on the opaque handle, and allow a Lime function to be
// registered and invoked through the table — all generically (no library names).
typedef struct {
    int (*op)(int);
    int (*cl)(int);
} CallbackTable;

// Invoke both registered callbacks on x and return op(x) + cl(x). Exercises the
// function-pointer fields through the real struct layout / ABI.
int cbt_invoke(CallbackTable *t, int x);

#endif
