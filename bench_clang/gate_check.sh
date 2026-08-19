#!/usr/bin/env bash
# gate_check.sh — verify the 5-major-library native execution gate.
# For each real-world corpus library slice, build + execute and report PASS/FAIL.
set -u
cd "$(dirname "$0")/.." || exit 1

export PATH="$PATH:/c/Users/szzxl/Downloads/clang+llvm-22.1.8-x86_64-pc-windows-msvc/clang+llvm-22.1.8-x86_64-pc-windows-msvc/bin"
export LIME_LLVM_BIN="C:\\Users\\szzxl\\Downloads\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\bin"

SLICES="zlib/zlib_slice zlib/zlib_crc libpng/libpng_slice sqlite/sqlite_slice libjpeg/libjpeg_slice curl/curl_slice"
PASS=0
FAIL=0
for s in $SLICES; do
  lime="bench_clang/realworld/corpus/$s.lime"
  exe="bench_clang/realworld/corpus/$s.exe"
  if [ ! -f "$lime" ]; then echo "SKIP  $s (no slice)"; continue; fi
  if ! ./target/release/lime.exe build --emit-object "$lime" >/dev/null 2>&1; then
    echo "FAIL  $s (build/link)"; FAIL=$((FAIL+1)); continue
  fi
  if [ ! -f "$exe" ]; then echo "FAIL  $s (no exe)"; FAIL=$((FAIL+1)); continue; fi
  out=$(./"$exe" 2>&1 | head -1)
  if [ -n "$out" ]; then echo "PASS  $s -> $out"; PASS=$((PASS+1)); else echo "FAIL  $s (no output)"; FAIL=$((FAIL+1)); fi
done
echo "---"
echo "PASS=$PASS FAIL=$FAIL"
