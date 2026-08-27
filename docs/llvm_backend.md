# Lime LLVM Backend Design (Step 10)

This document describes the Step 10 LLVM backend design. It defines the
**design**, not the implementation. Goal: transition from interpreter dependency
to a compiler backend, establishing the flow
`AST → Typed AST → Memory Analysis → LLVM IR`.

Constraints (maintained):
- No GC
- No borrow checker / lifetime syntax exposed to users
- Minimal Runtime dependency (Lime runtime = only a small C-equivalent helper)
- Existing Lexer/Parser/AST/TypeChecker/Generic/Interface/Async/Memory Analysis is reused

---

## 0. Current State and Migration Strategy

Current `src/main.rs` is a single-pass interpreter:
```
tokenize → parse → collect_defs → (operators) → type_check → memory_analyze → execute
```
Execution holds values in `Value` enum (i64/f64/String/Bool/Array/Struct/State/Option/Future...)
via Rust's heap (`Box` / `Vec` / `HashMap`).

After LLVM migration:
```
tokenize → parse → collect_defs
         → type_check        (existing: attach/verify Typed info on AST)
         → memory_analyze    (existing: determine Stack/Heap for each `let`)
         → lower_to_llvm     (new: generate LLVM IR)
         → (MC/obj) → execution / or ORC JIT
```

Principles:
- No new fields added to AST nodes. Typed/Memory information is referenced from
  `Defs` aggregated by separate passes (similar to existing `resolved_operator`).
- The Interpreter is not deleted; it **runs in parallel per phase**, and diff
  testing comparing LLVM output with Interpreter output guarantees correctness
  (Nightly testing).
- Initially only `main` is compiled; functionality is gradually expanded.

---

## 1. LLVM IR Generation Approach

### 1.1 Approach
- Add **Inkwell** (`llvm-sys` wrapper) to `Cargo.toml`. The target is the host
  native (since `llvm-sys` depends on the system's LLVM shared library, the
  development environment requires LLVM).
- To progress with test-driven development, the **first few phases use ORC JIT
  for direct execution**, later expanding to object/executable generation via
  `TargetMachine`.

### 1.2 Module Structure
```
src/
  main.rs              (existing: Lexer/Parser/AST/TC/Memory/Interp)
  codegen/
    mod.rs             (CodegenContext, holds Module/Builder/TargetData)
    types.rs           (Lime Type → LLVM Type mapping)
    value.rs           (Sized/Unsized value representation, temporary value stack)
    fn_builder.rs      (per-function IR generation, basic block management)
    structs.rs         (struct / state / variant type layout)
    calls.rs           (call / call_method / builtin → runtime FFI)
    generic.rs         (type argument monomorphization)
    interface.rs       (vtable / fat-pointer dispatch)
    async_rt.rs        (Future struct + state machine)
    runtime/
      runtime.c/.h     (allocator, print, List/String helpers, no exceptions)
      runtime.rs       (extern "C" declarations, built-in symbols)
```

### 1.3 Function Model
- Each Lime function → LLVM `Function`. Arguments are passed by value or pointer
  (see §4/§3).
- Return values follow ABI:
  - Scalar (i64/f64/i1) → register return.
  - Aggregate (struct/State/Option/List/String/Future) → caller-allocated sret
    pointer passed as first implicit argument (`sret` convention).
- Control flow is represented by `basic blocks`. `if/while/for/match` become
  block branches.

---

## 2. Conversion Responsibilities from Typed AST to LLVM

Currently `type_check` only detects errors and does not write types back to the
AST. Since LLVM requires "typed information", a lightweight **Typed AST (T-IR)**
intermediate layer is introduced.

### 2.1 Typed AST Form (minimal addition)
A new enum is prepared exclusively for `codegen` (without breaking existing `Expr`):
```
TypedExpr = TInt(i64) | TFloat(f64) | TString(&str) | TBool(bool)
          | TVar(String, Type)                      // variable reference + its type
          | TBinOp(Box, op, Box, ResolvedOperator)  // reuses existing resolved_operator
          | TCall { func, args: Vec<TypedExpr>, ret: Type }
          | TMethodCall { .. , ret: Type }
          | TFieldAccess { .. , field_ty: Type }
          | TAwait(Box, ret: Type)
          | ...
```
- At the end of `type_check`, run `lower_to_typed(stmts, defs) -> Vec<TypedStmt>`
  (reusing existing `infer_type` / `check_expr` results).
- Memory Analysis results (`let name -> Stack/Heap`) are kept in `Defs` or a
  separate map `memory: HashMap<(fn, var), MemoryPlace>` referenced by codegen.

