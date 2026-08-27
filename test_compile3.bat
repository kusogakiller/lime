@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64
set PATH=C:\Users\szzxl\Downloads\clang+llvm-22.1.8-x86_64-pc-windows-msvc\clang+llvm-22.1.8-x86_64-pc-windows-msvc\bin;%PATH%
cd C:\Users\szzxl\Downloads\lime\bench_clang\realworld\corpus\libfreshprobe
clang -fuse-ld=lld-link libfreshprobe.c -o libfreshprobe.exe -Wl,/subsystem:console -loldnames