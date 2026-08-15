#!/usr/bin/env bash
# Charger C/C++ vertical-slice verification script.
# Re-installs both test libraries (fresh store entries gain `symbols` in the
# manifest), then builds + runs the C and C++ Lime slices and checks output.
set -e
cd "C:/Users/szzxl/Downloads/lime"

LLVM_BIN="C:\\Users\\szzxl\\Downloads\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\bin"
export PATH="$PATH:/c/Users/szzxl/Downloads/clang+llvm-22.1.8-x86_64-pc-windows-msvc/clang+llvm-22.1.8-x86_64-pc-windows-msvc/bin"
export LIME_LLVM_BIN="$LLVM_BIN"

echo "=== cargo build --release ==="
cargo build --release 2>&1 | tail -5

echo ""
echo "=== charger install libmathx ==="
./target/release/lime.exe charger install bench_clang/charger/testlibs/libmathx

echo ""
echo "=== charger install libwidget ==="
./target/release/lime.exe charger install bench_clang/charger/testlibs/libwidget

echo ""
echo "=== build + run C slice (expect 7 / 7 / 49) ==="
./target/release/lime.exe build bench_clang/charger/slices/c_math.lime
./bench_clang/charger/slices/c_math.exe

echo ""
echo "=== build + run C++ slice (expect 12 / 24 / 3) ==="
./target/release/lime.exe build bench_clang/charger/slices/cpp_widget.lime
./bench_clang/charger/slices/cpp_widget.exe

echo ""
echo "=== done ==="
