# Lime

Lime is a small, statically-typed programming language with a tree-walking
interpreter and a native-code LLVM backend.

## Quick start

```sh
# Build the compiler
cargo build --release

# Run a single file
lime run hello.lime

# Type-check only
lime check hello.lime

# Build a project (requires citrus.toml)
citrus build
citrus run
```

## Example

```lime
fn main():
    println("hello, world")
    return
```

## Language features

- **Static typing** with type inference
- **Functions** with named parameters and return types
- **Structs** with methods
- **Enums** (state machines) with pattern matching
- **Tuples** with indexed access
- **Lists** and **slices**
- **Strings** with built-in methods
- **Closures** with capture support
- **Generics**
- **Interfaces**
- **Async/await**
- **Defer** statements
- **Range expressions** (e.g., `0..10`)
- **For-in loops** over ranges and lists

## Diagnostics

Lime produces structured error diagnostics with:

- **Error codes** (e.g., `error[E0201]`) for stable identification
- **File/line/col** locations for type errors
- **Source snippets** with caret pointers showing the error location
- **"Did you mean?"** suggestions for undefined names

Example:

```
error[E0201] main.lime:2:1
  |
2 | println(xyz)
  | ^
Type error: undefined variable 'xyz'
  = help: did you mean 'x'?
```

## Error code reference

| Code | Category | Description |
|------|----------|-------------|
| E0001 | Lexer | Invalid token or literal |
| E0101 | Parser | Syntax error |
| E0200-E0299 | Type checker | Type errors |
| E0201 | | Undefined variable |
| E0202 | | Unknown function |
| E0203 | | Unknown field |
| E0204 | | Unknown method |
| E0205 | | Tuple index out of bounds |
| E0206 | | Cannot index non-tuple |
| E0207 | | Wrong argument count |
| E0208 | | Argument type mismatch |
| E0209 | | Non-exhaustive match |
| E0401 | Codegen | Failed to write LLVM IR |
| E0402 | | Unlowered functions |
| E0501 | Linker | Build did not produce executable |
| E0601 | Runtime | Runtime error |
| E0701 | Memory | Memory analysis error |

## Known limitations

### Interpreter
- `let (a, b) = ...` destructuring parsed but not type-checked
- Closure-as-argument has stack overflow in project mode (works in single-file mode)

### Native backend
- `Option(T)` / `Result(T, E)` not lowered
- `defer` not supported in native
- `break` / `continue` not implemented in while loops
- Scalar `let x = 1` produces invalid IR (use tuples)
- String/list slicing not lowered
- For-in over lists partially supported
- Match on tuples partially supported
- Enum construction not lowered (bare variants work)

### Async runtime (Challenger)
- Single-thread only (multi-thread executor not supported)
- Sync primitive waiter arrays fixed at 256 slots
- Channel capacity maximum 65536
- Interpreter does not implement async Pending/Wake/Resume
- Reactor fd cleanup on task cancellation is incomplete

### Charger
- `charger install` has stack overflow on fixture libraries (pre-existing)
- Charger is frozen for 1.0 RC

### Platform
- Native builds require MSVC/Windows SDK environment
- `uuid.lib` must be available for linking

## Documentation

- [Language guide](docs/learn.md) - comprehensive reference
- [Grammar](docs/grammar.md) - formal grammar
- [Codegen](docs/codegen.md) - LLVM backend details
- [Runtime](docs/runtime.md) - runtime system
- [Closures](docs/closure.md) - closure implementation
- [Windows toolchain](docs/windows_toolchain.md) - MSVC setup

## Project structure

```
src/
  lib.rs          - compiler core (lexer, parser, type checker, interpreter, codegen)
  main.rs         - CLI entry point
  charger.rs      - C ABI adapter generation
  abiverify.rs    - ABI verification
tests/
  integration.rs  - end-to-end tests
  emission_regression.rs - LLVM emission tests
  diagnostic_tests.rs    - diagnostic format tests
  elif_tests.rs          - elif branch tests
  native_reliability_tests.rs - native feature tests
  link_failure_tests.rs  - linker error tests
  closure_capture_tests.rs - closure capture tests
docs/             - language and implementation documentation
examples/         - example programs
```

## Testing

```sh
# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration

# All tests
cargo test
```

## License

Licensed under either of

- Apache License, Version 2.0
- MIT License

at your option.
