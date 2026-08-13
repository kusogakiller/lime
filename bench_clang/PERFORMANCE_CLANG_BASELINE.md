# Lime Native vs Clang — Performance Baseline (Measurement-Only)

> **Phase:** Measurement ONLY. No optimization was implemented during this work.
> This document establishes the current performance of Lime Native (git `4019b2a`)
> versus Clang 22.1.8 under fair, identical conditions, so that the next optimization
> loop can be driven by evidence (Measure → Identify → Optimize → Re-measure → Compare).

---

## 1. Executive Summary

Lime Native (release, `-O2`) was benchmarked against Clang 22.1.8 (`-O2` primary,
`-O3` reference) across 16 workload categories. All benchmarks use identical
algorithms and input sizes in both languages; every result was verified for
**correctness** (Lime native output must equal Clang output) before being timed.

**Result:**

- **16 categories** attempted. **14 ran correctly in both** and were timed.
  **2 are NOT VERIFIED** due to Lime compiler/codegen limitations (float math,
  `List(str)`), not benchmark errors.
- **Lime wins: 0. Clang wins: 15.** ("approximately equal": 0.)
- Smallest gap: integer loop **1.05x** (near parity — a genuine strength).
- Largest gap: string character-access **3.81x**; string concat 1.63x;
  set 1.87x; most compute-bound loops ~1.5x.
- A serious **correctness/robustness defect** was also found: Lime Native
  **stack-overflows** on collections larger than ~5k–30k elements depending on
  shape (Clang handles millions). This blocks full-scale measurement of
  collection/algorithm categories and must be fixed before those areas can be
  fairly judged.

Clang `-O3` gave essentially no improvement over `-O2` on these workloads
(e.g. int_loop 88.6 → 88.1 ms), so the gap is a property of Lime's generated
code, not a missing optimization flag on Clang's side.

**Conclusion:** Lime Native is currently **slower than Clang in every measured
category** (1.05x–3.81x). The integer/arithmetic core is close; strings,
structs, and collections carry the largest overhead; and large-scale collection
workloads are presently **not executable** natively. The claim "Lime is faster
than Clang" is **NOT supported** by this baseline.

---

## 2. Environment

| Item | Value |
|------|-------|
| CPU | Intel x86_64, Family 6 Model 158 (Skylake-class), 8 logical cores |
| OS | Windows 10.0.26200 (build 26200), MINGW64 |
| Rust (compiler host) | 1.96.1 |
| Clang (reference) | 22.1.8 (LLVM 22.1.8), standalone bundle |
| Lime compiler | `target/release/lime.exe`, built from git `4019b2a` |
| Lime git revision | `4019b2a` (2026-08-05) + uncommitted working tree |
| Threading | single-threaded benchmarks (no parallelism) |
| Measurement clock | Python `time.perf_counter` (wall-clock), process spawn per sample |

## 3. Compiler Configuration

| Component | Configuration |
|-----------|---------------|
| Lime Native | `lime build --release --emit-object <f>.lime` → emits `<f>.ll` → `clang -O2 -c` → `lld-link` (links `runtime.c` + CRT libs). **Lime Native is therefore pinned at `-O2`.** |
| Clang O2 | `clang -O2 -o <name>_o2.exe <name>.c` |
| Clang O3 | `clang -O3 -o <name>_o3.exe <name>.c` |
| Primary comparison | **Lime `-O2` vs Clang `-O2`** (apples-to-apples; Lime cannot emit above `-O2`). Clang `-O3` reported for reference. |

**Fairness note:** Lime has no `-O3` equivalent, so the headline ratio uses
Clang `-O2`. Because Clang `-O3` ≈ `-O2` here, this does not flatter Clang.

## 4. Benchmark Methodology

- **Per benchmark:** one `.lime` (Lime) + one `.c` (Clang) implementing the *same*
  algorithm and *same* input size. C uses only its own optimizer (no SIMD, no
  OpenMP, no manual unrolling, no hand-tuned intrinsics).
- **Warmup:** 1 run (discarded). **Timed runs:** 11. **Per run:** fresh process.
- **Recorded per benchmark:** min / median / max / mean / stdev (ms).
- **Correctness gate:** Lime native stdout must equal Clang `-O2` stdout. Any
  mismatch stops the benchmark from being treated as a performance result.
- **Harness:** `bench_clang/run_benchmarks.py` (builds + times Lime and Clang O2/O3,
  checks output equality, writes `results/benchmark_results.json`).
  `bench_clang/aggregate.py` classifies by ratio.
- **Classification thresholds (policy §11):** `<0.90` significantly faster ·
  `0.90–0.98` faster · `0.98–1.02` equal · `1.02–1.10` slower ·
  `>1.10` significantly slower.

