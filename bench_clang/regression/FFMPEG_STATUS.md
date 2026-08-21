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

## Status

- **Minimal slice (libavutil version API): NATIVE EXECUTION GREEN.**
- Full FFmpeg (libavcodec/libavformat, opaque structs, callbacks, variadic
  `av_log`) is NOT yet exercised. Those slices remain future work and will
  stress Charger's opaque-pointer / callback / variadic handling.
- `av_log` (variadic) and `av_log_set_callback` (callback) in libavutil are the
  natural next probes for those ABI features.

## Environment notes (not Charger defects)

- FFmpeg `configure` cannot complete its native-clang link probe under this
  MSYS/LLVM-Windows setup (temp-path + `.exe`-extension friction). Worked around
  for the minimal slice by hand-generating the stub `config.h` set — no charger.rs
  change was needed for the environment.
- The whole-tree build also trips POSIX-only `file.c` (`close`/`read`/`open` via
  `<unistd.h>`, absent from MSVC UCRT). The minimal version slice avoids it by
  compiling only `version.c`; full build would need FFmpeg's `compat/` MSVC
  shims or `HAVE_UNISTD_H 0`.

## Next step (when extending)

Add `av_log` (variadic) / `av_log_set_callback` (callback) probes, then a
libavcodec/libavformat slice (opaque AVCodecContext/AVFormatContext). Extend
`run_regression.sh` with an `ffmpeg` row (generic, no library-specific branch).
