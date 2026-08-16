#include "libgvar.h"
#include <string.h>

int g_counter = 0;
const int g_const_pi = 3;
char *g_msg = 0;
int g_ints[4] = { 1, 2, 3, 4 };
int g_max = 100;
struct GPoint g_origin = { 7, 9 };

// internal-linkage global lives in this TU (via the header's `static`
// declaration); Charger injects lime_get/set_g_static_secret into this same TU.

int gvar_bump(void) {
    g_counter++;
    return g_counter;
}

int gvar_get_counter(void) {
    return g_counter;
}

void gvar_set_counter(int v) {
    g_counter = v;
}

int gvar_get_static(void) {
    return g_static_secret;
}

void gvar_set_msg(const char *s) {
    g_msg = (char*)s;
}

const char *gvar_get_msg(void) {
    return g_msg;
}

int gvar_get_origin_x(void) {
    return g_origin.x;
}

void gvar_set_origin(int x, int y) {
    g_origin.x = x;
    g_origin.y = y;
}

int gvar_sum_ints(void) {
    return g_ints[0] + g_ints[1] + g_ints[2] + g_ints[3];
}
