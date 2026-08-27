# Building & Testing Lime on Windows — Toolchain Requirements

Iteration 33 note. This file records the environment contract for building
Lime itself and for running its native-code tests.

## Compiler toolchain

* Rust 1.98+ (`rustup` default is fine)
* LLVM/Clang 22.x — the compiler expects `clang`, `llvm-nm`, and (for
  linking) `lld-link`. Point it at your LLVM install with either of:

```text
set LIME_LLVM_BIN=C:\path\to\LLVM\bin
:: or
set LLVM_SYS_221_PREFIX=C:\path\to\LLVM   (bin/ is derived)
```

## MSVC / Windows SDK requirement (IMPORTANT)

Producing an executable on Windows links against the MSVC CRT and Windows
SDK import libraries. **The native test suites (`cargo test`) therefore
require an MSVC link environment:**

> Run tests from an **x64 Developer Command Prompt for VS** (or execute
> `vcvarsall.bat x64` first).

If this environment is missing, `lld-link` cannot open `libcmt.lib` etc.
Since Iteration 33, `lime build` reports this as a hard error:

```text
error[E0501] build did not produce an executable (see linker diagnostics
above). Note: Windows native builds require an MSVC / Windows SDK
environment ...
```

and exits non-zero — it never prints `ok:` without having produced the
executable.

## Quick check

```text
where clang
echo %INCLUDE%   (must be non-empty in a vcvars prompt)
cargo test --release
```
