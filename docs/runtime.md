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

## Stdlib builtin helpers (Phase 12 Step 1)

The bundled stdlib packages (`string`/`math`/`time`/`fs`/`io`) wrap the
interpreter runtime builtins. The native backend now lowers those wrappers to
the C helpers below, so a stdlib-using program compiles to a native executable
whose output matches the interpreter.

| Symbol | Signature | Notes |
|--------|-----------|-------|
| `runtime_str_contains` | `int (i8* s, i8* sub)` | substring test. |
| `runtime_str_starts_with` | `int (i8* s, i8* prefix)` | |
| `runtime_str_ends_with` | `int (i8* s, i8* suffix)` | |
| `runtime_str_trim` | `i8* (i8* s)` | ASCII whitespace trim. |
| `runtime_str_replace` | `i8* (i8* s, i8* from, i8* to)` | empty `from` returns a copy. |
| `runtime_str_to_upper` | `i8* (i8* s)` | ASCII case mapping. |
| `runtime_str_to_lower` | `i8* (i8* s)` | |
| `runtime_str_repeat` | `i8* (i8* s, i64 times)` | `times < 0` returns empty. |
| `runtime_str_split` | `LimeList (i8* s, i8* sep)` | list of substrings (boxed as `i64` slots); matches Rust `str::split`. |
| `runtime_math_abs` | `double (double)` | |
| `runtime_math_sqrt` | `double (double)` | |
| `runtime_math_min` | `double (double, double)` | |
| `runtime_math_max` | `double (double, double)` | |
| `runtime_math_clamp` | `double (double, double, double)` | |
| `runtime_math_pow` | `double (double, double)` | |
| `runtime_time_now` | `double ()` | epoch seconds. |
| `runtime_time_sleep` | `int (double secs)` | sleeps; returns 1. |
| `runtime_input` | `i8* (i8* prompt)` | writes prompt, reads a line (newline stripped). |
| `runtime_read_file` | `i8* (i8* path)` | reads a file into a NUL-terminated string. |
| `runtime_write_file` | `int (i8* path, i8* content)` | overwrites; returns success. |
| `runtime_append_file` | `int (i8* path, i8* content)` | creates if missing. |
| `runtime_file_exists` | `int (i8* path)` | |
| `runtime_remove_file` | `int (i8* path)` | |
| `runtime_fs_create_dir` | `int (i8* path)` | creates missing parents. |
| `runtime_fs_size` | `i64 (i8* path)` | byte size of a file or directory. |
| `runtime_fs_metadata` | `void (i8* path, i64* size, i8* is_dir, i8* is_file)` | out-params. |
| `runtime_fs_list_dir` | `LimeList (i8* path)` | full paths of immediate children. |

`string.split` and `fs.list_dir` return `%LimeList` via the MSVC `sret` ABI
(`declare void @runtime_str_split(ptr sret(%LimeList), ptr, ptr)`). Their
string elements are boxed into the list's `i64` slots, so only `Int`-list
operations (`len`) apply natively today.

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

`compile_runtime_c` (in `src/lib.rs`) embeds `runtime.c`/`runtime.h` via
`include_str!`, writes them to the OS temp dir, and invokes clang (`-O2 -c`).
The object file is named by a hash of the embedded source
(`runtime-<hash>.obj`), so editing `runtime.c` never links a stale cached
object. The compiler locates clang/lld-link via the LLVM prefix
(`LIME_LLVM_PREFIX`, `LLVM_SYS_221_PREFIX`, or PATH lookup).
