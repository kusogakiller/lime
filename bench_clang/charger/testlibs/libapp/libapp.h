#ifndef LIBAPP_H
#define LIBAPP_H

// Application library that DEPENDS on libc_common (Charger #7: dependency graph).
// `app_compute` calls `common_add` (from libc_common) internally, so linking
// libapp requires libc_common's native artifact to be present.

long long app_compute(long long x);

#endif
