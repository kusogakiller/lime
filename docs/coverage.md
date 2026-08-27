# Lime Coverage Overview (Step 1)

## Purpose

To ensure Lime reaches the same "software scope" as Rust / Go, we inventory the
required capabilities, standard library, runtime, and toolchain.

This document **does not define syntax**. It is a capability inventory —
"what must be possible" — and syntax is determined in later steps.

Existing Lime principles and specs are preserved:
- Easy. Simple. Fast.
- Concise syntax / readability first
- No Rust-ification ('a / lifetime annotations / borrow checker required from users)
- No C++-ification (inheritance / template / operator overuse)
- No self / this, no impl block form
- No implicit type conversions
- No GC, no compiler automatic memory management

---

## 0. What the current Lime covers (prototype-based)

- Variables (`let` / `let mut`)
- Functions (argument types by name, return type postfixed)
- Struct (fields + methods, no self/this)
- State + Match (exhaustive matching required, `else` forbidden, `Ignore` for discards)
- Basic types: `int` / `float` / `bool` / `str`
- StringBuilder
- Type Checker (pre-execution type checking)
- Interpreter execution

→ Small scripts, CLI prototypes, and data transformations are feasible.
→ Web / concurrency / filesystem / package splitting are not.

---

## 1. Module System / Package Management

- **Purpose**: Composing medium-to-large software beyond a single file.
- **Necessity**: Equivalent to Rust (crate) / Go (package). Without it, practical applications cannot be written.
- **Priority**: High
- **Impact on existing specs**: `lime.toml` `[imports]` management (already decided). Source splitting is handled by the compiler merging multiple `.lime` files into one unit. No import syntax in AST. TypeChecker merges `Defs` across files.
- **Implementation difficulty**: Medium (compiler-side file merging + Defs merging)

---

## 2. Standard Library (Core)

- **Purpose**: Essential built-in functionality for practical applications.
- **Necessity**: Without fs / os / path / json / requests / time / datetime / math / random / string / collections / async / thread / logger, nothing can be built.
- **Priority**: High
- **Impact on existing specs**: `requests` is built-in (already decided). Core + citrus separation (already decided). String API (`.bytes()`/`.chars()`/`.slice()`) finalization is a prerequisite.
- **Implementation difficulty**: Medium to High (can be temporarily implemented as built-in functions in the interpreter prototype)

---

## 3. Explicit Type Conversion API

- **Purpose**: Intentional conversion between types.
- **Necessity**: Since implicit conversion is forbidden, int↔float↔str conversions are essential. Equivalent to Rust (`str::from_utf8`) / Go (`strconv`).
- **Priority**: High (required for writing the standard library)
- **Decided items**:
  - Function form: `int(x)` / `float(x)` / `str(x)` (between all basic types except bool)
  - **Numeric → bool conversion is forbidden** (both implicit and explicit):
    - Forbidden: `bool(0)` / `bool(1)` / `bool(123)`
  - Bool conversion is only allowed via dedicated values / explicit boolean expressions (e.g., comparison operator results, or dedicated syntax to obtain booleans).
  - Purpose: prevent unintentional condition checks, maintain readability, eliminate ambiguity.
- **Impact on existing specs**: Implicit conversion prohibition is maintained. Built-in conversion functions are `int(x)` / `float(x)` / `str(x)` only (bool excluded). Return types defined in TypeChecker.
- **Implementation difficulty**: Low

---

## 4. String Manipulation API

- **Purpose**: Safe manipulation of UTF-8 text.
- **Necessity**: Without byte/character access, slicing, and length, text processing is impossible.
- **Priority**: High
- **Decided items**:
  - `.bytes() -> Array(byte)`: byte sequence
  - `.chars() -> Array(char)`: character sequence (Unicode codepoint units)
  - `.slice(a, b) -> str`: substring
  - `.len() -> int`: **character count (Unicode codepoint units)**
  - `.byte_len() -> int`: byte length (for low-level UTF-8 operations)
  - `text[0]` remains forbidden (avoid ambiguity)
  - Encoding specification `str(utf16)` is maintained
  - StringBuilder exists as-is
  - **No string-specific operators**. String manipulation is provided via String API (methods), not operators. Reason: avoid operator proliferation, maintain readability, avoid adding special syntax.
- **Impact on existing specs**: TypeChecker defines return types `Array(byte)` / `Array(char)` / `str`. Operator Interface (`Add` etc.) applies only to numeric types; string concatenation is via API.
- **Implementation difficulty**: Low to Medium

---

## 5. Collections

- **Purpose**: Generic data structures.
- **Necessity**: Without lists, maps, sets, and tuples, practical processing is impossible. Equivalent to Rust (vector/map) / Go (slice/map).
- **Priority**: High
- **Decided items**: Arrays and lists are **unified as `List(T)`** (the distinction between fixed-length/variable-length is unnecessary or determined by the Runtime). `Map(K,V)` / `Set(T)` / `Tuple(...)` coexist.
- **Impact on existing specs**: Generic `List(T)` adopted (already decided). TypeChecker adds `List(Type)` / `Map(K,V)` / `Set(T)` / `Tuple(...)`. Bracket literals are treated as `List` literals.
- **Implementation difficulty**: Medium

---

## 6. Generic (Type Parameters)

- **Purpose**: Reusable abstractions (Result(T) / List(T) / Option(T) etc.).
- **Necessity**: Without abstractions, even the standard library cannot be written. Equivalent to Rust / Go generics.
- **Priority**: High
- **Impact on existing specs**: `state Result(T):` / `struct Box(T):` / `fn max(List(T where T: Comparable)):` (already decided). Parser / AST / TypeChecker extensions. Constraints use the Interface approach.
- **Implementation difficulty**: High

---

## 7. Option Type

- **Purpose**: Null safety.
- **Necessity**: Equivalent to Rust `Option`. Lime uses the Option type (already decided).
- **Priority**: Medium to High
- **Impact on existing specs**: `let User?: user` = internally `Option(User)`. Separate from State. TypeChecker adds `Option(Type)`. Handled via Match.
- **Implementation difficulty**: Medium

---

## 8. Interface (Implicit Implementation)

- **Purpose**: Polymorphism as an alternative to forbidden inheritance.
- **Necessity**: Abstractions like passing `Animal` as an argument are needed. Equivalent to Rust trait / Go interface (but Lime uses implicit implementation).
- **Priority**: Medium to High
- **Impact on existing specs**: `interface Animal: fn speak()`, implementing types implicitly conform by simply having matching methods (already decided). Operators also use the Interface approach (`Add` etc., targeting numeric types). Strings use String API instead of operators (see item 4). TypeChecker performs structural conformance checking.
- **Implementation difficulty**: High

---

## 9. Loop Syntax

- **Purpose**: Iteration.
- **Necessity**: Currently only if / match exist, with no repetition. Equivalent to Rust / Go for/loop.
- **Priority**: High (basic control structure)
- **Impact on existing specs**: `for x in list:` / `for i in 0..n:` / `while cond:`. Range `..` operator exists. Parser / AST / Interpreter / TypeChecker extensions.
- **Implementation difficulty**: Medium

---

## 10. Concurrency (thread / async runtime)

- **Purpose**: Simultaneous execution for web servers, background processing, etc.
- **Necessity**: Equivalent to Go goroutine / Rust async. Frequently needed in modern development.
- **Priority**: Medium
- **Decided items (Async syntax)**:
  - Regular functions: `fn function():`
  - Asynchronous functions: `lime function():`
  - await: `let data = await request("url")`
  - Example: `fn main():` / `lime main():`
  - Regular functions (`fn`) cannot participate in async processing. `await` usage is only allowed inside `lime` functions.
  - Callable rules and Runtime details for await will be specified in the Async / Runtime design.
  - Reason: Lime uses its own syntax to clearly distinguish sync/async. Avoids adding `async` as a reserved keyword or copying Rust/JS patterns.
- **Impact on existing specs**: `lime` keyword declares async functions. `await` is already decided. Runtime not yet designed (separate decision). Strong coupling with Memory analysis.
- **Implementation difficulty**: High (includes Runtime design)

---

## 11. unsafe / Pointer / C ABI (FFI)

- **Purpose**: OS API integration, C library interop, SIMD, and other low-level control.
- **Necessity**: Rust / Go also have FFI. Essential for reaching the systems domain.
- **Priority**: Medium (for systems domain)
- **Impact on existing specs**: `unsafe:` blocks, `User*` / `&user`, C ABI (cdecl / repr(C) compatible). TypeChecker adds Pointer type. Strong coupling with Memory analysis.
- **Implementation difficulty**: High

---

## 12. Error Propagation Syntax

- **Purpose**: Reduce verbosity in functions returning Result.
- **Necessity**: Can be replaced with Match + State, but that is verbose. Equivalent to Go `if err != nil` / Rust `?`.
- **Priority**: Medium
- **Impact on existing specs**: Not yet decided (candidates: `?` / `raise`). TypeChecker determines propagation type.
- **Implementation difficulty**: Medium

---

## 13. Test Framework

- **Purpose**: Quality assurance.
- **Necessity**: Equivalent to Rust `cargo test` / Go `go test`.
- **Priority**: Medium
- **Impact on existing specs**: `lime test` command + test annotations. Compiler / CLI feature.
- **Implementation difficulty**: Medium

---

## 14. Formatter / Documentation Generation

- **Purpose**: Improve maintainability.
- **Necessity**: Equivalent to Go `gofmt` / `godoc`.
- **Priority**: Low
- **Impact on existing specs**: `lime fmt` / `lime doc`. Tooling.
- **Implementation difficulty**: Low

---

## 15. Runtime

- **Purpose**: Execution infrastructure (async scheduler, task management, standard library foundation).
- **Necessity**: The interpreter prototype has no runtime. Essential for AOT compilation, concurrency, and standard library operation.
- **Priority**: High (prerequisite for concurrency and standard library)
- **Impact on existing specs**: Async Runtime / Task API not yet designed (separate decision). Coupled with Memory analysis.
- **Implementation difficulty**: High

---

## 16. Toolchain (compiler / package manager)

- **Purpose**: Build, execution, and dependency management.
- **Necessity**: `lime build/run/debug`, `citrus init/add/rem/update/install/pub/search` (already decided).
- **Priority**: High (prerequisite for distribution and reuse)
- **Impact on existing specs**: Compiler name `lime`, package manager name `citrus` (responsibility separation, already decided).
- **Implementation difficulty**: Medium

---

## Priority Summary

| Priority | Items |
|----------|-------|
| High | 1 Module / 2 Standard Library / 3 Explicit Conversion / 4 String API / 5 Collections / 6 Generic / 9 Loop / 15 Runtime / 16 Toolchain |
| Medium-High | 7 Option / 8 Interface |
| Medium | 10 Concurrency / 11 unsafe / FFI / 12 Error Propagation / 13 Testing |
| Low | 14 fmt / doc |

---

## Notes for Next Steps

- This document is a "capability inventory" and does not finalize syntax.
- Previously undecided items (explicit conversion / String API / Operator Interface names / Async syntax) are now decided. The grammar spec `grammar.md` has been created.
- Recommended implementation order:
  1. Explicit conversion API
  2. String API
  3. Collections (List unification)
  4. Loops
  5. Option
  6. Generic
  7. Interface
  8. Async
  9. Memory analysis
  10. LLVM
- All items maintain existing prohibitions (Rust-ification / C++-ification / self-this / impl / implicit conversion / string operators / match else).