### 2.2 Conversion Responsibility Split
| Responsibility | Layer |
|----------------|-------|
| Expression type determination | TypeChecker (existing) |
| Placement determination (Stack/Heap) | Memory Analysis (existing) |
| Type → LLVM type | `types.rs` |
| Expression → IR | `fn_builder.rs` (`visit_expr`) |
| Statement → IR | `fn_builder.rs` (`visit_stmt`) |
| Declaration → IR | `structs.rs` / `calls.rs` |
| Monomorphization | `generic.rs` |
| Polymorphic dispatch | `interface.rs` |
| Async | `async_rt.rs` |

---

## 3. Stack/Heap Memory Information in LLVM

Memory Analysis output (whether each `let` is Stack or Heap) maps directly to
allocation strategy.

- **Stack**: `alloca` in the function frame. Lifetime is the basic block scope.
  Since it doesn't escape, the pointer is only valid within the function. `alloca`
  is automatically promoted to registers by LLVM (mem2reg), so it's effectively
  OK whether it's stack or registers.
- **Heap**: Call `runtime_alloc(size, align)` (§9 FFI), then `bitcast` the
  returned `i8*` to the appropriate struct pointer type.
- **Explicit `heap`**: always `runtime_alloc`.
- **Explicit `stack` but escapes**: compile error already raised by Memory
  Analysis (Step 9). Therefore, by the time codegen is reached, "stack that
  escapes" cases do not exist.
- **Values used after `await` in async**: Memory Analysis already determined Heap.
  Placed in Future frame (§8) heap area.

There is no "ownership" concept on LLVM. It simply selects "alloca or malloc
depending on value lifetime." This is never exposed to users (= per design).

> Note: No GC/RC. The compiler internally uses a single-owner model.
> Value types are copied; heap types like String/List are internally optimized
> for movement/sharing by the compiler. Copy/move concepts are not exposed to users.

---

## 4. Struct Representation

Current `Value::Struct { name, fields: Vec<(String,Value)> }` is a tagged dynamic
tuple. In LLVM, each struct is laid out as a **named LLVM StructType**.

### 4.1 Layout
- `struct User { str: name }` →
  `%User = type { i8* }` (`str` is managed as `i8*` per §5 String).
- Field order follows `StructDef.fields`. Padding is determined by LLVM's `TargetData`.
- Generic struct `Vec2(T)` → **monomorphized** per type argument (§6),
  generating `%Vec2_i64` / `%Vec2_f64`.

### 4.2 Constructor
`User("Alice")` →
1. `alloca %User` (stack/heap per placement)
2. Field initialization via GEP + store
3. Value as `%User*` or sret copy

### 4.3 Field Access
`u.name` → `getelementptr` to field pointer → `load`. Since it's via pointer,
value-type copy semantics occur at the `load` timing.

### 4.4 State / Variant
`Result(T,E)` is currently `State { name, values }`. In LLVM:
- Each `State` is a **tagged union**: `{ i32 tag; [N x i8] payload }` or
  `i32` + the largest variant's struct.
- `Success(v)` / `Error(e)` → tag write + payload write.
- `match` → tag comparison for basic block branching. Exhaustiveness is guaranteed
  by TypeChecker.

---

## 5. List / String Runtime Design

Core of no-GC, minimal Runtime.

### 5.1 String
- Representation: `i8*` + length **fat pointer** or `%LimeStr = type { i8*, i64 len }`.
- Immutable semantics: concatenation `+` allocates a new buffer (same as existing
  Interpreter).
- Generation: `runtime_str_from_utf8(ptr, len)` / concatenation `runtime_str_concat(a,b)`.
- UTF-8 guarantee is compile-time (literals) + runtime entry verification (on error,
  via `Result`/`State`, no exceptions).

### 5.2 List
- Current: `Value::Array(Vec<Value>)`. In LLVM:
  - Header: `%LimeList = type { i8* data; i64 len; i64 cap }` (data is on Heap).
  - Element array `T` is allocated via `runtime_alloc` (element size × cap).
  - `List(T)` is monomorphized to fix `T`.
- Buffer is always Heap (Memory Analysis may treat List values as heap; or
  separate header=stack, buffer=heap. As stated in §3, "internal buffer is Heap").
- Index / `for` iteration: GEP + bounds check (failure returns `Error` via `State`
  or traps; spec uses Result-returning API).

### 5.3 Lifetime (Runtime side)
- Lime has single-owner, copy semantics. Reference counting (Rc/Arc) is **not used**.
- How to free living Heap values on function return / scope end?
  - Phase 1 (first half of Step 10): **do not free (tolerate leaks)** to complete
    the compiler.
  - Phase 2: linear scope analysis (DOM-based last-use position) automatically
    inserts `runtime_free`. Reuses Escape Analysis results; emits free after "last use."
  - GC / RC is forbidden. Only static insertion by the compiler.

