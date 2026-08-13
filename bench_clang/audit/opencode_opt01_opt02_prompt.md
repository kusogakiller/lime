# OpenCode Delegation — OPT-001 + OPT-002 (String War, approved design change)

> Dispatched via `opencode run` by Hermes (Orchestrator). Human approved the string
> design change (public API addition + string memory-layout change) on 2026-08-13.
> Independent verification by Hermes after OpenCode returns.

## 1. Objective
Close the Lime-vs-Clang gap on string benchmarks (string_access ~3.8x, string_concat ~1.6x,
mixed_workload ~18.8x) by:
- OPT-001: add a non-allocating character-access primitive `str.byte(i): int`.
- OPT-002: give Lime-managed strings an 8-byte capacity header so `str + str` reuses the
  left operand's buffer with amortized growth (eliminates O(n^2) copy in `s = s + "y"` loops).

Target: Lime string_access <= Clang O3 median AND string_concat <= Clang O3 median
(measured, not asserted). mixed_workload should also improve materially.

## 2. Current verified state (evidence)
- Linked runtime is `src/codegen/runtime/runtime.c` (embedded via `include_str!("codegen/runtime/runtime.c")`
  in `src/lib.rs`; compiled by `lime build` to a SEPARATE hashed `.obj`, then lld-linked).
  The standalone `runtime/runtime.c` (100 lines) is UNUSED. Edit `src/codegen/runtime/runtime.c`.
- `runtime_str_concat` (runtime.c ~line 172): `malloc(la+lb+1); memcpy; memcpy; return`.
  Every `s = s + "y"` re-mallocs the whole growing string -> O(n^2).
- `runtime_str_slice` (runtime.c ~line 151): allocates a fresh substring every call.
- string_access.ll: `text.slice(i,i+1)` lowers to `runtime_str_slice` (alloc per char) +
  `strlen` of the full string per char (O(n^2) scans). Clang folds length to constant and
  deletes the loop.
- `runtime.h` / `RUNTIME_H_SOURCE` (in `src/lib.rs`): declare new runtime functions there.
- Codegen string methods are in `src/codegen/fn_builder.rs` `codegen_string_method`
  (len/byte_len/slice/...). `declare` lines for runtime helpers are in `src/codegen/mod.rs`.
- Frozen baseline medians (ms): string_access Lime 84.78 / Clang O3 20.45;
  string_concat Lime 233.38 / Clang O3 146.34; mixed_workload Lime NOT VERIFIED (now ~365 / 19.5).

## 3. Problem evidence
- `bench_clang/profiling/string_access/string_access.ll` L5: per-char `runtime_str_slice` + per-char `strlen`.
- `bench_clang/profiling/string_concat/string_concat.ll`: per-`+` `runtime_str_concat` (malloc+copy).
- See `bench_clang/PERFORMANCE_PROFILING_PHASE2.md` §8 for full analysis.

## 4. Root-cause hypothesis
Lime strings are immutable values allocated per operation; no capacity is retained across
`+`, and character access allocates a substring + recomputes length. Clang avoids both.

## 5. Required investigation (do this first, report findings)
- Read `src/codegen/runtime/runtime.c` `runtime_str_concat`, `runtime_str_slice`,
  `runtime_str_new` (if any), and the `LimeList`/string typedefs.
- Read `src/lib.rs` `RUNTIME_C_SOURCE` / `RUNTIME_H_SOURCE` to learn how runtime is embedded
  and where to add declarations.
- Read `src/codegen/mod.rs` runtime `declare` block; note the exact `declare` syntax used.
- Read `src/codegen/fn_builder.rs` `codegen_string_method` to see how `len`/`slice` lower,
  and add a `byte` arm.
- Confirm how string literals are emitted (`@.str.N` globals) so OPT-002 does NOT corrupt them.

