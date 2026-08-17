#!/usr/bin/env bash
set -e
cd "C:/Users/szzxl/Downloads/lime"
LLVM="C:\\Users\\szzxl\\Downloads\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\bin"
export PATH="$PATH:$LLVM"
export LIME_LLVM_BIN="$LLVM"

echo "=== build+run c_iter8 slice (native exec) ==="
./target/release/lime.exe build --release --emit-object bench_clang/charger/slices/c_iter8.lime
EXE="bench_clang/charger/slices/c_iter8.exe"
if [ -f "$EXE" ]; then
  echo "--- run $EXE ---"
  ./"$EXE"
  echo "(exit=$?)"
else
  echo "ERROR: exe not produced at $EXE"
  ls -la bench_clang/charger/slices/c_iter8* 2>/dev/null
  exit 1
fi
