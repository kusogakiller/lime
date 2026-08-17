#!/bin/bash
cd "C:/Users/szzxl/Downloads/lime"
export PATH="$PATH:/c/Users/szzxl/Downloads/clang+llvm-22.1.8-x86_64-pc-windows-msvc/clang+llvm-22.1.8-x86_64-pc-windows-msvc/bin"
export LIME_LLVM_BIN="C:\\Users\\szzxl\\Downloads\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\clang+llvm-22.1.8-x86_64-pc-windows-msvc\\bin"
./target/release/lime.exe charger install bench_clang/realworld/corpus/libjpeg/src/libjpeg-turbo-3.1.0 2>&1 | tail -30
echo "INSTALL_EXIT=${PIPESTATUS[0]}"
