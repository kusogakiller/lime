// Iteration 8.5 variadic ABI fixture: float/double/int/long long mixed args.
// Exercises the MSVC x64 variadic calling convention's register-class
// assignment and stack spill (INTEGER vs SSE classes, 4-register caps).
#ifndef LIBVARFLOAT_H
#define LIBVARFLOAT_H

// Sum of `n` float variadic args (tests SSE register class + spill past 4).
float vf_sumf(int n, ...);

// Sum of `n` double variadic args (tests SSE class + spill past 4).
double vf_sumd(int n, ...);

// Mixed: alternating int / double / long long / float across 8 slots, to force
// register-class interleaving and stack spill on both INTEGER and SSE sides.
double vf_mixed8(int n, ...);

// 6 floats -> forces SSE register spill (XMM0..XMM3, then stack).
float vf_sixf(int n, ...);

// 6 doubles -> forces SSE register spill too.
double vf_sixd(int n, ...);

#endif
