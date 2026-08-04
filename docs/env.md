# ENV Standard Library

Environment variable access for the current process.

## API

| Function | Signature | Description |
|---|---|---|
| `get(key)` | `(str) -> Option(str)` | Get env var. Some(value) or None |
| `has(key)` | `(str) -> bool` | Check if env var exists |
| `set(key, value)` | `(str, str) -> bool` | Set env var. Always returns true |
| `remove(key)` | `(str) -> bool` | Remove env var. Always returns true |
| `all()` | `() -> Map` | All env vars as Map(str, str) |

## Examples

```lime
import env

fn main():
    env.set("MY_VAR", "hello")
    println(env.has("MY_VAR"))        // true
    println(env.get("MY_VAR"))        // Some(hello)
    println(env.has("NONEXISTENT"))   // false
    println(env.get("NONEXISTENT"))   // None
    env.remove("MY_VAR")
    println(env.has("MY_VAR"))        // false
    
    // PATH is always present
    println(env.has("PATH"))          // true
    return
```

## Limitations

- **`env_all()`**: The interpreter returns the full environment. The native codegen returns an empty map due to the i64-encoded pointer limitation in the C runtime Map type.
- **Thread safety**: `set`/`remove` are not thread-safe. Acceptable for single-threaded Lime programs.
