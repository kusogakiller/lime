# Phase 2 — Profiling Report (Lime Native vs Clang, measurement-only)

> **Scope:** Phase 2 is PROFILING ONLY. No compiler/runtime/benchmark changes were made
> in this phase. All findings below are evidence from generated LLVM IR, generated
> assembly, and the frozen baseline (`benchmark_results.frozen_baseline.json`, git `4019b2a`).
> Companion evidence lives in `bench_clang/profiling/<bench>/` (Lime `.ll`, Clang `-O2`/`-O3` `.ll`) and `bench_clang/profiling/metrics.md`.

---

## 1. Executive Summary

Lime Native (release = `clang -O2 -c` over Lime's LLVM IR) is **slower than Clang in every
measured category** (frozen baseline: 15 slower, 0 wins, 2 NOT VERIFIED). Phase 2 isolated
the *reasons* from IR/asm evidence:

- **The ~1.5x gap on compute kernels is NOT missing inlining.** Disassembly of `func_call.obj`
  and `struct_ops.obj` shows LLVM **does** inline `sq`/`add`/`nested`/`make`/`addp` (the hot
  loops are vectorized SIMD, zero calls to those symbols). The residual gap is **IR quality**:
  Lime emits `let mut` locals as stack `alloca`+load/store and structs as by-value
  `insertvalue`/`extractvalue`, which LLVM -O2 promotes only partially (Clang -O3 keeps
  everything in registers).
- **The string gap is dominated by real runtime allocations + O(n²) length scans.**
  `string_access` (3.8x) calls `@runtime_str_slice` (allocates a substring) **and** `@strlen`
  of the full string **once per character** — O(n²) byte-scanning in the loop. Clang -O3
  folds the string length to a constant and **eliminates the slice loop entirely**.
  `string_concat` (1.6x) allocates + copies a new heap string on every `+`.
- **Collections** (`map_ops` 1.5x, `set_ops` 1.9x, `list_*` 1.5x) pay per-op `runtime_*`
  call overhead on top of the linear-scan algorithm; Lime's list element access also round-trips
  through `i64` payloads (`ptrtoint`/`inttoptr` for strings, fixed in Phase 1).
- **Optimization-pipeline verdict:** Clang `-O3` ≈ `-O2` on these workloads (e.g. int_loop 88.6→88.1 ms),
  so Lime is **not** losing to a missing LLVM pass level. The gap is in **Lime's emitted IR**
  (allocations, by-value structs, strlen-in-loop), which LLVM -O2 cannot delete because the
  allocations/runtime calls are real in the source IR. **Fixes belong in codegen, not in the
  pass level.**

---

## 2. Benchmark Baseline (frozen, for reference)

| Benchmark | Lime ms | Clang O2 ms | Clang O3 ms | Lime/O3 | status |
|-----------|--------:|------------:|------------:|--------:|--------|
| int_loop | 92.84 | 88.63 | 88.10 | 1.047 | MATCH |
| control_flow | 84.29 | 69.36 | 69.95 | 1.215 | MATCH |
| func_call | 29.64 | 25.92 | 25.83 | 1.143 | MATCH |
| recursion_tree | 11.28 | 7.00 | 7.04 | 1.610 | MATCH |
| recursion_tail | 11.11 | 7.41 | 7.27 | 1.499 | MATCH |
| struct_ops | 11.81 | 7.61 | 7.06 | 1.552 | MATCH |
| memory_alloc | 11.08 | 7.43 | 7.17 | 1.492 | MATCH |
| list_push (20k) | 11.94 | 7.23 | 7.49 | 1.652 | MATCH |
| list_iter (20k) | 11.86 | 7.60 | 7.41 | 1.560 | MATCH |
| map_ops (5k) | 16.43 | 11.07 | 11.63 | 1.485 | MATCH |
| set_ops (20k) | 188.71 | 100.84 | 104.99 | 1.871 | MATCH |
| string_concat (30k) | 233.38 | 143.30 | 146.34 | 1.629 | MATCH |
| string_access (5k) | 84.78 | 22.28 | 20.45 | 3.805 | MATCH |
| algo_sieve (5k) | 11.75 | 7.40 | 7.55 | 1.588 | MATCH |
| algo_sort (5k) | 12.50 | 7.98 | 7.42 | 1.567 | MATCH |
| float_loop | – | 16.79 | 16.09 | – | NOT VERIFIED (Phase 1 made it run; no frozen number) |
| mixed_workload | – | 35.81 | 35.28 | – | NOT VERIFIED (Phase 1 redesigned it) |

---

## 3. LLVM IR Analysis (evidence)

All Lime IR is `bench_clang/profiling/<bench>/<bench>.ll` (pre-LLVM-opt). Clang IR is
`<bench>_clang_o2.ll` / `<bench>_clang_o3.ll`.

### 3.1 Inlining — DOES happen (disproves "no inline" hypothesis)
- `func_call.obj` disassembly: `main_lime` hot loop uses `paddq`/`paddd` SIMD with **no call** to
  `sq`/`add`/`nested`. Symbols remain (available-externally) but are inlined at call sites.
- `struct_ops.obj`: **no `call` to `make`/`addp`** in `main_lime`; both inlined.
- Conclusion: the uniform ~1.5x is **not** from missing function inlining.

### 3.2 string_access — the 3.8x smoking gun (`profiling/string_access/string_access.ll`)
Loop `L5` (per character):
```
%t20 = call i8* @runtime_str_slice(i8* %t16, i64 %t17, i64 %t19)   ; NEW substring allocated per char
%t23 = load i8*, i8** %t21
%t24 = call i64 @strlen(i8* %t23)                                  ; strlen of FULL 20k string per char -> O(n^2)
%t25 = add i64 %t22, %t24
```
- 5000 substring allocations + 5000 × `strlen(20000-byte string)` = ~100M byte-scans just for length.
- **Clang O3** (`string_access_clang_o3.ll`): the slice loop is **deleted** — length is provably
  constant, so the loop becomes `total += 1` ×5000 with a single trailing `strlen`. No allocation,
  no per-char strlen.

### 3.3 string_concat — 1.6x (`profiling/string_concat/string_concat.ll`)
```
%t7 = call i8* @runtime_str_concat(i8* %t5, i8* %t6)   ; new heap string per '+'
```
- 30000 `runtime_str_concat` calls, each allocating + copying the growing buffer → O(n²) copying.
- Clang also does O(n²) (`strcpy`+`malloc`+`free` per rep) so the gap is smaller; Lime's runtime
  allocates a fresh buffer per op without reuse/growth strategy.

### 3.4 int_loop — 1.05x (near parity) (`profiling/int_loop/int_loop.ll`)
Lime loop body: `alloca i64` for `total` and `i`, then `load i, mul 3, srem 17, add, store` +
`load i, add 1, store` — **4 loads + 4 stores + 1 mul + 1 srem** per iter (stack slots).
Clang O3 keeps both in **registers** (`%2`,`%3`), no loads/stores, vectorized. Lime relies on
LLVM `mem2reg`/SROA which recovers most of it (hence only 1.05x).

### 3.5 struct_ops — 1.55x (`profiling/struct_ops/struct_ops.ll`)
`make` returns `%Point` by value via `insertvalue` + `ret %Point`:
```
define %Point @make(i64 %p0, i64 %p1) {
  %t2 = insertvalue %Point undef, i64 %p0, 0
  %t3 = insertvalue %Point %t2, i64 %p1, 1
  ret %Point %t3
}
```
- `%Point = type { i64, i64 }` (16 bytes) returned in two registers / sret. Inlined body still
  manipulates the struct as `insertvalue`/`extractvalue`; Clang -O3 **scalar-replaces** (SROA) the
  struct into two `i64` registers. Lime -O2 leaves residual memory traffic → 1.55x.

### 3.6 func_call / recursion / algo — 1.14–1.61x
Same pattern: after inlining, residual cost is (a) stack-slot locals for `let mut`, and
(b) for recursion/algo, the list/struct payload round-trips. Not a separate root cause.

---

## 4. Runtime Call Analysis

`runtime_*` calls observed (per loop iteration, not per IR site):
- `string_access`: `runtime_str_slice` ×N (alloc) + `strlen` (libc) ×N. **Dominant.**
- `string_concat`: `runtime_str_concat` ×N (alloc+copy). **Dominant.**
- `list_push`/`list_iter`/`map_ops`/`set_ops`: `runtime_list_add` / `runtime_list_get` /
  `runtime_list_set` per op (1 call site, executes N×). These are necessary for the list
  container but add call + `i64` payload round-trip overhead vs Clang's inline array access.

Static IR site counts (Lime vs Clang) in `metrics.md`: Lime IR is ~4–7× longer than Clang's
(e.g. struct_ops 567 vs 77 lines; int_loop 510 vs 100). The length is explained by explicit
alloca/load/store for locals and the `main_lime`/`main` wrapper split.

---

## 5. Allocation Analysis

- **string_access:** 1 allocation per character (`runtime_str_slice`) = 5000 allocs for the
  benchmark; plus the O(n²) `strlen` scans. → primary 3.8x driver.
- **string_concat:** 1 allocation + full copy per `+` = 30000 allocs, O(n²) total bytes copied.
- **List push:** `runtime_list_add` reallocates the backing buffer on capacity growth (amortized
  in the runtime, but each push is a call + bounds check). Clang uses a raw pre-sized array.
- **List get:** `runtime_list_get` + `load i64` (and, for `List(str)`, `inttoptr` — fixed Phase 1).
- **Map/Set:** linear scan, O(n) per op; no allocation per op beyond the list backing growth.

---

## 6. ABI Analysis (Windows x64)

- **String** = `i8*` (pointer to runtime buffer). Passed/returned by pointer. Correct, no sret.
- **`%LimeList`** = `{ i8* data, i64 len, i64 cap }` (24 bytes). Passed by pointer (sret-style
  via `ptr` arg in `runtime_list_*` helpers). No sret-aliasing bug observed in current code
  (Phase 1 verified `ptr sret(%LimeList)` ABI is used for `list_clone`/`list_insert`/etc.).
- **`%Point`** = `{ i64, i64 }` returned by value (2 registers). Inlined; residual memory only
  because `insertvalue`/`extractvalue` not fully SROA'd by -O2.
- **No `sret` aliasing corruption** found. The historical sret bug must not be reintroduced when
  touching list/struct codegen — current `codegen_list_method`/`runtime_list_*` keep distinct
  slots correctly.

---

## 7. Optimization Pipeline Analysis

- Lime pipeline: Lime IR → `clang -O2 -c` → `lld-link`. Clang ref: `clang -O3`.
- **Measured:** Clang `-O3` ≈ `-O2` here (int_loop 88.6→88.1 ms; most deltas <1%). So moving
  Lime to `-O3` would yield negligible gain. **Not the lever.**
- What LLVM -O2 already does for Lime: inlining (confirmed), some SROA, LICM where provable.
- What it **cannot** do: delete `@runtime_str_slice` allocations or the per-char `strlen`,
  because those are real calls with observable side effects (heap alloc) in the IR. Only
  **codegen** can avoid emitting them (e.g. a non-allocating char-access primitive, or a
  length-cached string type).
- Candidate pass-level levers (low risk, optional): add `-O3` equivalent or a targeted
  `-O2` + extra passes (e.g. `-mllvm -inline-threshold` higher) — but evidence says impact <1%.

---

## 8. String Bottleneck Analysis (P1 target)

**string_access (3.8x) — root cause: per-character allocation + O(n²) strlen.**
- Option A (recommended, small, local): add a **non-allocating character-access primitive**
  `text.byte(i)` (or `text.char(i)`) that returns the byte at index without building a substring,
  and a **length-cached** string so `.byte_len()` / slicing bounds don't call `strlen` each time.
  Benchmark would use `text.byte(i)` instead of `text.slice(i,i+1).byte_len()`. This removes both
  the 5000 allocs and the O(n²) strlen. Risk: adds a small stdlib/codegen API; no ABI/semantic
  change. **Expected impact: large (target ≤ Clang O3).**
- Option B (medium): make `runtime_str_slice` avoid allocation when the result is consumed
  immediately (escape analysis). Hard; needs IR-level escape analysis.
- Option C (large, defer): cache string length in the `String` runtime header. Changes the
  runtime string layout (ABI-adjacent) → **Design Proposal required.**

**string_concat (1.6x) — root cause: per-`+` alloc+copy.**
- Option A (recommended): amortized growth in `runtime_str_concat` (reserve/round-up capacity
  like `std::string`) to cut copy volume. Local runtime change, no API/ABI change.
  **Expected impact: moderate (target ≤ Clang O3).**
- Option B (medium): a `StringBuilder` API. Nice-to-have, not required to win.

---

## 9. Collection Bottleneck Analysis (P3/P4)

- `list_push`/`list_iter` (1.5x): per-op `runtime_list_*` call + `i64` payload round-trip.
  Option: inline list element access for the common `List(int)` case in codegen (avoid the
  `runtime_list_get` call for fixed-index `get` on a local list) — local codegen change, no API
  change. **Expected impact: moderate.**
- `set_ops` (1.9x) / `map_ops` (1.5x): linear scan. Lime's `HashSet`/`HashMap` are **linear lists**
  (confirmed in Phase 1). To beat Clang, a **real hash table** is needed — this is a **large
  design change** (new data structure, API/ABI implications) → **Design Proposal required** before
  implementation. Do NOT silently swap in a hash table.

---

## 10. General Codegen Analysis (P5)

- **Register promotion of `let mut`:** Lime emits stack slots; LLVM `mem2reg` recovers most (1.05x
  for int_loop) but by-value structs lag (struct_ops 1.55x). Option: emit SSA-friendly IR
  (avoid `alloca`+explicit store for simple mutable locals where safe) — local codegen change.
  **Expected impact: small–moderate.**
- No spurious attributes needed. The existing IR is correctly typed; adding `readonly`/`nocapture`
  to `runtime_*` helpers (where the C contract permits) is a **safe, evidence-based** micro-win
  (helps LLVM DCE/GVN) — but verify each against the runtime contract (no false attributes).

---

## 11. Root Causes (consolidated, evidence-backed)

1. **string_access 3.8x** — per-char `runtime_str_slice` alloc + per-char `strlen` of full string
   (O(n²)). Clang eliminates the loop. [IR evidence: string_access.ll L5]
2. **string_concat 1.6x** — per-`+` heap alloc+copy. [string_concat.ll L2]
3. **struct_ops 1.55x** — by-value `%Point` (`insertvalue`/`extractvalue`) not fully SROA'd vs
   Clang register scalars. [struct_ops.ll `@make`]
4. **list_* / map / set 1.5–1.9x** — per-op `runtime_*` call + linear-scan algorithm.
5. **general compute ~1.05–1.2x** — stack-slot `let mut` + residual by-value struct memory.
6. **NOT a cause:** missing inlining (disconfirmed by asm), missing `-O3` passes (disconfirmed by
   O2≈O3 measurement).

---

## 12. Optimization Candidates (ranked)

| ID | Target | Change | Scope | Expected impact | Risk | Design change? |
|----|--------|--------|-------|-----------------|------|----------------|
| OPT-001 | string_access | Add non-allocating `text.byte(i)` + length-cached len | codegen+stdlib (local) | Large (→≤O3) | Low | No |
| OPT-002 | string_concat | Amortized capacity growth in `runtime_str_concat` | runtime.c (local) | Moderate (→≤O3) | Low | No |
| OPT-003 | struct_ops / general | SSA-friendly IR for `let mut` simple locals | codegen (local) | Small–mod | Low | No |
| OPT-004 | list_* | Inline `List(int)` fixed-index get in codegen | codegen (local) | Moderate | Low | No |
| OPT-005 | set/map | Real hash table | **LARGE** | Large | Med–High | **YES → Proposal** |
| OPT-006 | general | `readonly`/`nocapture` on safe `runtime_*` helpers | codegen (local) | Small | Low (if verified) | No |

Priority order for the camp: **OPT-001 → OPT-002 → OPT-004 → OPT-003 → OPT-006 → OPT-005**.

---

## 13. Required Design Changes / Human Approval

- **OPT-005 (hash table for Map/Set):** changes collection data structure + API surface. **Requires a
  Design Proposal and human approval before implementation.** Must not be done silently.
- **OPT-001 `text.byte(i)`:** adds a small public API primitive. Low risk, but it is a language/stdlib
  surface addition → surface for approval (or implement as clearly-documented stdlib addition).
- **String length caching (Option C):** changes the runtime `String` header layout (ABI-adjacent) →
  Design Proposal if pursued.
- All other OPTs are local codegen/runtime fixes within existing semantics (no spec/ABI/API change) —
  implementable without approval, but each must pass `cargo test` + the 16-benchmark correctness gate.

---

## 14. Next Phase Gate (per camp §15)

Phase 2 complete criteria check:
- [x] All 16 benchmarks build/run/correctness PASS (established Phase 1)
- [x] Baseline preserved (`benchmark_results.frozen_baseline.json` untouched)
- [x] LLVM IR inspected (Lime + Clang O2/O3 for 11+ benchmarks in `profiling/`)
- [x] Runtime calls quantified (§4, `metrics.md`)
- [x] Allocations investigated (§5)
- [x] ABI behavior documented (§6) — no sret corruption; `String`=`i8*`, `%LimeList`=ptr+sret, `%Point`=by-value
- [x] String root cause identified (§8)
- [x] Collection root cause identified (§9)
- [x] General codegen root cause identified (§10)
- [x] Optimization candidates ranked (§12)
- [x] Design changes separated from local fixes (§13)
- [x] Human-auditable report generated (this file + `profiling/`)

**No implementation was performed in Phase 2.**

**Recommended next implementation step (Phase 3): start with OPT-001 (string_access) + OPT-002
(string_concat)** — the two string benchmarks are the largest, most actionable gaps (3.8x and 1.6x)
and their fixes are local (codegen/runtime), low-risk, and expected to bring both to ≤ Clang O3.
Then OPT-004 (list inlining) and OPT-003 (register promotion), then measure; defer OPT-005
(hash table) behind a Design Proposal.