### Categories covered (micro + algorithm + realistic)
- A Integer arithmetic · B Float (NOT VERIFIED) · C Function calls · D Recursion
  (tree + tail) · E Control flow · F Struct ops · G String (concat + access) ·
  H Collections (list push/iter, map, set) · I Memory alloc · J Algorithm
  (sieve, quicksort) + realistic mixed workload (NOT VERIFIED).
- **Scale reduction:** collection/algorithm benchmarks were capped at N=5k–20k
  because Lime Native stack-overflows on larger collections (see §9). Both
  languages use the *same* reduced N, so ratios remain fair at that scale.

## 5. Benchmark Results (median ms, 11 runs)

| Benchmark | Classification | Lime / Clang-O2 | Lime med | Clang-O2 med | Clang-O3 med | Correctness |
|-----------|----------------|------|---------|--------------|--------------|-------------|
| int_loop | Lime slower | 1.047 | 92.84 | 88.63 | 88.10 | MATCH |
| control_flow | significantly slower | 1.215 | 84.29 | 69.36 | 69.95 | MATCH |
| func_call | significantly slower | 1.143 | 29.64 | 25.92 | 25.83 | MATCH |
| recursion_tree | significantly slower | 1.610 | 11.28 | 7.00 | 7.04 | MATCH |
| recursion_tail | significantly slower | 1.499 | 11.11 | 7.41 | 7.27 | MATCH |
| struct_ops | significantly slower | 1.552 | 11.81 | 7.61 | 7.06 | MATCH |
| memory_alloc | significantly slower | 1.492 | 11.08 | 7.43 | 7.17 | MATCH |
| list_push (N=20k) | significantly slower | 1.652 | 11.94 | 7.23 | 7.49 | MATCH |
| list_iter (N=20k) | significantly slower | 1.560 | 11.86 | 7.60 | 7.41 | MATCH |
| map_ops (N=5k) | significantly slower | 1.485 | 16.43 | 11.07 | 11.63 | MATCH |
| set_ops (N=20k) | significantly slower | 1.871 | 188.71 | 100.84 | 104.99 | MATCH |
| string_concat (N=30k) | significantly slower | 1.629 | 233.38 | 143.30 | 146.34 | MATCH |
| string_access (N=5k) | significantly slower | 3.805 | 84.78 | 22.28 | 20.45 | MATCH |
| algo_sieve (N=5k) | significantly slower | 1.588 | 11.75 | 7.40 | 7.55 | MATCH |
| algo_sort (N=5k) | significantly slower | 1.567 | 12.50 | 7.98 | 7.42 | MATCH |
| float_loop | **NOT VERIFIED** | – | – | 16.79 | 16.09 | (Lime build fails) |
| mixed_workload | **NOT VERIFIED** | – | – | 35.81 | 35.28 | (Lime build fails) |

- **Lime wins: 0** · **Clang wins: 15** · **Approximately equal: 0** · **Not comparable: 2**.
- **Largest Lime advantage:** int_loop (1.047x — still Lime slower).
- **Largest Lime disadvantage:** string_access (3.805x).

## 6. Lime Wins
**None.** No category was faster or approximately equal to Clang at the measured
scale. The closest (int_loop, 1.047x) is still ~5% slower.

## 7. Clang Wins
All 15 measurable categories. By margin:
- **Near parity (≤1.10x):** int_loop (1.05), func_call (1.14), control_flow (1.22).
- **Moderate (1.10–2.0x):** recursion_tree (1.61), recursion_tail (1.50),
  struct_ops (1.55), memory_alloc (1.49), list_push (1.65), list_iter (1.56),
  map_ops (1.49), algo_sieve (1.59), algo_sort (1.57), string_concat (1.63),
  set_ops (1.87).
- **Severe (>2.0x):** string_access (3.81).

## 8. Approximately Equal
**None** at the measured scale (int_loop at 1.047x is the only one inside the
1.02–1.10 "slower" band, not "equal").

## 9. Major Bottlenecks

### 9.1 MEASURED — uniform ~1.5x overhead on compute loops
Every pure-compute loop (int, recursion, struct, memory, list, map, sieve, sort)
lands at **1.45x–1.65x** vs Clang `-O2`. A consistent multiplier this tight
across different algorithms points to a **systematic per-operation cost**, not a
single bad loop. Most likely contributors (see §10): function-call/ABI overhead,
runtime helper call overhead, and LLVM IR that misses standard mid-level
optimizations.

### 9.2 MEASURED — string runtime (1.63x–3.81x)
- `string_concat` (1.63x): each `+` allocates a new buffer (`runtime_str_concat`).
- `string_access` (3.81x): per-character `.slice()` allocates a substring per call.
  This is the single worst gap and is clearly an allocation/API-shape problem.

