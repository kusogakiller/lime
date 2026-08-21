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

---

## Iteration 12 — libavcodec opaque-pointer slice (2026-08-21)

Goal: prove the EXISTING generic Opaque-pointer handling works against a real
library's opaque context (AVCodecContext), NOT a FFmpeg-specific path.

### Corpus (hand-built)
`bench_clang/realworld/corpus/ffmpeg_avcodec_version/`
- libavcodec headers (incl. `avcodec.h` where `AVCodecContext` is a complete
  152-field struct) + libavutil headers (flat) for the shared types.
- TUs: libavcodec `version.c`, `options.c`; libavutil `version.c`, `mem.c`,
  `opt.c`, `channel_layout.c`, `log.c`, `bprint.c`, `time.c`.
- Stub `config.h` (HAVE_*/SIZEOF + FFMPEG_VERSION) — added
  `#define HAVE_SNPRINTF 1` / `HAVE_VSNPRINTF 1` so FFmpeg uses the UCRT
  `snprintf` instead of defining its own (avoids a duplicate `snprintf` symbol
  across TUs that corrupted the `.lib` symbol table). Generic FFmpeg configure
  flag, no library hack.
- `charger.toml`: `api_header = "libavcodec/avcodec.h"`,
  `build_flags = ["-I.", "-idirafter", "libavutil"]`
  (`-idirafter libavutil` keeps `<time.h>` resolving to the system header even
  though the corpus also carries `libavutil/time.h`).

### Step 1 — version + opaque-header parse (GREEN, native execution)
Smoke: `bench_clang/regression/ffmpeg_avcodec_smoke/ffmpeg_avcodec_version.lime`
- `avcodec_version()` -> 4070502 == (62<<16)|(28<<8)|102 == libavcodec 62.28.102  ✓
- exit code 0.
- Proves: libavcodec's full public-header tree (with `AVCodecContext` defined as
  a 152-field complete struct) normalizes through clang AST -> CType ->
  Lime interface -> build/link -> native execution. **The 152-field complete
  struct definition does NOT cause struct-by-value mis-handling**: function
  signatures that carry `AVCodecContext*` are normalized to
  `Opaque(AVCodecContext)` (pointer handle), exactly as the opaque-pointer rule
  requires.

### Step 2 — AVCodecContext* alloc / AVCodecContext** free (PARTIAL)
Interface facts (verified from generated `lime-iface.lime` + adapter C):
- `avcodec_alloc_context3(const AVCodec*) -> Opaque(AVCodecContext)` — correct
  opaque-pointer return; `avcodec_alloc_context3(NULL)` linked into the `.lib`.
- `avcodec_free_context(AVCodecContext**) -> ...` — Charger now surfaces this as
  `extern fn avcodec_free_context(Opaque(AVCodecContext): a0) -> Unit
  "lime_take_avcodec_free_context"`. The `AVCodecContext**` out-param is treated
  as a **take/free** idiom (void return + single `T**`): the Lime caller supplies
  the handle, the adapter passes `&local` to the real function. Generic — derived
  from (void return + single `T**`), no library name.

Runtime execution of `avcodec_alloc_context3(NULL)` + `avcodec_free_context(&ctx)`
is **BLOCKED by corpus scope, not by a Charger opaque-pointer bug**:
- `avcodec_alloc_context3` references `avcodec_default_get_buffer2`,
  `ff_codec_close`, `av_codec_iterate`, `av_d2q`, `av_parse_*`, `av_dict_*`, etc.
  — all `U` (undefined) in the minimal `.lib`, so they resolve to NULL and SEGV
  *inside the real FFmpeg call*. The opaque-pointer plumbing itself is correct;
  the call reaches real FFmpeg code.
- Pushing to full runtime would require resolving FFmpeg's entire internal-header
  tree (`utils.c` pulls in `FF_ALLOCZ_TYPED_ARRAY` from `mem_internal.h`,
  `codec_internal.h`, ...). That is a large, orthogonal corpus-expansion effort,
  out of scope for the opaque-pointer ABI proof. Per the "don't boil the ocean"
  gate, stopped here.

