#ifndef LIBCALLBACKRETURN_H
#define LIBCALLBACKRETURN_H

// typedef callback with NON-VOID return value.
// Lime provides the callback; C invokes it later and uses the return value.
typedef int (*callback_int_t)(int);

// registration: C stores the callback and calls it later.
void register_callback(callback_int_t cb);

// trigger: C invokes the stored callback with `input`, returns (input + cb(input)).
// This makes the callback's return value observable in the C output.
int trigger_callback(int input);

// ---- pointer-returning typedef callback (Phase 2 coverage) ----
typedef const char *(*callback_str_t)(int);
void register_str_callback(callback_str_t cb);
// returns length of the string the callback produced for `input` (0 if NULL).
int trigger_str_callback(int input);

// ---- callback with userdata tail (Phase 2 coverage) ----
typedef void (*callback_ud_t)(int);
void register_ud_callback(callback_ud_t cb, void *userdata);
int trigger_ud_callback(int input);

#endif