### 9.3 MEASURED — set/map linear scan (1.49x–1.87x)
Lime's `HashSet`/`HashMap` are **linear lists** (O(n) per op). `set_ops` at 1.87x
reflects the scan cost plus Lime's collection runtime overhead. (Note: the C
reference here also uses a linear scan to mirror Lime's real semantics — Lime's
"hash map" is not hashed.)

### 9.4 MEASURED — CRITICAL DEFECT: stack overflow on large collections
Lime Native **stack-overflows** (`0xC00000FD`) on collections above a threshold:
- Plain `List(int)`: overflows between **30k and 50k** elements (single while-loop).
- `List(int)` flag array / 20k-element list + algorithm: overflows at ~20k.
- Two parallel `List(int)` (map_ops) at 20k: overflows; had to drop to 5k.
Clang handles millions effortlessly. This blocks full-scale collection/algorithm
measurement and is a correctness/robustness bug (a valid program crashes), not
merely a performance issue.

### 9.5 MEASURED — compiler/codegen limitations (NOT VERIFIED)
- **Float math:** `float_loop` fails native codegen ("some IR emitted
  incompletely" — the build correctly refuses to emit an object). Float arithmetic
  is effectively unusable in Lime Native today.
- **`List(str)`:** `mixed_workload` fails native codegen (`%t50 defined with type
  'ptr' but expected 'i64'`). Lists of strings do not codegen.
- **`List(Entry)` / lists of structs:** also fail native codegen (hit while
  prototyping the map benchmark). Only `List(int)` and `List(<scalar>)` reliably
  codegen.

## 10. LLVM / Codegen Analysis

- Lime emits textual LLVM IR, then runs `clang -O2 -c`. So Lime gets LLVM's
  standard `-O2` pipeline — it is **not** missing LLVM, it is missing
  **front-end-level** quality (the IR Lime produces is weaker than Clang's).
- Evidence the IR is sub-optimal rather than LLVM misconfigured: the uniform
  ~1.5x gap across very different loops suggests Lime emits extra indirection
  (e.g. runtime helper calls for arithmetic/comparison, value-based struct/collection
  handling, conservative ABI) that LLVM cannot fully eliminate.
- **MEASURED:** ratios are stable across 11 runs (low stdev), so the gap is
  systematic, not noise. **GUESSED (needs profiling to confirm):** exact share of
  the 1.5x attributable to (a) call/ABI overhead vs (b) missed LLVM passes vs
  (c) runtime helper cost. A future step should profile `perf`/LLVM IR diffs.

## 11. Runtime Analysis

- Lime is GC-free with a single-owner model (per policy §12); allocations are
  `malloc` via `runtime_alloc`, freed at exit. No leak-vs-correctness issue was
  observed in correctness checks.
- The dominant runtime costs observed: **per-operation allocation** in string ops
  and **linear scans** in collections. These are runtime/API-design costs, not
  memory-safety bugs.
- The **stack-overflow** defect (§9.4) is the most serious runtime issue: it
  indicates the generated code keeps unexpectedly large frames / the list header
  or loop context on the stack, or the default thread stack for the executable is
  too small. Worth distinguishing (by inspecting the emitted `.ll` and the link
  stack size) before fixing.

## 12. Optimization Candidates (proposed — NOT yet implemented)

> Priority ranking for the next optimization loop. Each needs re-measurement
> after implementation (Before → Change → After).

### Priority 1 — Fix collection stack-overflow (correctness/robustness, blocks measurement)
- **Bottleneck:** native crash on >~5k–30k element collections.
- **Evidence:** reproduceable `0xC00000FD` across list/sieve/sort at scale.
- **Expected impact:** unblocks full-scale H/I/J measurement; likely also removes
  hidden per-frame overhead.
- **Difficulty:** Medium. **Regression risk:** Low (bug fix). **Verify:** re-run
  list_push/sieve/sort at 1M; no crash; correctness MATCH.

### Priority 2 — String runtime allocation
- **Bottleneck:** `runtime_str_concat` / `.slice()` allocate per call (1.63x–3.81x).
- **Evidence:** string_access 3.81x, string_concat 1.63x.
- **Expected impact:** large win for string-heavy workloads.
- **Difficulty:** Medium. **Regression risk:** Low–Med. **Verify:** string_concat/
  string_access benchmarks.

### Priority 3 — General codegen quality (the uniform ~1.5x)
- **Bottleneck:** systematic per-op overhead (call/ABI + missed LLVM passes).
- **Evidence:** consistent 1.45–1.65x across all compute loops.
- **Expected impact:** moves int/recursion/struct/list from "significantly slower"
  toward parity.
- **Difficulty:** High (requires IR-level work). **Regression risk:** Med.
  **Verify:** int_loop/func_call/struct_ops; target ≤1.10x.

### Lower priority
- **Float codegen** (unblock `float_loop`): currently NOT VERIFIED. Difficulty Med.
- **`List(str)` / `List(Entry)` codegen**: unblock mixed_workload & richer
  collections. Difficulty Med.
- **Real hashing for HashMap/HashSet**: algorithmic, not just perf (current linear
  scan is O(n)). Difficulty Med.

## 13. Reproduction Instructions

```bat
:: from canonical repo C:\Users\szzxl\Downloads\ime
set PATH=%PATH%;C:\Users\szzxl\Downloads\clang+llvm-22.1.8-x86_64-pc-windows-msvc\clang+llvm-22.1.8-x86_64-pc-windows-msvc\bin

:: 1) build the Lime compiler (release)
cargo build --release

:: 2) (optional) confirm existing tests still pass
cargo test --workspace        :: -> 233 passed

:: 3) run the benchmark suite (builds + times Lime & Clang O2/O3, checks equality)
cd bench_clang
python3 run_benchmarks.py     :: -> results/benchmark_results.json
python3 aggregate.py          :: -> classification table

:: 4) individual check
lime.exe build --release --emit-object suite\int_loop.lime
suite\int_loop.exe
clang -O2 -o suite\int_loop_o2.exe suite\int_loop.c
suite\int_loop_o2.exe
```

Benchmark sources: `bench_clang/suite/*.lime` and `*.c`. Raw results:
`bench_clang/results/benchmark_results.json`. Validation gate:
`bench_clang/validate.py`.

## 14. Limitations

- **Scale:** collection/algorithm benchmarks run at reduced N (5k–20k) because of
  the stack-overflow defect (§9.4). Full-scale ratios are NOT VERIFIED.
- **NOT VERIFIED categories:** `float_loop` (float codegen incomplete),
  `mixed_workload` (`List(str)` codegen broken). These are Lime compiler limits,
  not benchmark flaws.
- **Lime pinned at `-O2`:** no `-O3`/LTO equivalent exists, so the comparison uses
  Clang `-O2` as the primary baseline (fair, since Clang `-O3` ≈ `-O2` here).
- **No profiling yet:** the 1.5x root-cause split (ABI vs passes vs runtime) is
  inferred, not measured via `perf`/IR diff. Marked as GUESSED in §10.
- **Single machine:** one Windows host; no cross-OS/CPU validation.
- **Micro/algorithm/realistic spread:** present, but the realistic workload
  (`mixed_workload`) is NOT VERIFIED, so the "realistic" row is represented only by
  the algorithm benchmarks at reduced scale.

## 15. Conclusion

Lime Native currently trails Clang in **every measured category** (best 1.05x on
integer loops, worst 3.81x on string access), with a systematic ~1.5x overhead
across compute-bound loops and severe string/collection costs. Additionally, a
**stack-overflow defect** makes large-scale collection and algorithm workloads
**unexecutable** natively, and float/`List(str)` codegen is broken.

This is a clean, auditable baseline. The next phase should (1) fix the
stack-overflow defect, (2) attack string allocation, and (3) raise general codegen
quality — each followed by a re-measurement against this document. The claim
"Lime is faster than Clang" is **not supported** by this baseline and must not be
made until Lime closes the measured gaps.

---

## Audit / Final Report (policy §16 / §17)

- **cargo test --workspace:** PASS (233/233).
- **Lime native build:** PASS (compiler builds; all 14 runnable benchmarks build to exe).
- **Lime native execution:** PASS (14/14 runnable produce correct output);
  NOT VERIFIED for `float_loop` and `mixed_workload` (compiler codegen limits).
- **Clang build:** PASS (all 16 `.c` compile at `-O2` and `-O3`).
- **Benchmark count:** 16 (14 measured + 2 NOT VERIFIED).
- **Lime wins:** 0. **Clang wins:** 15. **Approximately equal:** 0.
- **Largest Lime advantage:** int_loop (1.047x — still Lime slower).
- **Largest Lime disadvantage:** string_access (3.805x).
- **Main bottleneck:** systematic ~1.5x codegen/ABI overhead + string allocation
  (up to 3.8x) + collection stack-overflow defect.
- **Optimization priority:** P1 stack-overflow fix → P2 string runtime →
  P3 general codegen quality.
- **Release impact:** Lime Native is NOT release-ready for performance parity with
  Clang; the gaps above are release blockers for any "fast" claim. Correctness of
  the 14 runnable benchmarks is solid (MATCH). Two compiler limitations and one
  robustness defect must be resolved before a performance-focused release.
- **Human decision required:** approve the P1→P2→P3 optimization sequence (or
  reprioritize) before any code change begins. No optimization was performed in
  this phase per the "measure first" directive.
