# Lime LLVM Codegen (`src/codegen`)

`src/codegen` is the LLVM **backend**. It lowers the already-typed, memory-
analysed program into textual LLVM IR (`emit_llvm` → `--emit-ll`). There is no
Inkwell/`llvm-sys` dependency and no system LLVM requirement: the emitter
produces `.ll` text that documents the target layout and can be diffed against
the interpreter for correctness (the interpreter remains the execution
oracle).

## Pipeline

```
run_compilation
  → type_check        (unchanged interpreter TC)
  → memory_analyze    (unchanged; Stack/Heap per `let`)
  → optimize_program  (const folding, operator resolution)
  → emit_llvm         (textual IR)
```

## Modules

| Module | Responsibility |
|--------|----------------|
| `mod.rs` | `emit_llvm` driver: runtime decls, string globals, aggregate (struct/state/list/iface) decls, vtable globals, monomorphization, function emission, `main` wrapper. |
| `types.rs` | `Type` → LLVM type name (`llvm_type_name`), alignment/size/zero helpers. |
| `fn_builder.rs` | Per-function IR: blocks, `let` (stack/heap), `if`/`while`/`for`, calls, structs, states, lists, strings, interfaces, prints. |
| `runtime.rs` | Rust mirror of the C runtime ABI (see `docs/runtime.md`). |
| `runtime/` | `runtime.c` / `runtime.h`. |

## Type → LLVM mapping

| Lime type | LLVM IR |
|-----------|---------|
| `Int` (`i`/`int`) | `i64` |
| `Float` (`f`/`float`) | `double` |
| `Bool` (`b`/`bool`) | `i1` |
| `String` (`s`/`str`) | `i8*` |
| `Struct S` | `%S` (named struct, field-ordered) |
| `State R` | `%R = { i32 tag; [4 x i64] payload }` (tagged union) |
| `List(T)` | `%LimeList = { i8* data; i64 len; i64 cap }` |
| `Interface I` | `%LimeIface = { i8* data; i8* vtable }` (fat pointer) |

## Construct coverage

| Feature | Status |
|---------|--------|
| literals (int/float/bool/string) | ✅ |
| `let` / `let mut` (stack + heap) | ✅ |
| assignment, `return` | ✅ |
| `if`/`else`, `while`, `for` (range & list) | ✅ |
| binary ops (+ - * / % == != < > <= >= and or), `not` | ✅ |
| call (user / monomorphized generic / struct ctor / state ctor) | ✅ |
| struct ctor / field access / method call | ✅ (self passed by pointer `%S*`) |
| state ctor + `match` (tagged-union `switch`) | ✅ |
| list literal / `len` / `get` / `add` / `set` | ✅ |
| string `len` / `slice` / `concat` / `chars` / `bytes` | ✅ |
| interface dispatch (vtable fat pointer) | ✅ |
| `print` / `println` (int/float/bool/string/list/struct) | ✅ |
| async (`lime`/`await`) | ✅ | `await` lowers to a direct synchronous call (matches interpreter `force_run`) |

## Return-type inference

Functions declared without an explicit return type (e.g. `fn add(i: a, i: b):`
with a `return a + b` body) are assigned an inferred return type so the
emitted LLVM signature stays valid. `infer_return_type` walks the body for the
first `return <expr>` using a parameter-env-aware `infer_expr_type`; call sites
use `call_ret_type` so `let x = f()` carries the correct value type instead of
being dropped as `void`.

## Interface ABI

- Each struct `S` implementing interface `I` yields a constant vtable:
  `@vtable_<S>_<I> = private constant [N x i8*] [ i8* bitcast(@<S>_<m>) to i8*, ... ]`
  with one slot per `I.methods` entry (in declared order).
- A concrete struct value is **boxed** into a `%LimeIface` fat pointer
  (`box_to_iface`): `data` = bitcast(`%S*` → `i8*`), `vtable` = bitcast of the
  vtable global to `i8*`. Boxing happens at call/let/assign sites where an
  interface type is expected but a struct value is supplied.
- Dispatch (`codegen_interface_method_call`): extract `data` + `vtable`,
  bitcast vtable to `i8**`, `gep` the method slot, load the `i8*` fn pointer,
  bitcast to `(i8* data, args...) -> ret`, and call with `data` as the first
  argument. Struct methods receive `self` by pointer (`%S*`) so the data
  pointer is directly ABI-compatible.

## Known limitations (Phase 1)

- No `runtime_free` insertion yet (leaks tolerated).
- Async (`lime`) functions are emitted as ordinary LLVM functions; `await` is a
  no-op lowering that compiles the inner call as a direct synchronous call. There
  is no coroutine/state-machine lowering and no async runtime.
- Interface values are only materialized through the fat-pointer boxing path;
  there is no unsized-interface-in-aggregate support yet.
- The emitted IR is not yet assembled/linked into a runnable binary (no system
  LLVM toolchain in the dev environment); correctness is established by
  comparing interpreter output against the intended semantics.
