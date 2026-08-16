#ifndef LIBGVAR_H
#define LIBGVAR_H

// Phase 1 Iteration 4: global variables (general mechanism).
// Charger surfaces C file-scope globals via generated getter/setter shims.

extern int g_counter;        // scalar, mutable, extern
extern int g_max;            // scalar, const (via macro-const below)
extern const int g_const_pi; // const scalar global
extern char *g_msg;          // pointer global (nullable)
extern int g_ints[4];        // fixed array global (element-wise)
extern struct GPoint { int x; int y; } g_origin; // struct global (by address)

// static global (internal linkage). Declared `static` in the header so the
// translation unit that includes it (libgvar.c) gets its own instance; Charger
// injects the accessor into that same TU so it can be reached from Lime without
// exposing the symbol externally (which would be an ABI lie for a static).
static int g_static_secret = 42;

// C-side functions that observe/modify globals (prove ABI round-trip)
int gvar_bump(void);          // g_counter++
int gvar_get_counter(void);   // return g_counter
void gvar_set_counter(int v); // g_counter = v
int gvar_get_static(void);    // return g_static_secret (proves accessor works)
void gvar_set_msg(const char *s); // g_msg = s (observable from C)
const char *gvar_get_msg(void);
int gvar_get_origin_x(void);  // return g_origin.x
void gvar_set_origin(int x, int y);
int gvar_sum_ints(void);      // sum g_ints[0..3]

#endif
