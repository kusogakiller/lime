#ifndef LIBCALLBACKTAIL_H
#define LIBCALLBACKTAIL_H

// Iteration 13 adversarial callback-tail fixture.
// Exercises the callback + trailing-argument ABI hole: a function-pointer
// parameter followed by other parameters must NOT be blindly dropped.
typedef void (*cb_t)(int);

// A. callback setter / registration (last param)
int callback_last(cb_t cb);

// B. callback + userdata (tail after callback)
int callback_tail(cb_t cb, void *userdata);

// C. callback + userdata + scalar tail
int callback_tail_int(cb_t cb, void *userdata, int value);

// D. callback + ordinary scalar argument (after callback, not userdata)
int callback_before(cb_t cb, int value);

// E. callback + two trailing pointers
int callback_two_tail(cb_t cb, void *userdata, void *extra);

// F. callback returning a value through the callback (callback arg only)
int callback_return(cb_t cb);

// G. callback + const userdata
int callback_const_userdata(cb_t cb, const void *userdata);

// H. INLINE function pointer (no typedef) + tail — this is the shape that
//    reaches CType::Function and triggers the drop_from heuristic.
int callback_inline_tail(void (*cb)(int), void *userdata);

// I. INLINE function pointer (no typedef), last param
int callback_inline_last(void (*cb)(int));

// Helper used by the implementation to drive invocation.
void invoke_cb(cb_t cb, int v);

#endif /* LIBCALLBACKTAIL_H */
