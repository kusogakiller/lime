# FFmpeg integration status (Iteration 11)

## Goal
Add FFmpeg (high-priority C-ABI stress test: opaque structs, callbacks,
pixel/sample formats, large public API surface) to the Charger native-execution
gate, starting from the smallest public API slice.

## Environment / AST investigation (DONE)

- Source: `C:/Users/szzxl/Downloads/ffmpeg-8.1.2` (FFmpeg 8.1.2 "Hoare",
  libavutil 60.26.102 / libavcodec 62.28.102 / libavformat 62.12.102).
- **Pure C**: 3355 `.c`, only 6 `.cpp` (CUDA/compat stubs), 1195 `.h`. Fits the
  C-only Universal C layer.
- **Public API headers hand-written and present** (no `.h.in` templates, unlike
  OpenSSL): `libavutil/avutil.h`, `libavcodec/avcodec.h`, `avformat.h` all exist
  and parse with clang 22 (`-fsyntax-only`, zero errors) once a minimal
  `libavutil/avconfig.h` stub is supplied.
- **No nasm/yasm / no external deps** with `--disable-x86asm` + `--disable-everything`.

## Minimal slice chosen

`libavutil` only (smallest component), version API — no opaque structs, no
callbacks, no out-params:
- `unsigned avutil_version(void);`     -> packed version int
- `const char *av_version_info(void);` -> version string

## Charger changes made (GENERIC, no FFmpeg-specific branch)

1. **`-idirafter` for the library's own header dir** (`src/charger.rs`, native
   compile loop). Previously the dir was added with `-I`, which placed it BEFORE
   the system include dirs on the angle-bracket search path. That let a
   same-named local header shadow a system one: FFmpeg's `libavutil/time.h`
   shadowed the real `<time.h>` (UCRT), so `strftime` was undeclared and the
   build failed. `-idirafter` places the library dir LAST, so `#include <time.h>`
   resolves to the system header while the library's own angle-bracket includes
   (`#include <jinclude.h>` in libjpeg-turbo) still resolve via fallback.
   Validated: 6-library regression gate PASS=*** FAIL=0 after the change
   (the first attempt used `-iquote`, which wrongly broke libjpeg's angle-bracket
   self-includes — fixed to `-idirafter`).
2. **No FFmpeg macro / no `if library==ffmpeg`** anywhere in charger.rs.

## Minimal corpus (hand-built, mirrors existing per-library smoke corpora)

`bench_clang/realworld/corpus/ffmpeg_avutil_version/`
- 152 libavutil headers (flat) + `libavutil/version.c` (the only TU needed for
  the version API).
- Stub `config.h` (HAVE_*/ARCH_*/SIZEOF + FFMPEG_VERSION/CONFIGURATION/LICENSE),
  `libavutil/avconfig.h`, `libavutil/ffversion.h` — these stand in for the files
  `configure` would generate; enough to compile `version.c`.
- `charger.toml` with `api_header = "libavutil/avutil.h"` (the corpus also
  contains the stub `config.h`, which the root-header heuristic would otherwise
  pick as the API header and extract no functions).

## Native execution (GREEN — 2026-08-21)

Full vertical slice verified:
```
avutil.h  -> clang AST -> Charger install (.lib + manifest + lime-iface)
           -> Lime interface -> Lime build/link -> native execution
```
Smoke: `bench_clang/regression/ffmpeg_smoke/ffmpeg_smoke.lime`
- `avutil_version()` -> 3938918 == (60<<16)|(26<<8)|102 == libavutil 60.26.102  ✓
- `av_version_info()` -> "8.1.2-stub" (stub ffversion.h string)              ✓
- exit code 0.

### Callback + variadic slice (GREEN — 2026-08-21)

Second slice exercises a C function-pointer **argument** and a **variadic**
function through the real FFmpeg API. Smoke:
`bench_clang/regression/ffmpeg_smoke/ffmpeg_cb_variadic.lime`
- `av_log_set_callback(mycb)` — `mycb` is a top-level Lime `fn`; Charger surfaces
  the callback arg as `Callback` (the Lime parser turns `Callback` into a raw C
  function pointer). Lime passes `mycb`'s address directly — no closure wrapper.
- `av_log(0, 0, "value=%d\n", 7)` — variadic, surfaced via
  `charger_variadic.json` (`"av_log": "int"`). Charger emits `lime_av_log_v1`.
