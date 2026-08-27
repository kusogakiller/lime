@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64
cd C:\Users\szzxl\Downloads\lime
cargo run --release -- charger install bench_clang\realworld\corpus\libfreshprobe