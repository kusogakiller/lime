# Phase 9 — LLVM Backend & Runtime

**Status: COMPLETE (Phase 1 / textual-IR stage)**

Phase 9 delivers a working LLVM codegen + C runtime for Lime, covering the
aggregate/collection/interface features added across prior phases. Execution
remains interpreter-driven; `codegen` produces textual `.ll` IR validated
structurally against interpreter semantics.

## Goals vs. outcome

| Goal | Outcome |
|------|---------|
| Struct codegen | ✅ ctor (`insertvalue`), field access (`extractvalue`), method call (`%S* self`). |
| State codegen | ✅ tagged-union struct + `match` as `switch` on tag. |
| List codegen | ✅ literal, `len`/`get`/`add`/`set` via runtime; printing. |
| String codegen | ✅ `len`/`slice`/`concat`/`chars`/`bytes`; printing. |
| Interface codegen | ✅ vtable globals + `%LimeIface` fat-pointer dispatch. |
| Runtime layer | ✅ `runtime.c/.h` + Rust mirror + ABI test. |
| Test expansion | ✅ 7 integration + 6 unit tests. |
| Docs | ✅ `docs/runtime.md`, `docs/codegen.md`, `docs/phase9.md`. |

## Deliverables

- **Runtime**: `src/codegen/runtime/{runtime.c,runtime.h}`, `src/codegen/runtime.rs`.
- **Codegen features** (in `src/codegen/fn_builder.rs`, `mod.rs`):
  - `codegen_for` (range + list/array `while` lowering).
  - `infer_return_type` / `call_ret_type` (return-type-less functions).
  - `emit_print_value`, `codegen_list_print`, `codegen_struct_print`.
  - `emit_vtable_decls`, `box_to_iface`, `codegen_interface_method_call`, `codegen_arg_coerce`.
  - `%LimeList` globals `@.str.lbracket/@.str.rbracket/@.str.space`.
- **Parser/TC fixes**: `type_from_str` now maps `i/f/b/s` shorthands → concrete
  types (was dropping to `Type::Var`, which broke `print`/IR for shorthand
  struct fields).
- **Examples** (interpreter-verified): `examples/phase9_demo/`, `examples/iface_demo/`.
- **Tests**: `tests/integration.rs` (`stdlib_string_math`, `collections_demo`,
  `emit_llvm_smoke`, `phase9_demo`, `iface_demo`, `emit_llvm_interface`,
  `emit_llvm_phase9_demo`); `main.rs` unit tests (`type_from_str`, optimizer
  folding, parser); `runtime.rs` layout ABI test.

## Verified outputs

- `examples/phase9_demo`: `3,30,3,4,5,hello world,42`
- `examples/iface_demo`: `woof,4,meow,woof,meow`
- `cargo test --workspace`: 13 passed (6 unit + 7 integration).
- `cargo build --workspace`: clean (warnings only on intentionally unused
  `RUNTIME_C/RUNTIME_H` path consts, marked `#[allow(dead_code)]`).

## Open items for a later phase

1. Insert `runtime_free` at last use (escape analysis / linear lifetime).
2. Assemble + link emitted `.ll` into a runnable binary (requires system LLVM).
3. Async (`lime`/`await`) codegen.
4. Unsized-interface-in-aggregate support.
