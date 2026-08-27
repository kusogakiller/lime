#ifndef LIBPARAMTYPES_H
#define LIBPARAMTYPES_H

#ifdef __cplusplus
extern "C" {
#endif

/* Iteration 30 regression fixture: raw C functions whose parameters use
 * multi-token base-type spellings (`unsigned char`, `signed char`) — the
 * shape that strip_param_name previously mangled to a single token
 * ("unsigned"), producing Other("unsigned") CType, an Opaque(unsigned) Lime
 * parameter, and an iface reference to a lime_val_* shim that was never
 * emitted (dangling symbol). Also covers struct by-value return/argument,
 * which legitimately require the lime_ret_/lime_val_ wrappers. */

typedef struct PTPoint {
    long long x;
    long long y;
} PTPoint;

unsigned char ptt_uc(unsigned char v);
signed char    ptt_sc(signed char v);
unsigned int   ptt_ui(unsigned int v);
long long      ptt_ll(long long v);
double         ptt_d(double v);

PTPoint ptt_make(long long x, long long y);          /* by-value return */
long long ptt_sum(PTPoint p);                        /* by-value argument */

#ifdef __cplusplus
}
#endif

#endif