## 6. Implementation constraints (PRECISE — follow exactly)
### OPT-002 — string capacity header (layout change, APPROVED)
- A Lime-managed String value remains an `i8*` pointing at NUL-terminated UTF-8 bytes.
- NEW invariant: for strings ALLOCATED BY THE LIME RUNTIME (via the new allocator or via
  `runtime_str_concat`/`runtime_str_slice`), the 8 bytes immediately BEFORE the data pointer
  hold the allocated capacity as a little-endian `i64` (header at `(char*)s - 8`).
  `cap = *(i64*)((char*)s - 8)`. For string LITERALS and any externally-owned `i8*` (no
  header), treat `cap` as 0 / "not owned" — never write a header before them.
- Add `char* runtime_str_new(int64_t cap)`: allocate `cap + 1 + 8` bytes, store `cap` at
  header, set `data[0]='\0'`, return `ptr + 8` (the data pointer).
- Rewrite `runtime_str_concat(char* a, char* b)`:
  - `la = strlen(a); lb = strlen(b); need = la + lb + 1;`
  - Determine if `a` is Lime-owned: read `cap_a = *(i64*)((char*)a - 8)` ONLY IF `a` was
    allocated by the runtime. To detect safely without corrupting literals, use this rule:
    the runtime marks owned strings by storing `cap >= 0` in the header; literals have NO
    header (reading `a-8` would be UB). Therefore: ONLY call `runtime_str_concat` reuse path
    when the left operand came from a prior runtime allocation. Since the codegen for `s = s + b`
    cannot guarantee that, implement the SAFE rule: if `a` is owned (detect via a sentinel —
    see below) and `need <= cap_a`, reuse: `memcpy(a + la, b, lb + 1); return a;`
    else allocate `char* r = runtime_str_new(MAX(need, (la+lb)*2))` (amortized growth),
    `memcpy(r, a, la); memcpy(r+la, b, lb+1); return r;`
  - SENTINEL for owned detection: store capacity as a POSITIVE i64 in the header. For strings
    that are NOT owned (literals), the codegen must NOT pass them to the reuse path. The
    simplest correct approach: `runtime_str_concat` ALWAYS allocates a fresh owned string on
    the FIRST concat (when `a` is a literal with no header), and reuses on subsequent concats
    (when `a` is owned). To distinguish, have `runtime_str_new` set a header, and have
    `runtime_str_concat` check: if `*(i64*)((char*)a - 8)` is readable AND `cap_a >= la`
    (consistent), treat as owned and reuse; otherwise allocate fresh. Document this heuristic.
  - The contract: "the left operand of `+` may be reused/destroyed; the caller must not retain
    other references to it across a `+`." This holds for `s = s + b`. Keep this contract in a comment.
- Update `runtime_str_slice` to allocate via `runtime_str_new` (with a header) so slices are
  also owned (so chained slices could reuse, though slice reuse is secondary). Keep slice
  semantics identical (returns a new substring, NUL-terminated).
- ALL OTHER string helpers (`runtime_strlen`/`strlen`, `runtime_str_eq`, `runtime_str_compare`,
  `runtime_str_contains`, `runtime_str_starts_with`, `runtime_str_split`, etc.) take `i8*`
  data and treat it as a plain C string — they need NO change (they never read the header).
  Verify each still compiles and behaves.

### OPT-001 — `str.byte(i): int`
- Add `int64_t runtime_str_byte(char* s, int64_t i)`: `len = strlen(s); if (i >= 0 && i < len)
  return (int64_t)(unsigned char)s[i]; else return -1;` (bounds-safe; no allocation).
- `runtime.h`: declare `int64_t runtime_str_byte(char*, int64_t);` and `char* runtime_str_new(int64_t);`.
- `src/codegen/mod.rs`: add `declare i64 @runtime_str_byte(i8*, i64)` and
  `declare i8* @runtime_str_new(i64)` (with `nounwind`/`nocapture` where safe:
  `runtime_str_byte` is `readonly nounwind nocapture`; `runtime_str_new` is `nounwind`).
- `src/codegen/fn_builder.rs` `codegen_string_method`: add arm `"byte"` ->
  `call i64 @runtime_str_byte(i8* {obj}, i64 {idx})`, return `Type::Int`.

