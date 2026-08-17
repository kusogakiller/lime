#!/usr/bin/env bash
cd "C:/Users/szzxl/Downloads/lime"
LLVM="C:\\Users\\szzxl\\Downloads\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\bin"
export PATH="$PATH:$LLVM"
export LIME_LLVM_BIN="$LLVM"
python3 bench_clang/validate.py 2>&1 | tail -25