---

## 6. Generic Handling

Current: `Type::Var(T)` has constraints resolved during `type_check`, but the
actual type is not unified (the interpreter absorbs polymorphism at runtime via
`Value`). LLVM uses **Monomorphization (type argument unification)**.

- If `fn max<T>(...)` is called with `T=i64` and `T=f64`, generate
  `@max_i64` / `@max_f64` respectively.
- Steps:
  1. `collect_defs` keeps generic functions as "templates."
  2. From the call site (`TCall`), obtain the concrete type of actual arguments.
  3. If not yet generated, run `instantiate(fn, [concrete_type])`:
     - Replace `Type::Var(T)` with the concrete type (rewrite AST type annotations/constraints).
     - Constraint `T: Compare` is statically checked for whether the concrete type
       satisfies the Interface (reuses existing logic).
     - Codegen the unified `FunctionDef`.
  4. Cache for the same concrete type (don't regenerate).
- Generics with Interface constraints go to §7 vtable.

> No generic-specific Memory rules are added (per Step 9 decision). After
> monomorphization, normal Memory Analysis is applied to the concrete type.

---

## 7. Interface Dispatch Method

Current: `Type::Interface(name, [Type])`, implicit implementation, static
resolution via `resolved_operator`. In LLVM, two options:

### 7.1 Static Monomorphization (preferred, Phase 1)
- When the receiver's concrete type is determined at the call site (most Lime
  code is statically determinable), **directly call the concrete method**
  (devirtualize), same as `resolved_operator`.
- Like Memory Analysis, "called interface methods are resolved to concrete types."

### 7.2 vtable / fat-pointer (when needed, Phase 2)
- Only when the concrete type cannot be statically determined (stored in
  collections, unknown via argument):
  **fat pointer**: `InterfaceValue = { i8* data; vtable* vp }`.
  - `vtable = { fn ptr, fn ptr, ... }` (one entry per method).
  - Method call: `vp->slot[k](data, args...)`.
- Users are not shown trait/object syntax. This is internal compiler dispatch only.
- Operator Interface (`Add`/`Equal`/`Compare`) maps existing `resolved_operator`
  directly to LLVM `call` (already statically resolved).

---

## 8. Async / Future Representation

Current: `lime` functions → `Value::Future{func,args}`, `await` force-runs
(Interpreter). In LLVM, either a true async runtime (state machine) or
**simple synchronous expansion**.

### 8.1 Phase 1: Synchronous Expansion (single-threaded cooperative)
- Codegen `lime` functions as "functions that return a Future":
  - `Future` struct = `{ i32 state; i8* frame; fn* resume }`.
  - `frame` is Heap (Memory Analysis already determined async-escaping values as Heap).
- `await e`:
  1. Evaluate `e` → `Future f`.
  2. Save current state to `frame`, execute `f`'s `resume()`.
  3. Progress via **simple event loop / synchronous polling** (single thread)
     until `f` completes.
- No exception mechanism. Failures propagate as `Result(T,E)` / `State` values
  (maintaining existing spec).

### 8.2 Phase 2: True State Machine (LLVM coroutine / manual splitting)
- Split `lime` function body at `await` boundaries into basic blocks,
  generating a C++20-coroutine-equivalent state machine (or using `llvm.coro.*`
  intrinsics).
- `Future` is a heap-allocated resume state-holding area.
- Thread pool / runtime scheduler is added to the §9 minimal runtime.

### 8.3 Design Principles
- No `async` reserved keyword or keyword added (only `await`).
- `fn` and `lime` fully share the return value type system (existing). Codegen
  uses the same return value ABI.
- No special type system for async-only (only Runtime execution model).

---

## 9. FFI / Runtime Design

Minimal Runtime (`runtime.c` + `runtime.rs` extern declarations). All `extern "C"`,
`#[no_mangle]`, C ABI.

| Runtime Symbol | Role |
|----------------|------|
| `runtime_alloc(size, align) -> i8*` | Heap allocation (§3) |
| `runtime_free(i8*)` | Linear deallocation (§5.3 Phase 2) |
| `runtime_str_from_utf8 / concat / len / slice` | String operations |
| `runtime_list_new / push / get / len` | List operations |
| `runtime_print(i8*, len)` | Standard output (behind existing `print`) |
| `runtime_panic(msg)` | Unreachable/overflow etc. (no exceptions, abort) |
| `runtime_async_schedule(Future*)` | Async scheduler (§8) |

