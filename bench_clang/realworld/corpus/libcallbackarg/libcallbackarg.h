// Synthetic fixture: a C function that takes a function-pointer ARGUMENT
// (callback registration) and later invokes it. Exercises the generic
// "Lime fn -> C function-pointer argument" path — no library-specific code.
// Mirrors the shape of FFmpeg's av_log_set_callback(void (*cb)(...)).
#ifndef LIBCALLBACKARG_H
#define LIBCALLBACKARG_H

// Register a callback that takes one int. The C side stores it and calls it
// later from trigger(). A NULL callback is allowed (no-op).
void reg_cb(void (*cb)(int));

// Invoke the registered callback with x. Returns x*2 if a callback is set
// (the callback receives x and we return its effect proxy), else 0.
int trigger(int x);

// Direct callback invocation probe: call cb(x) and return what the callback
// stored (via a side channel) — here we just return x so the smoke can assert
// the callback ran by having the callback print.
void invoke_direct(void (*cb)(int), int x);

#endif
