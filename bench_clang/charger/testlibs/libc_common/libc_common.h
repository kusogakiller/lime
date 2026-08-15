#ifndef LIBC_COMMON_H
#define LIBC_COMMON_H

// Shared utility library used as a dependency by libapp (Charger #7:
// dependency graph). Provides a primitive that libapp's exported function
// calls internally, so linking libapp requires libc_common's artifact too.

long long common_add(long long a, long long b);

#endif