- Language built-ins `print/len/StringBuilder/int/float...` all lower to `call`
  to the above Runtime (1:1 correspondence with existing Interpreter builtin
  matching).
- Floating point / integer arithmetic uses LLVM IR native instructions
  (`add`/`fadd` etc.).
- Operator `resolved_operator` (Operator Interface) maps directly to concrete
  function `call`.

### 9.1 Stdlib Runtime Builtin Integration (Phase 12 Step 1)

Wrapper functions in bundled stdlib packages (`string`/`math`/`time`/`fs`/`io`)
are lowered to C helper calls so they work in the native backend
(`codegen_runtime_builtin` in `fn_builder.rs`).

- Wrapper functions (e.g., `string.trim`) are merged into `defs.functions` with
  dotted names and resolved via `Defs::resolve_function` / `Defs::resolve_type`
  (`pub(crate)`) fallback. Bare type names (e.g., `Instant(f)`) are resolved to
  `time.Instant` via `resolve_type`.
- `codegen_call` dispatch order: runtime builtins → struct constructors →
  state constructors → monomorphic functions → user functions.
- Correspondence table: see `docs/runtime.md` "Stdlib builtin helpers."
  `split`/`fs_list_dir` returning string lists return `%LimeList` via MSVC
  `sret` ABI.
- `compile_runtime_c()` embeds and compiles `runtime.c` on-the-fly with clang,
  naming the `.obj` by a hash of the source content (preventing stale cached
  objects from being linked when the source is edited).

---

## 10. Phase Splitting (gradual implementation plan)

### Phase 0 — Foundation (non-breaking)
- Add `inkwell` to `Cargo.toml`. Initialize `Context/Module/Builder/TargetMachine`
  in `codegen/mod.rs`, generate an empty `main` and verify it produces an
  executable (Hello-world equivalent).
- After `type_check`, run `lower_to_typed` to create Typed AST (expressions only
  first).
- Diff test infrastructure: compare Interpreter output with LLVM output.

### Phase 1 — Scalar + Control Flow
- `int/float/bool/str` literals, `let`, assignment, `if/else`, `while`,
  `for` (range), `return`, binary operations (statically resolved), `print`.
- Memory: all `let` via `alloca` (escape handled later). First pass through with
  Stack only.
- Goal: execute existing `steptest_*` scalar portions with LLVM and match
  Interpreter output.

### Phase 2 — Struct / State / Match
- `struct` layout, constructor, field access (§4).
- `State`/`Result`/`Option` tagged union + `match` branching (§4.4).
- Exhaustiveness trusts existing TypeChecker results.

### Phase 3 — Heap + Memory Analysis Reflection
- Emit `runtime_alloc`/`free` per §3. Escape values and explicit heap → heap.
- §5.3 linear free insertion (`runtime_free` after last use).

### Phase 4 — List / String Runtime
- Implement §5 Runtime functions, lower List/Array/Range iteration and String
  API to LLVM.

### Phase 5 — Generic (Monomorphization)
- Implement §6 `instantiate`. Monomorphize codegen for `List(T)`/`Result(T,E)`/
  `Vec2(T)` etc.

### Phase 6 — Interface Dispatch
- Implement §7.1 static devirtualize. Add §7.2 vtable/fat-pointer if needed.

### Phase 7 — Async (synchronous expansion)
- Generate `lime`/`await` with §8.1 Future struct + simple scheduler.
- Goal: execute `steptest_async.lime` with LLVM and match Interpreter output.

### Phase 8 — Optimization + True Async + Executable
- `PassManager` for O2-equivalent optimization.
- §8.2 coroutine state machine (optional).
- `TargetMachine` for `.o` / executable generation, link with Runtime via
  `clang`/`cc`.

---

## 11. Risks and Pending Decisions

- **LLVM version dependency**: `inkwell`/`llvm-sys` depend on system LLVM.
  CI requires LLVM setup. As a fallback, `#[cfg]` can keep the Interpreter.
- **Memory deallocation policy**: Phase 1 tolerates leaks. Final is linear free
  insertion (no RC/GC).
- **Async concurrency**: Phase 1 is single-threaded synchronous expansion.
  True concurrency is Phase 8 onward.
- **Generic code bloat**: monomorphization inflates binaries, but Lime is a
  small language so it's tolerable. Sharing (via Interface within generic
  functions) is suppressed by vtable.

---

## 12. Next Actions

1. Create Phase 0 `Cargo.toml` / `codegen/mod.rs` skeleton and verify empty `main` execution.
2. Add diff test infrastructure (Interpreter vs LLVM) in `sandbox/`.
3. Implement phases sequentially, verifying `steptest_*.lime` consistency at each phase.
4. `git commit` after each phase completion.
