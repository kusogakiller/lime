# OpenCode Delegation Brief — Lime Native vs Clang Benchmark Suite

## Role / authority
You are implementing benchmark infrastructure ONLY. You are NOT optimizing the Lime compiler, runtime, or codegen. This is a MEASUREMENT-ONLY task (per Lime development policy §18). Do not propose or make any change outside `C:/Users/szzxl/Downloads/lime/bench_clang/` (a NEW directory we created for this purpose). Do not edit the Lime compiler/runtime/stdlib. If you find a compiler bug, REPORT it in a `NOTES.md` — do not fix it.

## Canonical repository
- ONLY `C:\Users\szzxl\Downloads\lime` (the OUTER repo). Do NOT touch `C:\Users\szzxl\Downloads\lime\lime` or anything under it.
- You are working inside `C:\Users/szzxl/Downloads\lime\bench_clang\` which already exists.

## Hard context you MUST know before writing any benchmark

### Toolchain
- Lime compiler binary: `C:\Users\szzxl\Downloads\lime\target\release\lime.exe` (already built, release).
  - Build native exe: `lime.exe build --release --emit-object <file.lime>`  → produces `<file>.exe` (and `.ll`, `.obj`) NEXT TO the source `.lime` file (same directory), then link with lld-link.
  - `--release` = `-O2` (the compiler runs `clang -O2 -c` on the LLVM IR). Lime Native is therefore pinned at -O2.
  - Type check: `lime.exe check <file.lime>`
- Reference C compiler: `C:\Users\szzxl\Downloads\clang+llvm-22.1.8-x86_64-pc-windows-msvc\clang+llvm-22.1.8-x86_64-pc-windows-msvc\bin\clang.exe` (version 22.1.8).
  - Build: `clang.exe -O2 -o <name>_clang_o2.exe <name>.c` and `clang.exe -O3 -o <name>_clang_o3.exe <name>.c`
- The harness `C:\Users\szzxl\Downloads\lime\bench_clang\run_benchmarks.py` already exists and will BUILD + TIME everything. You only write source files. Do NOT modify the harness unless you find a correctness bug — if you do, report it.

### Lime language surface (verified by us — follow exactly)
- Function: `fn main():` ; return type: `fn foo(int: a): int:` ; params are `type: name` (e.g. `int: n`, NOT `n int`).
- Vars: `let x = 0` (immutable) or `let mut x = 0` (mutable). No type inference needed but you may annotate `let mut int: x = 0`.
- Control: `if (cond):` / `else:` ; NO `elif` keyword — use nested `else:` `if (...):` ; `while (cond):`. `for x in <list>` and `for x in a..b` exist.
- Arithmetic: `+ - * / %` on int and float. No implicit conversions. `int(x)` truncates. `float(x)` converts.
- Strings: concatenation via `+` (both sides str). Methods: `.len()`, `.byte_len()`, `.slice(a,b)`, `.chars()`, `.bytes()`. NO `.substring`. Build up strings by repeated `s = s + x`.
- Collections — use the `collections` stdlib package with FUNCTIONAL (persistent) API:
  - List: `let List(int): xs = []` ; `xs.add(item)` (method, mutates in place — VALIDATED native) ; `xs.get(i)` ; `xs.len()` ; `xs.set(i,v)` ; `collections.push(xs,item)` returns new list ; `collections.length(xs)` ; `collections.reverse(xs)` ; `collections.pop(xs)` ; `collections.contains(xs,item)`.
  - Map (persistent): `let HashMap(int,int): m = collections.make_hash_map()` ; `m = collections.hashmap_insert(m,k,v)` (returns NEW map) ; `collections.hashmap_get(m,k)` ; `collections.hashmap_contains_key(m,k)` ; `collections.hashmap_remove(m,k)` ; `collections.hashmap_len(m)`. IMPORTANT: these are PERSISTENT — the C reference must mirror the same immutable-update semantics (i.e. assign the returned map back) so results match.
  - Set (persistent): `let HashSet(int): s = collections.make_hash_set()` ; `s = collections.hashset_add(s,item)` ; `collections.hashset_contains(s,item)` ; `collections.hashset_remove(s,item)` ; `collections.hashset_len(s)`.
  - For importing the package in a `.lime` file you may need `import collections` at top — CHECK by running `lime.exe check`. If a single-file `import` is unsupported, instead declare the needed `collections.*` functions locally with identical signatures (copy the signatures from `packages/collections/v0.1.0/src/collections.lime`). Prefer the import form if it works.
- Struct: `struct Point:` then fields `int: x` / `int: y`. Construct `Point(1, 2)`. Access `p.x`. Structs are value types.
- Output: `println(value)` prints to stdout with newline. For numeric accumulation, print the final sum/result.
- Do NOT use `lime` (async) functions. Do NOT use `match`/`state` unless required.

### CRITICAL constraints (measurement fairness — policy §3,§9,§10)
1. Lime `.lime` and C `.c` must implement the SAME algorithm and SAME workload size. Same N, same operations. The C side must NOT use SIMD, OpenMP, threading, or hand-unrolled loops that change the algorithm. Clang's own optimizer is the comparison target, not manual C tricks.
2. Workloads must be STACK-SAFE. We confirmed deeply nested loops with large local collection buffers inside while-loops cause a STACK OVERFLOW (0xC00000FD) in the generated exe (the default thread stack is small). Keep loops flat / moderate. If you need big iteration counts, prefer a single-level loop. If you must nest, keep total frames small (e.g. N=10000 x inner 100 is fine; 1000 x 100 x 1000 is NOT).
3. Each benchmark must print a deterministic result so correctness can be checked (Lime output == C output). Make the result independent of timing.
4. Do NOT create a Lime compiler special-case. Benchmark code must look like ordinary user code.
5. Output equality: integer/float results must match between Lime and C. For floats, allow the C reference to print with enough precision; if Lime prints fewer digits, compare within tolerance (document in NOTES.md). Prefer integer-result or checksum-style outputs where possible to avoid float-print drift.

## Deliverables (write into C:\Users\szzxl\Downloads\lime\bench_clang\suite\)

For EACH of the following categories, create `<name>.lime` AND `<name>.c` with identical workload:

A. int_loop (ALREADY DONE — use as template; do not overwrite): add/mul/rem/compare over large loop.
B. float_loop: float accumulation + multiply/divide + a math-heavy inner workload (sin/cos/sqrt if Lime supports `math` package; if not, use multiply/divide accumulation). Print checksum int (round final or scale to int).
C. func_call: many direct small function calls (e.g. 50M calls to a small `add`/`square` helper), nested calls, counted.
D. recursion: normal recursion (e.g. fib or sum-to-n recursive) AND a tail-recursive workload (countdown accumulator) — implement as TWO benchmarks `recursion_tree` and `recursion_tail`.
E. control_flow: while loop with if/elif/else chain (nested conditions) doing arithmetic branches. (Use nested else-if since no elif.)
F. struct_ops: struct creation + field access + copy + pass as arg + return (e.g. Point ops in a loop). Implement `struct_ops`. Also `struct_pass` (pass/return structs).
G. string_ops: string creation, concatenation in loop, comparison, indexing/access, repeated ops, large strings. Implement `string_concat` (loop concat, print final length) and `string_access` (build a string, then index/char access in a loop, checksum).
H. collections_list: List create, push, get, iterate, growth. `list_push` (push N, checksum), `list_iter` (build + iterate summing).
H2. collections_map: Map insert, lookup, remove. `map_ops` (insert N, lookup all, remove half, checksum). Use persistent API mirrored in C.
H3. collections_set: Set insert, lookup. `set_ops` (insert N, contains checks, checksum).
I. memory_ops: repeated allocation / large object creation / collection growth. `alloc_loop` (allocate N structs or N lists in a loop, checksum) — keep stack-safe.
J. mixed_workload: a realistic app-like workload combining arithmetic + functions + strings + collections. `mixed_workload` (e.g. tokenize a string into a list, count frequencies in a map, compute a checksum). Keep it realistic and stack-safe.

Also produce a `micro`/`algo`/`realistic` spread: ensure at least one benchmark per category is a MICRO benchmark, one is an ALGORITHM benchmark (e.g. quicksort or sieve implemented in both Lime and C — `algo_sieve` for prime sieve, `algo_sort` for sort), and `mixed_workload` counts as REALISTIC. Add `algo_sieve` (Sieve of Eratosthenes, count/sum primes) and `algo_sort` (sort N ints via a simple algorithm, checksum) — implement identically in Lime and C.

Minimum total: ~16 benchmark pairs (A,B,C,Dx2,E,Fx2,Gx2,Hx3,I,J, algo_sieve, algo_sort). Quality over quantity; every pair must build+run in BOTH Lime Native and Clang.

## Required behavior from you
1. Write all `.lime` and `.c` files.
2. For EVERY `.lime` file, run `lime.exe check` then `lime.exe build --release --emit-object` and actually RUN the produced `.exe` (it lands next to the .lime in suite/). Confirm it prints a result and does NOT crash (watch for rc=3221225725 stack overflow → reduce N / flatten loops).
3. For EVERY `.c` file, run `clang.exe -O2 -o ...` then run it; confirm output matches the Lime output (within float tolerance for float cases — document tolerance in NOTES.md).
4. If a `.lime` benchmark cannot be made to run natively (compiler limitation), DO NOT fake it. Mark it in `NOTES.md` with the exact error and move on. Categories that cannot be measured must be reported, not silently dropped.
5. Write `C:\Users\szzxl\Downloads\lime\bench_clang\NOTES.md` documenting: any compiler limitations hit, any float-tolerance decisions, any benchmark you could not make run natively, and the workload sizes (N) chosen for each.

## Verification you must NOT skip
- Every `.lime` must `lime build --release --emit-object` successfully AND the exe must run and print.
- Every `.c` must compile with clang -O2 AND run, output equals Lime's.
- Do NOT report "done" on a benchmark you did not actually execute and compare.

## Non-goals (do NOT do)
- Do NOT modify the Lime compiler, runtime, stdlib, or any file outside `bench_clang/`.
- Do NOT optimize anything.
- Do NOT write the final report (we do that).
- Do NOT touch `lime\lime` or git commit anything.

## Hand back
Report: list of benchmark pairs created, which built+ran in both, which matched, and the contents of NOTES.md. Keep it concise and auditable. We will re-run the harness ourselves and verify independently.
