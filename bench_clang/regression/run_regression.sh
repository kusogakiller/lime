#!/usr/bin/env bash
# run_regression.sh — permanent Charger regression gate (6 libraries).
#
# For each major C library it:
#   1) re-installs the library via `charger install` (catches AST-extraction,
#      normalization, and adapter-generation regressions — not just link),
#   2) builds the *smoke* slice (strict subset of the proven native gate),
#   3) executes it and requires non-empty output.
#
# Any failure aborts with non-zero exit. This must stay green after every
# Charger change. Generic: no library-specific branches inside the loop.
set -u

cd "$(dirname "$0")/../.." || exit 1

export PATH="$PATH:/c/Users/szzxl/Downloads/clang+llvm-22.1.8-x86_64-pc-windows-msvc/clang+llvm-22.1.8-x86_64-pc-windows-msvc/bin"
export LIME_LLVM_BIN="C:\\Users\\szzxl\\Downloads\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\bin"

# Format:  lib : corpus-path : smoke-slice(.lime)
# corpus-path is relative to repo root (bench_clang/...) OR an absolute Windows
# path (used when the corpus source lives outside the repo, e.g. SDL2 which is
# kept in Downloads to avoid bloating the git tree). No library-specific logic.
LIBS="zlib:bench_clang/realworld/corpus/zlib:bench_clang/regression/zlib_smoke/zlib_smoke
libpng:bench_clang/realworld/corpus/libpng:bench_clang/regression/png_smoke/png_smoke
sqlite:bench_clang/realworld/corpus/sqlite:bench_clang/regression/sqlite_smoke/sqlite_smoke
libjpeg:bench_clang/realworld/corpus/libjpeg:bench_clang/regression/jpeg_smoke/jpeg_smoke
curl:bench_clang/realworld/corpus/curl:bench_clang/regression/curl_smoke/curl_smoke
sdl2:C:/Users/szzxl/Downloads/lime_corpus_src/SDL2-2.30.9-win:bench_clang/regression/sdl2_smoke/sdl2_smoke
ffmpeg:bench_clang/realworld/corpus/ffmpeg_avutil_version:bench_clang/regression/ffmpeg_smoke/ffmpeg_smoke
ffmpeg_media_objects:bench_clang/realworld/corpus/ffmpeg_media_objects:bench_clang/regression/ffmpeg_media_objects_smoke/ffmpeg_media_objects_smoke
libcallbackarg:bench_clang/realworld/corpus/libcallbackarg:bench_clang/regression/libcallbackarg_smoke/libcallbackarg_smoke
libcallbackreturn:bench_clang/realworld/corpus/libcallbackreturn:bench_clang/regression/libcallbackreturn_smoke/libcallbackreturn_smoke
anon_flatten:bench_clang/realworld/corpus/anon_flatten:bench_clang/regression/anon_flatten_smoke/anon_flatten_smoke
libenumedge:bench_clang/realworld/corpus/libenumedge:bench_clang/regression/libenumedge_smoke
libenumedge_nanchor:bench_clang/realworld/corpus/libenumedge:bench_clang/regression/libenumedge_n-anchor
libpackedbitfield:bench_clang/realworld/corpus/libpackedbitfield:bench_clang/regression/libpackedbitfield_smoke"

PASS=0
FAIL=0
for row in $LIBS; do
  # Parse with field separators that may also appear INSIDE a value:
  # the corpus path for an absolute Windows location is `C:/...`, whose
  # drive-letter `:` collides with the `:` delimiter. So take `lib` from the
  # LEFT (first `:`), `smoke` from the RIGHT (last `:`), and `corpus` as the
  # middle slice. This keeps the drive-letter `:` inside an absolute path
  # intact (e.g. `C:/Users/.../SDL2-2.30.9`).
  lib="${row%%:*}"
  smoke="${row##*:}.lime"
  corpus="${row#*:}"
  corpus="${corpus%:*}"
  # Absolute corpus path (starts with a drive letter or /) is used verbatim;
  # otherwise it is relative to the repo root.
  case "$corpus" in
    [A-Za-z]:*|/*) corpus_path="$corpus" ;;
    *) corpus_path="$corpus" ;;
  esac
  # 1) re-install (regression in AST/adapter surfaces as install failure)
  if ! ./target/release/lime.exe charger install "$corpus_path" >/dev/null 2>&1; then
    echo "FAIL  $lib (charger install)"; FAIL=$((FAIL+1)); continue
  fi
  # 2) build smoke slice
  if ! ./target/release/lime.exe build --emit-object "$smoke" >/dev/null 2>&1; then
    echo "FAIL  $smoke (build/link)"; FAIL=$((FAIL+1)); continue
  fi
  exe="${smoke%.lime}.exe"
  if [ ! -f "$exe" ]; then echo "FAIL  $smoke (no exe)"; FAIL=$((FAIL+1)); continue; fi
  out=$(./"$exe" 2>&1 | head -3 | tr '\n' '|')
  if [ -n "$out" ]; then echo "PASS  $lib -> $out"; PASS=$((PASS+1)); else echo "FAIL  $smoke (no output)"; FAIL=$((FAIL+1)); fi
done
echo "---"
echo "PASS=$PASS FAIL=$FAIL"
[ "$FAIL" -eq 0 ]