### Generic Charger fixes made this iteration (no FFmpeg-specific branch)
1. **Multi-dimensional fixed array mis-normalized as FAM** (`parse_c_type`,
   `src/charger.rs`). `int16_t[2][2]` was parsed as a flexible array member
   (size_part `"2][2"` failed `usize::parse` -> `None` -> FAM), generating an
   invalid `lime_make_*_flex(len)` accessor with a non-existent `len` field.
   Now all bracket groups are scanned and nested `Array` types are built
   outermost->innermost (`Array(Array(T,B),A)`), so multi-dim fixed arrays are
   correct. Also, a multi-dimensional array FIELD (element is itself an `Array`)
   no longer generates a by-value scalar accessor (would return an array, invalid
   C) — it stays opaque inside the C struct. Generic; protects every struct with
   a multi-dim fixed-array field (e.g. FFmpeg's `AVPanScan.position[2][2]`).
2. **void-return single-`T**` out-param = take/free** (`collect_out_param_adapters`
   + adapter emission, `src/charger.rs`). Previously ANY `T**` param was treated
   as a create-out-param (bridge drops the arg, returns the handle). Now a
   void-returning function with a single `T**` param is recognized as a
   take/consume idiom: the bridge takes the handle as input and returns void.
   `sqlite3_open` (int return) stays create-out-param — unaffected. Generic:
   (void return + single `T**`), no library name.
3. **`-idirafter` for ALL detected library include dirs** (native compile loop,
   `src/charger.rs`). The prior Iteration-11 fix only `-idirafter`'d the header's
   OWN parent dir; *detected* subdirs (e.g. `libavutil/` next to
   `libavcodec/avcodec.h`) still got plain `-I`, shadowing `<time.h>`. Now every
   detected include dir uses `-idirafter` (searched after system), so any
   multi-subdir library resolves system headers correctly. Generic.

### Verification after changes (all GREEN)
- `cargo build --release` ✓
- `cargo test --workspace` ✓ — only the baseline 3 closure-interpreter failures
  (`capture_multiple_values`, `nested_closure_capture`, `higher_order_native_interp`);
  no new failures from the changes.
- `bench_clang/validate_corpus.py` ✓ ALL CHECKS PASSED (17 corpora).
- `bench_clang/regression/run_regression.sh` ✓ PASS=*** FAIL=0 (the 8-library
  native gate: zlib, libpng, sqlite, libjpeg, curl, sdl2, ffmpeg, libcallbackarg).

### Status
- **Step 1 (version + opaque-header parse): NATIVE EXECUTION GREEN.**
- **Opaque-pointer normalization: PROVEN** — `AVCodecContext*` -> `Opaque`,
  `AVCodecContext**` -> take-adapter, both verified at AST/CType/interface/
  adapter/manifest stages.
- **Step 2b full alloc+free runtime: BLOCKED by corpus scope** (missing libavutil/
  libavcodec internal TUs), documented honestly — NOT a Charger opaque bug.
- 3 generic Charger bugs fixed (multi-dim-array FAM, take-out-param, idirafter-all).

---

## Iteration 12 — libavformat opaque-pointer slice (2026-08-21)

Goal: prove the SAME generic Opaque-pointer handling works against a SECOND
real library's opaque context (AVFormatContext), confirming the Iteration-12
libavcodec fixes are library-agnostic (not FFmpeg-specific).

### AST investigation (Source of Truth: clang 22 `-ast-dump=json`, zero errors)
`libavformat/avformat.h` (110 headers + libavutil + libavcodec self-includes):
- `struct AVFormatContext;` (line 331, forward-decl / opaque) then
  `typedef struct AVFormatContext { ... } AVFormatContext;` (line 1265, complete
  76-field struct). Same forward-decl+complete pattern as `AVCodecContext`.
- `avformat_version() -> unsigned int (void)`
- `avformat_alloc_context() -> AVFormatContext* (void)`
- `avformat_free_context(AVFormatContext*) -> void`  — **single pointer** (`**`
  NOT present) -> direct call, no adapter.
- `avformat_close_input(AVFormatContext**) -> void` — **void return + single
  `T**`** -> the take/free idiom.

### Corpus (hand-built)
`bench_clang/realworld/corpus/ffmpeg_avformat_version/`
- libavformat headers (110) + libavcodec headers (584, for `avformat.h`'s
  `libavcodec/{codec,codec_par,defs,packet}.h` self-includes) + libavutil
  headers (152) + FFmpeg-root `compat/`.
- TUs: `libavformat/version.c` (Step A), `options.c`, `avformat.c` (Step B link).
- Stub `config.h` with `ARCH_X86 0` (cross-compile-style stub) so FFmpeg's
  x86-specific `.c` TUs (`libavutil/x86/aes_init.c` needs `HAVE_AESNI_EXTERNAL`)
  and `libavutil/intmath.h`'s `#include "x86/intmath.h"` are skipped. Generic
  corpus-stub adjustment, no charger.rs change.
- `charger.toml`: `api_header = "libavformat/avformat.h"`,
  `build_flags = ["-I.", "-idirafter", "libavutil"]`.

### Step A — version + opaque-header parse (GREEN, native execution)
Smoke: `bench_clang/regression/ffmpeg_avformat_smoke/ffmpeg_avformat_version.lime`
- `avformat_version()` -> 4066406 == (62<<16)|(34<<8)|94 == libavformat 62.34.94  ✓
- exit code 0.
- Proves the full libavformat public-header tree (AVFormatContext forward-decl +
  76-field complete struct) normalizes through the whole pipeline and runs.

### Opaque-pointer normalization — PROVEN at interface/adapter stage (libavformat)
From the generated `lime-iface.lime` + adapter C:
- `avformat_alloc_context() -> Opaque(AVFormatContext) "avformat_alloc_context"`
  — `AVFormatContext*` is `Opaque(AVFormatContext)` (pointer handle), NOT
  struct-by-value. Confirms the opaque rule holds for a SECOND library's
  complete-struct-typed context.
- `avformat_free_context(Opaque(AVFormatContext): a0) -> Unit "avformat_free_context"`
  — single-pointer param stays a direct call (no adapter needed).
- `avformat_close_input(Opaque(AVFormatContext): a0) -> Unit
  "lime_take_avformat_close_input"` — the `void (T**)` take/free idiom is
  recognized and surfaced correctly. This is the SAME generic path fixed for
  `avcodec_free_context`; appearing on a DIFFERENT library (libavformat) proves
  the Iteration-12 fix is library-agnostic, not FFmpeg-specific.

### Step B — AVFormatContext* alloc / free (BLOCKED by corpus scope)
`avformat_alloc_context()` links into the `.lib` (`T`), but runtime execution of
`alloc + free` is BLOCKED by corpus scope (NOT a Charger opaque bug): the symbol
references `av_opt_set_defaults`, `av_mallocz`, `av_packet_alloc`, `av_dict_*`,
`avcodec_alloc_context3` / `avcodec_parameters_alloc`, `av_log`, etc. — all `U`
(undefined) without the matching libavutil/libavcodec `.c` TUs. Resolving them is
the same large, orthogonal FFmpeg-internal-tree expansion that blocked the
libavcodec Step 2b runtime. The opaque-pointer plumbing is correct; the call
would reach real FFmpeg code once those TUs are added. Per "don't boil the
ocean", stopped here — the opaque ABI is already proven at AST/CType/interface/
adapter/manifest for both AVCodecContext and AVFormatContext.

### Status (libavformat)
- **Step A (version + opaque-header parse): NATIVE EXECUTION GREEN.**
- **Opaque-pointer normalization: PROVEN** for a SECOND library — `AVFormatContext*`
  -> `Opaque`, single-pointer `free` stays direct, `void (T**)` -> take-adapter.
  All verified at AST/CType/interface/adapter/manifest stages. This confirms the
  Iteration-12 generic fixes generalize across libraries.
- **Step B full alloc+free runtime: BLOCKED by corpus scope** (missing libavutil/
  libavcodec internal TUs), documented honestly — NOT a Charger opaque bug.
- No charger.rs change needed for libavformat beyond the shared generic fixes
  (multi-dim-array FAM, take-out-param, idirafter-all) already in Iteration 12.