- Output: `FFMPEG_CB_RAN` then `DONE`, exit 0.
  `FFMPEG_CB_RAN` proves `av_log()` invoked the registered Lime callback — i.e.
  callback registration + MSVC x64 variadic ABI + FFmpeg's real dispatch all work
  end-to-end.
- Generic callback-argument capability independently proven by synthetic fixture
  `bench_clang/realworld/corpus/libcallbackarg` (C stores a Lime fn ptr and calls
  it back; smoke `libcallbackarg_smoke.lime` prints `CALLBACK_RAN` + `TRIGGER_RET:84`).

## Charger changes made (GENERIC, no FFmpeg-specific branch)

1. **`-idirafter` for the library's own header dir** (see minimal-slice writeup).
2. **Callback-parameter preservation** (`src/charger.rs`, the "null-callback
   bridge" in `collect_out_param_adapters`). Previously ANY function-pointer
   parameter was dropped + NULL-shimmed (the `sqlite3_exec(..., cb, data, errmsg)`
   optional-callback idiom). Now a function-pointer parameter is dropped only
   when it is FOLLOWED by further params (the user-data/errmsg tail). A
   **last-position** function-pointer parameter — a callback REGISTRATION/setter
   such as `av_log_set_callback(void (*cb)(...))` — is kept and surfaced as
   `Callback`, so a Lime callback can be passed through. Generic: position-based
   (fn-ptr-is-last vs fn-ptr-has-tail), no library names. Reuses the existing
   `Callback`→raw-fn-ptr Lime path (parser `parse_type` maps `Callback` to
   `fn(...)`; `codegen_extern_call` passes `ptr @funcname` for `fn(...)` params).
   Validated: 6-library regression gate PASS=*** FAIL=0 after the change.
3. No FFmpeg macro / no `if library==ffmpeg` anywhere in charger.rs (grep-audited).

## Minimal corpus (hand-built, mirrors existing per-library smoke corpora)

`bench_clang/realworld/corpus/ffmpeg_avutil_version/`
- 152 libavutil headers (flat) + TUs: `libavutil/version.c`, `log.c`, `bprint.c`,
  `time.c` (added for the callback/variadic slice — `log.c` needs `bprint.c`'s
  `av_bprint*` and `time.c`'s `av_gettime`, and `#include "compat/va_copy.h"`).
- Stub `config.h` / `libavutil/avconfig.h` / `libavutil/ffversion.h`; FFmpeg-root
  `compat/` copied into the corpus root.
- `charger.toml`: `api_header = "libavutil/avutil.h"`, `build_flags = ["-I."]`
  (corpus root on the include path so `compat/va_copy.h` resolves).
- `libavutil/charger_variadic.json`: `{"av_log": "int"}` — registers `av_log`'s
  variadic slot shape (generic mechanism; no charger.rs change).

## Status

- **Minimal slice (libavutil version API): NATIVE EXECUTION GREEN.**
- **Callback-argument (`av_log_set_callback`): GREEN** — Lime fn passed as C fn ptr.
- **Variadic (`av_log`): GREEN** — surfaced via `charger_variadic.json`.
- **Callback + variadic combined (`av_log` fires the registered Lime cb): GREEN.**
- Full FFmpeg (libavcodec/libavformat, opaque structs) is NOT yet exercised. Those
  slices remain future work and will stress Charger's opaque-pointer handling.

## Environment notes (not Charger defects)

- FFmpeg `configure` cannot complete its native-clang link probe under this
  MSYS/LLVM-Windows setup (temp-path + `.exe`-extension friction). Worked around
  for the minimal slice by hand-generating the stub `config.h` set — no charger.rs
  change was needed for the environment.
- `log.c` / `bprint.c` / `time.c` compile under the stub config + `-idirafter`
  header layout (the earlier `libavutil/time.h` shadow fix). They pull in
  `<windows.h>` / `<unistd.h>` but the minimal TU set does NOT need POSIX
  `file.c` (`close`/`read`/`open`), so the version/callback/variadic slice
  builds clean. A whole-tree build would still trip `file.c` — out of scope.

## Next step (when extending)

Add a libavcodec/libavformat slice (opaque AVCodecContext/AVFormatContext) to
stress Charger's opaque-pointer handling. `run_regression.sh` already has the
generic `ffmpeg` row; the callback/variadic smoke can be folded in.
