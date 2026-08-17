#!/usr/bin/env bash
cd "C:/Users/szzxl/Downloads/lime"
LLVM="C:\\Users\\szzxl\\Downloads\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\bin"
export PATH="$PATH:$LLVM"
export LIME_LLVM_BIN="$LLVM"
LIME=./target/release/lime.exe
for s in bench_clang/charger/slices/c_dep.lime bench_clang/charger/slices/variadic.lime; do
  echo "===== $s ====="
  "$LIME" build --release --emit-object "$s" 2>&1 | tail -4
  exe="${s%.lime}.exe"
  if [ -f "$exe" ]; then echo "--- run ---"; ./"$exe"; echo "(exit=$?)"; else echo "no exe"; fi
done
