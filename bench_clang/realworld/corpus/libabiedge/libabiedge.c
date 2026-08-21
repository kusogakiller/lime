#include "libabiedge.h"

void take_2d_array(int (*arr)[4], int rows) {
    // touch element to prove the decayed-pointer path works
    (void)arr[0][0];
    (void)rows;
}

int read_nested_anon_i(struct nested_anon *s) { return s->i; }
int read_nested_anon_hi(struct nested_anon *s) { return s->bits.hi; }

int enum_width_test(enum small_enum e) { return (int)e; }

int take_const_ptr(const int *p) { return *p; }
int take_volatile_ptr(volatile int *p) { return *p; }
int take_alias(int_alias_t v) { return v; }

void call_log(log_fn_t fn, const char *msg) { if (fn) fn(msg); }
