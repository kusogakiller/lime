#ifndef LIBFRESHPROBE_H
#define LIBFRESHPROBE_H

#include <stdarg.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct FreshProbe {
    int value;
} FreshProbe;

// incomplete-pointer-typedef handle (mirrors libvarargedge VarHandle)
typedef struct FreshHandle_ { int marker; } *FreshHandle;

int fresh_add(int x);
FreshProbe* fresh_make(void);
int fresh_get(FreshProbe* p);
FreshHandle fresh_hmake(void);

// empty variadic function to pull stdarg.h declarations into the header
int fresh_vdummy(int count, ...);

#ifdef __cplusplus
}
#endif

#endif
