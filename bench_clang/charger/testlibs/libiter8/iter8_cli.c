#include "iter8.h"
#include <stdio.h>

// (3) `#else`-guarded `main()` TU — the Iteration 8 filter-correctness class.
//
// This file mimics the structure that broke the naive preprocessor-depth fix:
// an OUTER `#ifdef` guard, with a NESTED `#if/#else` block INSIDE it, and the
// `main()` definition living in the outer `#else` branch. The preprocessor
// tracker must:
//   * detect this `main` (it is a real executable entry point) and DROP the
//     whole translation unit from the library build, AND
//   * NOT corrupt the nesting depth of the inner `#if/#else` so that a guarded
//     `main` in a *true* branch elsewhere is still treated as guarded.
//
// If the filter is correct, this file is excluded and the library links with
// exactly iter8.c's symbols. If it were wrongly retained, the archive would
// carry a second `main` and any Lime slice linking it would fail with a
// duplicate-symbol `main` link error.

#if defined(ITER8_ENABLE_CLI)
#  ifndef ITER8_QUIET
#    define ITER8_QUIET 0
#  endif
#else
// standalone CLI entry (like sqlite shell.c / cjpeg). Real program.
int main(void) {
    return 0;
}
#endif /* !ITER8_ENABLE_CLI */

// A library function also lives in this TU at top level (depth 0, after the
// #endif). Because the TU defines `main`, the whole file is dropped — which is
// the correct outcome. It is not referenced by the regression slice.
coord_t cli_helper(coord_t x) { return x * 3.0; }
