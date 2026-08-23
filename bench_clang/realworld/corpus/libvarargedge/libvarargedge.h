#ifndef LIBVARARGEDGE_H
#define LIBVARARGEDGE_H

#include <stdarg.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct VarDummy { int marker; } *VarHandle;

VarHandle vae_handle(void);
int vae_sum(int n, ...);
int vae_ptr_int(VarHandle a, int b);
double vae_dbl_ptr(double a, double b, VarHandle c);

#ifdef __cplusplus
}
#endif

#endif
