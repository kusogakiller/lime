#!/usr/bin/env bash
# Charger C-only vertical-slice verification script.
# Re-installs the C test library (fresh store entry gains `symbols` in the
# manifest), then builds + runs the C Lime slices and checks output.
set -e
cd "C:/Users/szzxl/Downloads/lime"

LLVM_BIN="C:\\Users\\szzxl\\Downloads\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\bin"
export PATH="$PATH:/c/Users/szzxl/Downloads/clang+llvm-22.1.8-x86_64-pc-windows-msvc/clang+llvm-22.1.8-x86_64-pc-windows-msvc/bin"
export LIME_LLVM_BIN="$LLVM_BIN"

echo "=== cargo build --release ==="
cargo build --release 2>&1 | tail -5

echo ""
echo "=== charger install libmathx (C) ==="
./target/release/lime.exe charger install bench_clang/charger/testlibs/libmathx

echo ""
echo "=== charger install sqlite (C, real-world) ==="
./target/release/lime.exe charger install bench_clang/realworld/sqlite

echo ""
echo "=== build + run C slices ==="
echo "--- c_math (expect 7 / 7 / 49) ---"
./target/release/lime.exe build bench_clang/charger/slices/c_math.lime
./bench_clang/charger/slices/c_math.exe
echo "--- c_callback (expect 7) ---"
./target/release/lime.exe build bench_clang/charger/slices/c_callback.lime
./bench_clang/charger/slices/c_callback.exe
echo "--- c_ptr (expect 2) ---"
./target/release/lime.exe build bench_clang/charger/slices/c_ptr.lime
./bench_clang/charger/slices/c_ptr.exe
echo "--- sqlite_slice (expect 1 / 0) ---"
./target/release/lime.exe build bench_clang/realworld/sqlite_slice.lime
./bench_clang/realworld/sqlite_slice.exe

echo ""
echo "=== done ==="
