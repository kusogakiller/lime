# Iteration 1 — Report (OPT-004: inline List(T) fixed-index get)

## Goal
Remove the per-element cross-object call to `@runtime_list_get` for `xs[i]` by emitting an
inline GEP+load+`select(0)` bounds check, to shrink the list_iter/list_push/map_ops/set_ops gap
(~1.5–1.9x vs Clang O3).

## Evidence
- `runtime_list_get` is declared in `src/codegen/mod.rs` and implemented in
  `src/codegen/runtime/runtime.c`, compiled to a SEPARATE .obj. LLVM -O2 cannot inline it
  (body not in the Lime module) → real call per index op.

## Root Cause (hypothesis)
Per-op cross-object call + i64 payload load is the dominant fixed cost on Lime list access.

## Design
Inline `List(T)` index access: getelementptr data/len, load i8*, bitcast to ptr,
getelementptr i64, load i64, signed bounds check (idx>=0 && idx<len), `select i1` → 0 if OOB,
then `convert_from_i64` for element typing. Reproduces runtime_list_get semantics exactly.

## Files Changed
- `src/codegen/fn_builder.rs` `Expr::Index` arm (lines ~609-671).

## Tests
- `cargo test --workspace` 233/233 PASS (both before and after revert).

## Benchmark Before (frozen baseline, git 4019b2a) / After (OPT-004) / After (REVERTED)
| Benchmark | Frozen Lime ms | OPT-004 Lime ms | Reverted Lime ms | Clang O3 ms | Note |
|-----------|---------------:|----------------:|----------------:|------------:|------|
| list_iter | 11.86 | 13.13 | 10.51 | 7.41 | OPT-004 REGRESSED (1.56x→1.84x); reverted→1.55x |
| list_push | 11.94 | 10.53 | 10.38 | 7.49 | marginal |
| mixed_workload | NOT VERIFIED | 368.93 | 365.86 | 19.47 | ~18.8x BOTH states → NOT from OPT-004 (uses `.get`) |
| string_access | 84.78 | 78.28 | (n/a) | 19.25 | variance |
| string_concat | 233.38 | 210.79 | (n/a) | 112.18 | variance |

## Speedup
NONE. OPT-004 was a NET REGRESSION on list_iter (the targeted benchmark).

## Correctness
All 16 benchmarks MATCH (validate.py) in both OPT-004 and reverted states. No correctness break.

## Regression
YES — OPT-004 regressed list_iter (1.56x→1.84x) and did not help others. Per camp Keep/Revert:
**REVERTED**. State restored to pre-OPT-004 (list_iter 1.55x, all correctness MATCH).

## Remaining Issues
- List access at ~1.55x remains (cross-object `runtime_list_get` call survives; the inline
  version emitted MORE IR and blocked LLVM opts). The call is cheap relative to the list's
  element-access pattern; the ~1.55x is dominated by other factors (by-value struct / i64
  payload / loop structure), NOT this call. Inlining was the wrong lever here.
- mixed_workload ~18.8x is the per-char string `slice`+`concat` O(n^2) bottleneck (see
  String Design Proposal). Not caused by OPT-004.

## Next Proposed Action
Do NOT retry OPT-004. Pivot to the string bottlenecks (OPT-001 + OPT-002), which require a
Design Proposal + human approval (new string API + string layout/aliasing change). Until
approved, the only safe local wins are marginal (attribute annotations). Recommend obtaining
approval for the string plan before further iterations, since strings dominate every large gap.

## Human Approval Required
YES — String optimization plan (OPT-001 non-allocating char access; OPT-002 amortized-capacity
string concat / in-place append) touches string public API and/or string memory layout.