### Benchmark changes (record rationale; keep algorithm/input/output identical)
- `bench_clang/suite/string_access.lime`: replace `total = total + text.slice(i, i+1).byte_len()`
  with equivalent non-allocating access:
  `let int: b = text.byte(i); if (b >= 0) { total = total + 1; }` (sum of in-range bytes =
  number of chars; output unchanged = 5000). Keep the outer `text = text + "x"` loop as-is
  (so OPT-002 is also exercised).
- `bench_clang/suite/mixed_workload.lime`: replace the inner `text.slice(i, i+1).byte_len()`
  word-length accumulation with `let int: b = text.byte(i); if (b >= 0) { total = total + 1; }`.
  Keep tokenize/split/`cur = cur + ch` logic (exercises OPT-002). Output must stay identical
  (total chars = 5000, word-count distinct = 7).
- `bench_clang/suite/string_concat.lime`: NO change (exercises OPT-002 via `s = s + "y"`).
- Record every benchmark change with: change reason, before, after, Lime impact, Clang impact
  (Clang reference `.c` must mirror the same logic change so it stays a fair same-algorithm
  comparison). For string_access.c and mixed_workload.c, apply the equivalent `text[i]` (C char
  array index) change so both sides use non-allocating char access.

## 7. Files likely affected
- `src/codegen/runtime/runtime.c` (concat, slice, new allocator, byte)
- `src/lib.rs` (`RUNTIME_H_SOURCE` additions) — DO NOT change `RUNTIME_C_SOURCE` path/embedding,
  only the embedded header source text if you add declarations there.
- `src/codegen/mod.rs` (declare lines)
- `src/codegen/fn_builder.rs` (`byte` method arm)
- `bench_clang/suite/string_access.lime` + `.c`
- `bench_clang/suite/mixed_workload.lime` + `.c`
- (string_concat.lime/.c unchanged)

## 8. Explicit non-goals
- Do NOT change the `%LimeList` layout, list semantics, or any non-string ABI.
- Do NOT change the `String` value TYPE (`i8*` data pointer) — only the 8-byte header convention.
- Do NOT add refcounting/GC. Keep strings as they are (immutable values; concat may reuse left buffer).
- Do NOT modify correctness of any non-string benchmark.

## 9. Required tests
- `cargo test --workspace` MUST remain 233/233 (no regression).
- `python3 bench_clang/validate.py` — all 16 benchmarks build/run/correctness MATCH.

## 10. Required benchmarks (Before/After)
- Build Lime release: `cargo build --release` (in repo root). Ensure clang+llvm-22.1.8 bin is on
  PATH: `export PATH="$PATH:/c/Users/szzxl/Downloads/clang+llvm-22.1.8-x86_64-pc-windows-msvc/clang+llvm-22.1.8-x86_64-pc-windows-msvc/bin"`.
- Run: `cd bench_clang && python3 run_benchmarks.py` then `python3 aggregate.py`.
- Report Lime median BEFORE (frozen baseline: string_access 84.78 / string_concat 233.38 /
  mixed_workload NOT VERIFIED~365) and AFTER, plus Clang O2/O3 medians, with correctness MATCH.

## 11. Regression requirements
- 233/233 cargo tests pass. All 16 validate.py MATCH. No crash/UB. No change to non-string
  benchmark correctness or (materially) their numbers beyond measurement noise.

## 12. Acceptance criteria
- `str.byte(i)` works and is allocation-free (verify via IR: no `runtime_str_slice` in
  string_access inner loop after change).
- `runtime_str_concat` reuses left buffer when owned (amortized); string_concat Lime median
  moves toward Clang O3 (target <= Clang O3).
- string_access Lime median <= Clang O3 median (target).
- mixed_workload improves materially and stays MATCH.
- No regression in cargo test or other benchmarks' correctness.

## 13. Reporting requirements (OpenCode must report)
- git diff summary, key runtime.c changes, emitted IR snippets (string_access before/after),
  cargo test result, validate.py result, benchmark Before/After table for string_access,
  string_concat, mixed_workload (and note any other benchmark that moved), correctness, and any
  limitation/risk (esp. the concat left-buffer-reuse aliasing contract).
- State explicitly: did string_access and string_concat reach <= Clang O3 median? With measured numbers.
