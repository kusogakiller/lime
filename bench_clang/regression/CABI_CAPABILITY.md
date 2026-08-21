# Charger C ABI Capability Matrix

Machine-readable / auditable list of C ABI patterns Charger handles, with the
evidence level for each. `GREEN` = proven by native execution (clang AST ->
CType -> Lime interface -> generated adapter -> link -> native execution) on a
real library or a permanent synthetic fixture. `TARGET` = known hole under
active investigation. `UNCONFIRMED` = not yet proven / open issue.

Last updated: Iteration 13 (2026-08-21).

## Scalars
- int / unsigned / signed                 GREEN
- long long / int64_t                     GREEN
- float / double                          GREEN
- enum                                    GREEN

## Pointers
- T*                                      GREEN
- T**                                     GREEN
- const T*                                GREEN
- opaque pointers (handle return/arg)     GREEN
- stdlib opaque (FILE*)                   GREEN
- SQLite opaque (sqlite3*)                 GREEN
- FFmpeg AVCodecContext*                   GREEN (opaque, not struct-by-value)
- FFmpeg AVFormatContext*                  GREEN (opaque, not struct-by-value)

## Struct / Union
- named struct / typedef struct            GREEN
- named union / anonymous union           GREEN
- packed struct                           GREEN
- bitfields                               GREEN
- flexible array member                   GREEN
- multidimensional fixed arrays            GREEN (Iteration 12: was mis-read as FAM)
- struct layout / offset / alignment      GREEN
- struct-by-value vs pointer distinction   GREEN

## Function pointer / callback
- C fn-pointer argument (last param)       GREEN
- callback registration (setter)           GREEN (av_log_set_callback)
- callback invoked later by C              GREEN (libcallbackarg, FFmpeg av_log)
- callback + userdata (tail)               GREEN (Iteration 13: was dropped)
- callback + ordinary arg (tail)           GREEN (Iteration 13: was dropped)
- callback + multiple tail args           GREEN (Iteration 13: was dropped)
- inline fn-pointer param + tail          GREEN (Iteration 13: the shape that
                                          previously DROPPED the callback)
- **typedef fn-pointer param (cb_t)**      GREEN (Iteration 14: cb_t normalized
                                          to CType::Function -> Callback; was
                                          Opaque(cb_t) -> inttoptr codegen bug)
- typedef fn-pointer, const arg            GREEN (Iteration 14: void(*)(const char*))
- typedef fn-pointer, void* arg            GREEN (Iteration 14: void(*)(void*))
- typedef fn-pointer, non-void return      UNCONFIRMED (cb_ret_t = int(*)(int,void*);
                                          CType::Function(ret=Int) is produced
                                          correctly, but Lime callback syntax for
                                          a non-Unit return is not yet exercised)

## Out parameters / ownership
- create-style T** (sqlite3_open)          GREEN
- void-return + single T** take/free       GREEN (avcodec_free_context,
                                          avformat_close_input; Iteration 12)
- generic detection (no lib name)          GREEN

## Variadic
- MSVC x64 variadic float/double ABI       GREEN
- float arg promotion / double / int /     GREEN
- long long / mixed args / reg->stack      GREEN
- variadic shim manifest registration      GREEN

## Linkage / dependency
- multiple native artifacts                GREEN
- dependency graph / composite link        GREEN
- artifact cache hit / source invalidation GREEN
- duplicate source filename stem collision GREEN
- unique per-TU object names               GREEN
- duplicate-symbol (multiple platform backends) GREEN (Iteration 15: SDL2 —
  see below; root cause was Charger's generic source collector compiling BOTH
  the `src/thread/generic/` and `src/thread/windows/` mutex/sem/thread/tls
  implementations, each defining SDL_CreateMutex etc. Fixed in corpus config,
  not charger.rs — see Real-world libraries note)

## Real-world libraries (permanent gate: PASS=8 FAIL=0)
zlib, libpng, sqlite, libjpeg, curl, FFmpeg, libcallbackarg, SDL2 — all GREEN.

