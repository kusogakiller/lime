#!/usr/bin/env bash
cd "C:/Users/szzxl/Downloads/lime"
LLVM="C:\\Users\\szzxl\\Downloads\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\bin"
export PATH="$PATH:$LLVM"
export LIME_LLVM_BIN="$LLVM"
LIME=./target/release/lime.exe

echo "===== CACHE KEY PROOF ====="
echo "CHARGER_VERSION = $(grep -n 'const CHARGER_VERSION' src/charger.rs | head -1)"
echo "--- install a Tier-A lib twice (2nd should be cache HIT) ---"
"$LIME" charger install bench_clang/realworld/corpus/zlib 2>&1 | tail -3
echo "--- now touch a source and reinstall (should be cache MISS / rebuild) ---"
touch bench_clang/realworld/corpus/zlib/src/zlib-1.3.1/zutil.c
"$LIME" charger install bench_clang/realworld/corpus/zlib 2>&1 | tail -3

echo "===== SEMANTIC SUPPLEMENT VALIDATION ====="
echo "--- install libsemantic (has charger_semantic.toml) ---"
"$LIME" charger install bench_clang/charger/testlibs/libsemantic 2>&1 | tail -3
echo "--- run semantic slice (native exec) ---"
"$LIME" build --release --emit-object bench_clang/charger/slices/semantic.lime 2>&1 | tail -2
EXE=bench_clang/charger/slices/semantic.exe
if [ -f "$EXE" ]; then ./"$EXE"; echo "(exit=$?)"; else echo "no semantic exe"; fi
