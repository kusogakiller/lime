#ifndef LIBABIEDGE_H
#define LIBABIEDGE_H

// Multi-dimensional array parameter.
// C: passes as pointer to first element (decayed). Lime sees Pointer.
void take_2d_array(int (*arr)[4], int rows);

// Nested anonymous union inside a struct.
struct nested_anon {
    int tag;
    union {
        int i;
        float f;
        struct {
            short lo;
            short hi;
        } bits;
    };  // anonymous
};

int read_nested_anon_i(struct nested_anon *s);
int read_nested_anon_hi(struct nested_anon *s);

// Enum width: explicit underlying type.
enum small_enum : unsigned char { E_A = 1, E_B = 200, E_C = 255 };
int enum_width_test(enum small_enum e);

// const correctness: const pointer param (ABI == pointer).
int take_const_ptr(const int *p);

// volatile qualifier (ABI == pointer, qualifier dropped).
int take_volatile_ptr(volatile int *p);

// integer type alias (stdint-style).
typedef int int_alias_t;
int take_alias(int_alias_t v);

// function pointer typedef normalization (already GREEN, re-verify).
typedef void (*log_fn_t)(const char *);
void call_log(log_fn_t fn, const char *msg);

#endif
