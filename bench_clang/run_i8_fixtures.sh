#!/usr/bin/env bash
set -e
cd "C:/Users/szzxl/Downloads/lime"
LLVM="C:\\Users\\szzxl\\Downloads\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\bin"
export PATH="$PATH:$LLVM"
export LIME_LLVM_BIN="$LLVM"

echo "=== install libiter8 ==="
./target/release/lime.exe charger install bench_clang/charger/testlibs/libiter8
echo "=== install libiter8b ==="
./target/release/lime.exe charger install bench_clang/charger/testlibs/libiter8b

echo "=== build+run c_iter8 slice (native exec) ==="
./target/release/lime.exe build --release --emit-object bench_clang/charger/slices/c_iter8.lime
EXE="bench_clang/charger/c_iter8.exe"
if [ -f "$EXE" ]; then
  echo "--- run $EXE ---"
  ./"$EXE"
  echo "(exit=$?)"
else
  echo "ERROR: exe not produced"
  exit 1
fi