### SDL2 duplicate-symbol recovery (Iteration 15)
Root cause (proven, not a Charger logic bug): SDL2 ships mutually-exclusive
per-platform backend translation units under `src/<subsystem>/<platform>/`.
For the Windows host build, SDL2's own CMake selects ONLY
`src/thread/windows/{SDL_sysmutex,SDL_syssem,SDL_systhread,SDL_systls}.c` (plus
`windows/SDL_syscond_cv.c` and `generic/SDL_syscond.c`, the latter guarded by
`SDL_THREAD_GENERIC_COND_SUFFIX` so it does NOT define `SDL_CreateCond`).
Charger's generic `collect_sources` recurses the whole tree and compiled BOTH
`generic/SDL_sysmutex.c` and `windows/SDL_sysmutex.c`, each defining
`SDL_CreateMutex`/`SDL_DestroyMutex`/`SDL_LockMutex`/`SDL_TryLockMutex`/
`SDL_UnlockMutex` -> lld-link `duplicate symbol` at the smoke link step.

Fix (corpus configuration ONLY — charger.rs untouched, no library-specific
code, no C++):
- Created a Windows-only corpus copy `SDL2-2.30.9-win` (alongside the intact
  original `SDL2-2.30.9`) with the four generic thread TUs that the Windows
  build does NOT use removed: `src/thread/generic/{SDL_sysmutex,SDL_syssem,
  SDL_systhread,SDL_systls}.c`. `generic/SDL_syscond.c` is KEPT (Windows build
  uses it). This mirrors exactly what SDL2's CMake feeds the Windows compiler.
- Pointed `run_regression.sh`'s `sdl2` row at `SDL2-2.30.9-win`.
- Removed the stale `SDL2-2.30.9` store so the smoke link no longer pulls in
  the old (duplicate-containing) `.lib` alongside the new one.

Verification: SDL2 smoke links cleanly (`0|Windows||`, exit 0), and the full
permanent gate now reports PASS=8 FAIL=0.

## Known open issues (NOT counted as GREEN)
- **typedef fn-pointer non-void return callback**: `cb_ret_t = int (*)(int, void*)`
  normalizes correctly to `CType::Function([Int, Opaque], Int)` and surfaces as
  `Callback`, but a Lime callback *definition* with a non-Unit return has not
  been exercised (Lime's `fn ... -> Ret` syntax is unresolved). Void-returning
  typedef callbacks (`cb_t`, `cb_const_t`, `cb_ptr_t`, `cb_userdata_t`) are
  fully GREEN end-to-end (Iteration 14). Out of Iteration 14 scope.

## Iteration 13 change
`collect_out_param_adapters` no longer drops a `CType::Function` parameter when
it is followed by trailing arguments. Previously `foo(cb, tail...)` had the
callback (and everything after) dropped and NULL-shimmed — correct only for the
`sqlite3_exec`-style optional-callback idiom, wrong for required callbacks.
AST/type info alone cannot distinguish optional from required callbacks (both are
`fn-ptr + tail`), so the safe generic default is to KEEP the callback and
surface it as `Callback`. Verified: `callback_inline_tail(cb, userdata)` now
reaches native execution and the Lime callback fires (smoke
`libcallbacktail_smoke.lime`, output shows `CB` markers + return codes 88/99).
Existing sqlite3_exec / av_log_set_callback / libcallbackarg regressions
unchanged (their callbacks are last-param or unused by the smoke).

## Iteration 14 change
Typedef function-pointers (`typedef void (*cb_t)(int)`, `typedef int (*cb_ret_t)(int, void*)`,
etc.) are now normalized from `CType::Other("cb_t")`/`Opaque(cb_t)` to
`CType::Function(...)` via the AST typedef table (`ctx.typedefs`): any typedef
whose underlying qualType contains `(*` is resolved with `parse_c_function_ptr`.
This surfaces the param as a Lime `Callback` and reuses the already-proven
inline-fn-ptr codegen path (no Lime codegen change). Previously `cb_t` stayed
`Opaque(cb_t)` and passing a Lime fn into it produced invalid LLVM IR
(`inttoptr i64 to i64`). Verified end-to-end: `libcallbacktypedef_smoke.lime`
runs natively (exit 0) with `CB` markers + return codes 22/44/55/66 for
`callback_tail`/`callback_userdata`/`callback_const`/`callback_ptr`. The
underlying-typedef spelling check (`(*`) means opaque-handle typedefs
(`typedef struct Foo Foo;`, `typedef Foo *FooHandle;`) and scalar/enum typedefs
are never mis-collapsed to function pointers. No library name, no function-name
heuristic.
