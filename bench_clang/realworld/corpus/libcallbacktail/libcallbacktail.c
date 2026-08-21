#include "libcallbacktail.h"

/* Minimal adversarial implementation. Stores the last-registered callback so
 * invoke_cb / the *_tail functions can actually call back into Lime, proving
 * the callback argument is NOT dropped at the Charger boundary. */
static cb_t g_last_cb = 0;

int callback_last(cb_t cb) {
    g_last_cb = cb;
    if (cb) { cb(1); }
    return 11;
}

int callback_tail(cb_t cb, void *userdata) {
    g_last_cb = cb;
    if (cb) { cb(2); }
    (void)userdata;
    return 22;
}

int callback_tail_int(cb_t cb, void *userdata, int value) {
    g_last_cb = cb;
    if (cb) { cb(value); }
    (void)userdata;
    return 33;
}

int callback_before(cb_t cb, int value) {
    g_last_cb = cb;
    if (cb) { cb(value); }
    return 44;
}

int callback_two_tail(cb_t cb, void *userdata, void *extra) {
    g_last_cb = cb;
    if (cb) { cb(5); }
    (void)userdata; (void)extra;
    return 55;
}

int callback_return(cb_t cb) {
    g_last_cb = cb;
    if (cb) { cb(6); }
    return 66;
}

int callback_const_userdata(cb_t cb, const void *userdata) {
    g_last_cb = cb;
    if (cb) { cb(7); }
    (void)userdata;
    return 77;
}

int callback_inline_tail(void (*cb)(int), void *userdata) {
    g_last_cb = (cb_t)cb;
    if (cb) { cb(8); }
    (void)userdata;
    return 88;
}

int callback_inline_last(void (*cb)(int)) {
    g_last_cb = (cb_t)cb;
    if (cb) { cb(9); }
    return 99;
}

void invoke_cb(cb_t cb, int v) {
    if (cb) { cb(v); }
}
