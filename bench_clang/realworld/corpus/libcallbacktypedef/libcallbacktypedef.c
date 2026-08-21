#include "libcallbacktypedef.h"

static cb_t g_tail = 0;
static cb_ret_t g_ret = 0;
static cb_userdata_t g_ud = 0;
static cb_const_t g_const = 0;
static cb_ptr_t g_ptr = 0;

int callback_tail(cb_t cb, void *userdata) {
    g_tail = cb;
    if (cb) { cb(5); }
    (void)userdata;
    return 22;
}

int callback_return(cb_ret_t cb, void *userdata) {
    g_ret = cb;
    int r = 0;
    if (cb) { r = cb(6, userdata); }
    (void)userdata;
    return 33;
}

int callback_userdata(cb_userdata_t cb, void *userdata) {
    g_ud = cb;
    if (cb) { cb(7, userdata); }
    (void)userdata;
    return 44;
}

int callback_const(cb_const_t cb, void *userdata) {
    g_const = cb;
    if (cb) { cb("hi"); }
    (void)userdata;
    return 55;
}

int callback_ptr(cb_ptr_t cb, void *userdata) {
    g_ptr = cb;
    if (cb) { cb(userdata); }
    return 66;
}
