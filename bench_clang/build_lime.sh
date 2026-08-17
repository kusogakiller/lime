#!/usr/bin/env bash
set -e
cd "C:/Users/szzxl/Downloads/lime"
LLVM="C:\\Users\\szzxl\\Downloads\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\bin"
export PATH="$PATH:$LLVM"
export LIME_LLVM_BIN="$LLVM"
echo "=== rebuild lime (release) ==="
cargo build --release 2>&1 | tail -8
echo "BUILD_EXIT=${PIPESTATUS[0]}"
