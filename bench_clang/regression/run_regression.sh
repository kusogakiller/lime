#!/usr/bin/env bash
# run_regression.sh — permanent 5-library Charger regression gate.
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

# library corpus source dir  ->  smoke slice (relative to bench_clang/realworld/corpus or regression)
LIBS="zlib:realworld/corpus/zlib:regression/zlib_smoke/zlib_smoke
libpng:realworld/corpus/libpng:regression/png_smoke/png_smoke
sqlite:realworld/corpus/sqlite:regression/sqlite_smoke/sqlite_smoke
libjpeg:realworld/corpus/libjpeg:regression/jpeg_smoke/jpeg_smoke
curl:realworld/corpus/curl:regression/curl_smoke/curl_smoke"

PASS=0
FAIL=0
for row in $LIBS; do
  lib="${row%%:*}"
  rest="${row#*:}"
  corpus="bench_clang/${rest%%:*}"
  smoke="bench_clang/${rest##*:}.lime"
  # 1) re-install (regression in AST/adapter surfaces as install failure)
  if ! ./target/release/lime.exe charger install "$corpus" >/dev/null 2>&1; then
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
