#include "libcallbackreturn.h"

static callback_int_t g_cb = 0;
static callback_str_t g_str_cb = 0;
static callback_ud_t g_ud_cb = 0;
static void *g_ud = 0;

void register_callback(callback_int_t cb) { g_cb = cb; }

int trigger_callback(int input) {
    if (!g_cb) return -1;
    // callback return value feeds the C output: result = input + cb(input)
    return input + g_cb(input);
}

void register_str_callback(callback_str_t cb) { g_str_cb = cb; }

int trigger_str_callback(int input) {
    if (!g_str_cb) return -1;
    const char *s = g_str_cb(input);
    if (!s) return 0;
    int n = 0;
    while (s[n]) n++;
    return n;
}

void register_ud_callback(callback_ud_t cb, void *userdata) {
    g_ud_cb = cb;
    g_ud = userdata;
}

int trigger_ud_callback(int input) {
    if (!g_ud_cb) return -1;
    g_ud_cb(input);
    return (int)(long)g_ud;
}
