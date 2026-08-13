# Iteration 1 — OpenCode Prompt (OPT-004: inline List(T) fixed-index get)

> Saved per protocol §12 (audit trail). Delegation to OpenCode was BYPASSED this iteration
> because `opencode run` returned empty output / previously `UnknownError` (CLI unavailable in
> this environment). Hermes implemented + independently verified directly. Prompt retained for
> traceability.

## 1. Objective
Remove the per-element cross-object call to `@runtime_list_get` for `List(T)` indexed access
(`xs[i]` / `list_get(xs, i)`) by emitting the element load inline in the Lime LLVM IR, keeping
identical bounds-check semantics. Target: reduce list_iter / list_push / map_ops / set_ops
Lime-vs-Clang gap (currently ~1.5–1.9x).

## 2. Current verified state
- `runtime_list_get` is declared in `src/codegen/mod.rs` and implemented in
  `src/codegen/runtime/runtime.c` (compiled to a SEPARATE .obj, linked by `lime build`).
  Because its body is NOT in the Lime module, LLVM -O2 CANNOT inline it → it is a real call
  every index op.
- `runtime_list_get(ptr, i64)` body: `if (index >= 0 && index < len) return data[index]; else return 0;`
  where `%LimeList = { i8* data, i64 len, i64 cap }`.
- `%LimeList` element storage is `i64` (see fn_builder.rs list builtins); `List(str)` stores an
  `i8*` payload as `i64` (handled by `convert_from_i64` / `convert_to_i64`, added in Phase 1).

## 3. Problem evidence
- `bench_clang/profiling/list_iter/list_iter.ll`: each `xs[i]` lowers to
  `call i64 @runtime_list_get(ptr ..., i64 ...)`. 20k calls in the loop.
- Clang reference (`list_iter_clang_o2.ll`) accesses its array via inline `getelementptr`+`load`
  with no helper call. Gap ~1.56x.

## 4. Root-cause hypothesis
Per-op cross-object call + i64 payload load is the dominant fixed cost on Lime list access;
LLVM cannot inline across the object boundary.

## 5. Required investigation
Confirm `%LimeList` field offsets (data=0, len=1, cap=2) and that no other code path relies on
`runtime_list_get` side effects (it has none; it is pure read).

## 6. Implementation constraints
- Emit inline IR for `List(T)` index that reproduces EXACTLY: signed bounds check
  `idx >= 0 && idx < len`, return element value if in range else `0` (i64) / `null` (i8*) /
  `0.0` (double) / `false` (i1) / zeroed struct.
- Preserve element typing: `List(int)` -> i64 load; `List(str)` -> load i64 then inttoptr to i8*;
  `List(float)` -> load i64 then bitcast to double; `List(bool)` -> load i64 then trunc to i1;
  `List(struct S)` -> load i64 then inttoptr to ptr %S* . Use existing `convert_from_i64`.
- Do NOT change `%LimeList` layout, runtime API, or ABI. This is codegen-only.
- Negative / out-of-range index MUST return the same value as `runtime_list_get` (0 / null),
  so semantics are preserved (no UB, no behavior change).

## 7. Files likely affected
- `src/codegen/fn_builder.rs` — `Expr::Index` (line ~609) and the `"list_get"` builtin arm (~2282).
- Possibly `codegen_method_call` "get" path if it also routes through runtime_list_get.

## 8. Explicit non-goals
- Do NOT modify `runtime.c` list semantics.
- Do NOT add a capacity field / change string or list memory layout.
- Do NOT touch `set_ops`/`map_ops` algorithm (linear scan) — separate OPT-005 (Design Proposal).

## 9. Required tests
- `cargo test --workspace` (must remain 233/233).
- `bench_clang/validate.py` — all 16 benchmarks build/run/correctness MATCH.

## 10. Required benchmarks
- Before/After on full suite; focus on list_iter, list_push, map_ops, set_ops, struct_ops.

## 11. Regression requirements
- No correctness change: `validate.py` MATCH for all 16. `cargo test` 233/233.

## 12. Acceptance criteria
- `xs[i]` for `List(int)` lowers to `getelementptr`+`load`+`select` (no `@runtime_list_get` call)
  in the emitted IR, while preserving bounds-check return value.
- list_iter/list_push Lime median moves toward Clang O3 (measured, not asserted).
- No regression in correctness or other benchmarks.

## 13. Reporting requirements
- Show git diff, emitted IR snippet (before/after), benchmark Before/After, correctness, regression.
