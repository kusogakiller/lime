@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat" x64
set LLVM_SYS_221_PREFIX=C:\Users\szzxl\Downloads\clang+llvm-22.1.8-x86_64-pc-windows-msvc\clang+llvm-22.1.8-x86_64-pc-windows-msvc
set LIME_LLVM_PREFIX=C:\Users\szzxl\Downloads\clang+llvm-22.1.8-x86_64-pc-windows-msvc\clang+llvm-22.1.8-x86_64-pc-windows-msvc
set PATH=C:\Users\szzxl\Downloads\clang+llvm-22.1.8-x86_64-pc-windows-msvc\clang+llvm-22.1.8-x86_64-pc-windows-msvc\bin;%PATH%
cd C:\Users\szzxl\Downloads\lime

target\release\lime.exe charger install bench_clang\realworld\corpus\libfreshprobe
target\release\lime.exe charger install bench_clang\realworld\corpus\libcolprobe
target\release\lime.exe charger install bench_clang\realworld\corpus\libvarargedge
target\release\lime.exe charger install bench_clang\realworld\corpus\libubi
target\release\lime.exe charger install bench_clang\realworld\corpus\libfamprobe
target\release\lime.exe charger install bench_clang\realworld\corpus\libpackedbitfield
target\release\lime.exe charger install bench_clang\realworld\corpus\libenumedge
target\release\lime.exe charger install bench_clang\realworld\corpus\libenumedge_nanchor
target\release\lime.exe charger install bench_clang\realworld\corpus\zlib
target\release\lime.exe charger install bench_clang\realworld\corpus\sqlite
target\release\lime.exe charger install bench_clang\realworld\corpus\curl
target\release\lime.exe charger install bench_clang\realworld\corpus\libcallbackarg
target\release\lime.exe charger install bench_clang\realworld\corpus\libcallbackreturn
target\release\lime.exe charger install bench_clang\realworld\corpus\anon_flatten
target\release\lime.exe charger install bench_clang\realworld\corpus\libpackedbitfield