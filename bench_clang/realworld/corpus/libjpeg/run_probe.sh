#!/bin/bash
cd "C:/Users/szzxl/Downloads/lime"
LIB=.lime-charger/store/libjpeg-turbo-3.1.0/0.1.0/c486a9f9cca22b00/libjpeg-turbo-3.1.0.lib
SRC=bench_clang/realworld/corpus/libjpeg/src/libjpeg-turbo-3.1.0
export PATH="$PATH:/c/Users/szzxl/Downloads/clang+llvm-22.1.8-x86_64-pc-windows-msvc/clang+llvm-22.1.8-x86_64-pc-windows-msvc/bin"
OUT="bench_clang/realworld/corpus/libjpeg/probe_native.exe"
clang -I"$SRC/src" bench_clang/realworld/corpus/libjpeg/probe_native.c "$LIB" -o "$OUT"
"$OUT"
