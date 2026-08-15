#include "liblayout.h"
Padded g_p;
Wrapper g_w;
U g_u;
Flags g_f;
long sink() { return (long)&g_p + (long)&g_w + (long)&g_u + (long)&g_f; }
