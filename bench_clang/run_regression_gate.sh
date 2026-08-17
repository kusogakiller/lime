#!/usr/bin/env bash
cd "C:/Users/szzxl/Downloads/lime"
LLVM="C:\\Users\\szzxl\\Downloads\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\bin"
export PATH="$PATH:$LLVM"
export LIME_LLVM_BIN="$LLVM"
LIME=./target/release/lime.exe
PASS=0; FAIL=0

run() { # desc  limefile  expected_exit
  local desc="$1" f="$2" exp="$3"
  if "$LIME" build --release --emit-object "$f" >/dev/null 2>&1; then
    local exe="${f%.lime}.exe"
    if [ -f "$exe" ]; then
      ./"$exe" >/dev/null 2>&1
      local rc=$?
      if [ "$rc" = "$exp" ]; then echo "PASS  $desc (exit=$rc)"; PASS=$((PASS+1)); else echo "FAIL  $desc (exit=$rc, expected $exp)"; FAIL=$((FAIL+1)); fi
    else echo "FAIL  $desc (no exe)"; FAIL=$((FAIL+1)); fi
  else echo "FAIL  $desc (build error)"; FAIL=$((FAIL+1)); fi
}

echo "########## SYNTHETIC CHARGER SLICES ##########"
for s in bench_clang/charger/slices/*.lime; do
  run "$(basename "$s")" "$s" 0
done

echo "########## ITERATION 8 REGRESSION FIXTURE ##########"
run "c_iter8 (i8 fixture)" bench_clang/charger/slices/c_iter8.lime 0

echo "########## REAL-WORLD TIER A (native) ##########"
run "zlib_slice"      bench_clang/realworld/corpus/zlib/zlib_slice.lime 0
run "zlib_crc"        bench_clang/realworld/corpus/zlib/zlib_crc.lime 0
run "libjpeg_slice"   bench_clang/realworld/corpus/libjpeg/libjpeg_slice.lime 0
run "libpng_slice"    bench_clang/realworld/corpus/libpng/libpng_slice.lime 0
run "sqlite_slice"    bench_clang/realworld/corpus/sqlite/sqlite_slice.lime 0
run "curl_slice"      bench_clang/realworld/corpus/curl/curl_slice.lime 0

echo "########## SUMMARY ##########"
echo "PASS=$PASS FAIL=$FAIL"
