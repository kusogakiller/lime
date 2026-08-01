# Lime Runtime (Phase 9)

The Lime runtime is a minimal C-ABI helper library linked alongside the LLVM
IR emitted by `src/codegen`. It owns the only pieces of code that cannot be
expressed as plain LLVM IR: heap allocation, string manipulation, and list
(buffer) management.

## Files

| File | Role |
|------|------|
| `src/codegen/runtime/runtime.h` | C declarations of every runtime symbol. |
| `src/codegen/runtime/runtime.c` | Implementations. |
| `src/codegen/runtime.rs` | Rust side: `extern "C"` declarations, `LimeList` repr(C) mirror, and `RUNTIME_C` / `RUNTIME_H` path constants used by a future `cc`/link step. |

## Value conventions

Lime is a single-owner, copy-on-use language with **no GC and no reference
counting** (see `docs/llvm_backend.md` §5.3). Every runtime value is stored as
a flat, fixed-width word so it can live in an SSA register or a list slot:

- `Int`   → `i64`
- `Float` → `double` (bitcast to `i64` when stored in a list)
- `Bool`  → `i1` (zext to `i64` when stored in a list)
- `String`→ `i8*` (NUL-terminated UTF-8; ptrtoint to `i64` in lists)
- `List(T)` → `%LimeList = { i8* data; i64 len; i64 cap }`

`%LimeList` matches the C struct exactly:

```c
typedef struct {
    char *data;   // heap array of `cap` int64_t elements
    int64_t len;
    int64_t cap;
} LimeList;
```

The Rust mirror `codegen::runtime::LimeList` is `#[repr(C)]` with the same
field order (`data` @0, `len` @8, `cap` @16) so the two sides agree on the
ABI. A unit test (`runtime::tests::lime_list_layout_matches_llvm`) guards this.

## Runtime symbols

| Symbol | Signature | Notes |
|--------|-----------|-------|
| `runtime_alloc` | `i8* (i64 size, i64 align)` | `malloc`; aborts on OOM. |
| `runtime_free` | `void (i8*)` | `free`. Phase 1 does not yet insert frees (leaks tolerated). |
| `runtime_panic` | `void (i8* msg)` | prints and `abort()`. |
| `runtime_print` | `void (i8*)` | writes a NUL-terminated string to stdout. |
| `runtime_str_slice` | `i8* (i8* s, i64 start, i64 end)` | substring `[start, end)` (byte offsets). |
| `runtime_str_concat` | `i8* (i8* a, i8* b)` | immutable concatenation. |
| `runtime_str_chars` | `LimeList (i8* s)` | list of UTF-8 codepoints. |
| `runtime_str_bytes` | `LimeList (i8* s)` | list of byte values. |
| `runtime_list_empty` | `LimeList ()` | empty list. |
| `runtime_list_add` | `LimeList (LimeList, i64)` | append (grows x2). |
| `runtime_list_set` | `LimeList (LimeList, i64, i64)` | bounds-checked replace. |

## Memory policy

- Allocations are never freed by the runtime itself in Phase 1. The compiler
  is responsible for emitting `runtime_free` at the last use of a heap value
  (planned for a later phase via linear escape analysis).
- Strings produced by `runtime_str_*` are freshly allocated and owned by the
  caller.
- The runtime is intentionally dependency-free (only libc: `malloc`/`free`/
  `stdio`/`string`).

## Building / testing the runtime

The runtime is plain C99 and compiles standalone:

```sh
cc -std=c99 -c src/codegen/runtime/runtime.c -o /tmp/runtime.o
```

(No C toolchain is required to build the `lime` compiler itself; the runtime
is only needed when producing a native executable from emitted IR.)
