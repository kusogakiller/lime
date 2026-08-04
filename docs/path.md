# Path Standard Library

Cross-platform file path manipulation utilities. All functions operate on string paths without touching the filesystem.

## API

| Function | Signature | Description |
|---|---|---|
| `join(a, b)` | `(str, str) -> str` | Join two path components with `/` separator. If b is absolute, returns b. |
| `basename(path)` | `str -> str` | Last component of a path (after last separator). |
| `dirname(path)` | `str -> str` | Directory portion of a path. Returns `.` for bare filenames. |
| `filename(path)` | `str -> str` | Filename without extension. |
| `extension(path)` | `str -> str` | File extension including dot. Empty string if none. |
| `is_absolute(path)` | `str -> bool` | True if path starts with `/` or drive letter (`C:`). |
| `normalize(path)` | `str -> str` | Resolve `.` and `..`, collapse separators. |
| `equals(a, b)` | `(str, str) -> bool` | Logical equality after normalization. |
| `parent(path)` | `str -> str` | Parent directory. Root's parent is root itself. |

## Examples

```lime
import path

fn main():
    println(path.join("foo", "bar"))          // "foo/bar"
    println(path.basename("/foo/bar.txt"))    // "bar.txt"
    println(path.dirname("/foo/bar.txt"))     // "/foo"
    println(path.filename("/foo/bar.tar.gz")) // "bar.tar"
    println(path.extension("/foo/bar.txt"))   // ".txt"
    println(path.is_absolute("/foo"))         // true
    println(path.is_absolute("foo"))          // false
    println(path.normalize("/foo/./bar/../baz.txt")) // "/foo/baz.txt"
    println(path.equals("/foo/./bar", "/foo/bar"))   // true
    println(path.parent("/foo/bar.txt"))      // "/foo"
    return
```

## Cross-Platform Behavior

- Paths always use `/` as separator in output (even on Windows)
- Both `/` and `\` are recognized as separators in input
- Absolute paths start with `/` or a drive letter (`C:\`)
- `normalize` resolves `.` (current) and `..` (parent) without filesystem access

## Runtime Implementation

**Interpreter:** Rust-side functions using string manipulation (no `std::path::Path`)
**Codegen:** C runtime functions (`runtime_path_*`) in `runtime.c`
**Package:** Thin wrapper in `packages/path/v0.1.0/src/path.lime`
