# OS Standard Library

Operating system information utilities.

## API

| Function | Signature | Description |
|---|---|---|
| `name()` | `() -> str` | OS name: "windows", "linux", "macos" |
| `arch()` | `() -> str` | CPU arch: "x86_64", "aarch64", "x86", "arm" |
| `platform()` | `() -> str` | Platform: "windows", "darwin", "linux" |
| `hostname()` | `() -> str` | Machine hostname |
| `cwd()` | `() -> str` | Current working directory |
| `set_cwd(path)` | `(str) -> bool` | Change cwd. Returns false on failure |

## Examples

```lime
import os

fn main():
    println(os.name())       // "windows" / "linux" / "macos"
    println(os.arch())       // "x86_64" / "aarch64"
    println(os.platform())   // "windows" / "linux" / "darwin"
    println(os.cwd())        // "C:\Users\..."
    os.set_cwd("/tmp")
    return
```

## Cross-Platform

Uses compile-time `cfg!` macros in the interpreter and `#ifdef` in the C runtime. Works on Windows, Linux, macOS, FreeBSD, and Unix-like systems.
