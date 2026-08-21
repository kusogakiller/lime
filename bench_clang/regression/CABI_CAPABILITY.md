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

## Real-world libraries (permanent gate: PASS=8 FAIL=0)
zlib, libpng, sqlite, libjpeg, curl, SDL2, FFmpeg, libcallbackarg — all GREEN.

## Known open issues (NOT counted as GREEN)
- **typedef fn-pointer into `Opaque(name)` arg**: passing a Lime fn into a
  `cb_t`-typedef function pointer surfaced as `Opaque(cb_t)` triggers a Lime
  codegen bug (`inttoptr i64 to i64`, invalid). Inline `void (*)(...)` params
  (surfaced as `Callback`) are unaffected and work end-to-end. This is a Lime
  codegen issue distinct from the callback-tail fix; tracked separately, out of
  Iteration 13 scope. The opaque-pointer *normalization* for `cb_t` is correct;
  only the Lime->C fn-pointer *passing* path for the `Opaque(cb_t)` spelling is
  broken.

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
