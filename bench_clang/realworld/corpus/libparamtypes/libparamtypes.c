#include "libparamtypes.h"

unsigned char ptt_uc(unsigned char v) { return (unsigned char)(v + 1); }
signed char   ptt_sc(signed char v)  { return (signed char)(v - 1); }
unsigned int  ptt_ui(unsigned int v) { return v * 3u; }
long long     ptt_ll(long long v)    { return v * 2; }
double        ptt_d(double v)        { return v / 2.0; }

PTPoint ptt_make(long long x, long long y) {
    PTPoint p; p.x = x; p.y = y; return p;
}

long long ptt_sum(PTPoint p) { return p.x + p.y; }
