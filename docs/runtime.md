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
- `Map(K,V)` → `%LimeMap` (opaque hash map)
- `Set(T)` → `%LimeSet` (opaque hash set)
- `Json` → `%LimeJson*` (tagged union: null/bool/int/float/string/array/object)
- `Closure` → `%LimeClosure = { i8* fn_ptr; i8* env_ptr }`
- `Interface` → `%LimeIface = { i8* data; i8* vtable }`

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
| `runtime_str_from_i64` | `i8* (i64 v)` | integer to string conversion. |
| `runtime_str_from_f64` | `i8* (double v)` | float to string conversion. |
| `runtime_str_from_bool` | `i8* (i8 v)` | bool to string conversion. |
| `runtime_math_abs` | `double (double)` | |
| `runtime_math_sqrt` | `double (double)` | |
| `runtime_math_min` | `double (double, double)` | |
| `runtime_math_max` | `double (double, double)` | |
| `runtime_math_clamp` | `double (double, double, double)` | |
| `runtime_math_pow` | `double (double, double)` | |
| `runtime_math_floor` | `double (double)` | |
| `runtime_math_ceil` | `double (double)` | |
| `runtime_math_round` | `double (double)` | |
| `runtime_math_trunc` | `double (double)` | |
| `runtime_math_exp` | `double (double)` | |
| `runtime_math_log` | `double (double)` | natural logarithm. |
| `runtime_math_log10` | `double (double)` | base-10 logarithm. |
| `runtime_math_sin` | `double (double)` | |
| `runtime_math_cos` | `double (double)` | |
| `runtime_math_tan` | `double (double)` | |
| `runtime_math_asin` | `double (double)` | |
| `runtime_math_acos` | `double (double)` | |
| `runtime_math_atan` | `double (double)` | |
| `runtime_math_pi` | `double ()` | returns π. |
| `runtime_math_e` | `double ()` | returns e. |
| `runtime_time_now` | `double ()` | epoch seconds. |
| `runtime_time_sleep` | `int (double secs)` | sleeps; returns 1. |
| `runtime_input` | `i8* (i8* prompt)` | writes prompt, reads a line (newline stripped). |
| `runtime_eprint` | `void (i8*)` | writes to stderr. |
| `runtime_eprintln` | `void (i8*)` | writes to stderr with newline. |
| `runtime_read_line` | `i8* ()` | reads a line from stdin. |
| `runtime_read_all` | `i8* ()` | reads all of stdin. |
| `runtime_write_stdout` | `int (i8* s)` | writes to stdout; returns bytes written. |
| `runtime_write_stderr` | `int (i8* s)` | writes to stderr; returns bytes written. |
| `runtime_read_file` | `i8* (i8* path)` | reads a file into a NUL-terminated string. |
| `runtime_write_file` | `int (i8* path, i8* content)` | overwrites; returns success. |
| `runtime_append_file` | `int (i8* path, i8* content)` | creates if missing. |
| `runtime_file_exists` | `int (i8* path)` | |
| `runtime_remove_file` | `int (i8* path)` | |
| `runtime_fs_create_dir` | `int (i8* path)` | creates missing parents. |
| `runtime_fs_size` | `i64 (i8* path)` | byte size of a file or directory. |
| `runtime_fs_metadata` | `void (i8* path, i64* size, i8* is_dir, i8* is_file)` | out-params. |
| `runtime_fs_list_dir` | `LimeList (i8* path)` | full paths of immediate children. |
| `runtime_fs_copy` | `int (i8* src, i8* dst)` | copies a file. |
| `runtime_fs_rename` | `int (i8* src, i8* dst)` | renames a file. |
| `runtime_fs_is_file` | `int (i8* path)` | |
| `runtime_fs_is_dir` | `int (i8* path)` | |
| `runtime_fs_remove_dir` | `int (i8* path)` | |
| `runtime_fs_read_lines` | `LimeList (i8* path)` | reads file as list of lines. |
| `runtime_fs_write_lines` | `int (i8* path, LimeList lines)` | writes list of lines to file. |
| `runtime_list_empty` | `void (LimeList* out)` | initializes an empty list. |
| `runtime_list_add` | `void (LimeList*, i64)` | append (grows x2). |
| `runtime_list_set` | `void (LimeList*, i64, i64)` | bounds-checked replace. |
| `runtime_list_len` | `i64 (LimeList)` | |
| `runtime_list_get` | `i64 (LimeList, i64)` | |
| `runtime_list_insert` | `void (LimeList*, i64, i64)` | insert at index. |
| `runtime_list_clear` | `void (LimeList*)` | |
| `runtime_list_sort` | `void (LimeList*)` | in-place sort. |
| `runtime_list_clone` | `void (LimeList*, LimeList*)` | deep clone. |
| `runtime_map_len` | `i64 (LimeMap)` | |
| `runtime_map_is_empty` | `int (LimeMap)` | |
| `runtime_map_insert` | `LimeMap (LimeMap, i64, i64)` | insert key-value. |
| `runtime_map_get` | `i64 (LimeMap, i64)` | returns value for key. |
| `runtime_map_remove` | `LimeMap (LimeMap, i64)` | remove by key. |
| `runtime_map_contains_key` | `int (LimeMap, i64)` | |
| `runtime_map_clear` | `LimeMap (LimeMap)` | |
| `runtime_map_clone` | `LimeMap (LimeMap)` | |
| `runtime_set_len` | `i64 (LimeSet)` | |
| `runtime_set_is_empty` | `int (LimeSet)` | |
| `runtime_set_add` | `LimeSet (LimeSet, i64)` | |
| `runtime_set_remove` | `LimeSet (LimeSet, i64)` | |
| `runtime_set_contains` | `int (LimeSet, i64)` | |
| `runtime_set_clear` | `LimeSet (LimeSet)` | |
| `runtime_set_clone` | `LimeSet (LimeSet)` | |
| `runtime_queue_push` | `LimeList (LimeList, i64)` | enqueue. |
| `runtime_queue_pop` | `i64 (LimeList)` | dequeue. |
| `runtime_queue_front` | `i64 (LimeList)` | peek front. |
| `runtime_queue_back` | `i64 (LimeList)` | peek back. |
| `runtime_queue_len` | `i64 (LimeList)` | |
| `runtime_queue_is_empty` | `int (LimeList)` | |
| `runtime_queue_clear` | `LimeList (LimeList)` | |
| `runtime_stack_push` | `LimeList (LimeList, i64)` | push. |
| `runtime_stack_pop` | `i64 (LimeList)` | pop. |
| `runtime_stack_peek` | `i64 (LimeList)` | peek top. |
| `runtime_stack_len` | `i64 (LimeList)` | |
| `runtime_stack_is_empty` | `int (LimeList)` | |
| `runtime_stack_clear` | `LimeList (LimeList)` | |
| `runtime_make_closure` | `LimeClosure* (void* fn_ptr, void* env_ptr)` | create closure with captured env. |
| `runtime_call_closure_i64` | `i64 (LimeClosure*, void* packed_args)` | call closure returning int. |
| `runtime_call_closure_ptr` | `void* (LimeClosure*, void* packed_args)` | call closure returning pointer. |
| `runtime_make_fn_ref` | `LimeClosure* (void* fn_ptr)` | create function reference (no capture). |
| `runtime_json_stringify` | `i8* (LimeJson*)` | JSON to string. |
| `runtime_json_parse` | `LimeJson* (i8*)` | string to JSON. |
| `runtime_json_get` | `LimeJson* (LimeJson*, i8* key)` | get field by key. |
| `runtime_json_has` | `i8 (LimeJson*, i8* key)` | check field exists. |
| `runtime_json_len` | `i64 (LimeJson*)` | number of fields/elements. |
| `runtime_json_at` | `LimeJson* (LimeJson*, i64 index)` | get element by index. |
| `runtime_json_as_string` | `i8* (LimeJson*)` | extract string value. |
| `runtime_json_as_int` | `i64 (LimeJson*)` | extract int value. |
| `runtime_json_as_float` | `double (LimeJson*)` | extract float value. |
| `runtime_json_as_bool` | `i8 (LimeJson*)` | extract bool value. |
| `runtime_json_null` | `LimeJson* ()` | create null. |
| `runtime_json_object` | `LimeJson* ()` | create empty object. |
| `runtime_json_array` | `LimeJson* ()` | create empty array. |
| `runtime_json_set` | `i8 (LimeJson*, i8* key, LimeJson* val)` | set field. |
| `runtime_json_push` | `i8 (LimeJson*, LimeJson* elem)` | append to array. |
| `runtime_path_join` | `i8* (i8* a, i8* b)` | join path segments. |
| `runtime_path_basename` | `i8* (i8* path)` | filename without directory. |
| `runtime_path_dirname` | `i8* (i8* path)` | directory part. |
| `runtime_path_filename` | `i8* (i8* path)` | full filename. |
| `runtime_path_extension` | `i8* (i8* path)` | file extension. |
| `runtime_path_is_absolute` | `int (i8* path)` | |
| `runtime_path_normalize` | `i8* (i8* path)` | normalize path separators. |
| `runtime_path_equals` | `int (i8* a, i8* b)` | compare paths. |
| `runtime_path_parent` | `i8* (i8* path)` | parent directory. |
| `runtime_os_name` | `i8* ()` | OS name (e.g., "windows"). |
| `runtime_os_arch` | `i8* ()` | architecture (e.g., "x86_64"). |
| `runtime_os_platform` | `i8* ()` | platform string. |
| `runtime_os_hostname` | `i8* ()` | machine hostname. |
| `runtime_os_cwd` | `i8* ()` | current working directory. |
| `runtime_os_set_cwd` | `int (i8* path)` | change working directory. |
| `runtime_env_get` | `i8* (i8* key)` | get env var (NULL if missing). |
| `runtime_env_has` | `int (i8* key)` | check env var exists. |
| `runtime_env_set` | `int (i8* key, i8* value)` | set env var. |
| `runtime_env_remove` | `int (i8* key)` | remove env var. |
| `runtime_env_all` | `LimeMap ()` | all env vars as map. |
| `runtime_regex_compile` | `i8* (i8* pattern)` | compile regex pattern. |
| `runtime_regex_is_match` | `int (i8* compiled, i8* text)` | test match. |
| `runtime_regex_find` | `i8* (i8* compiled, i8* text)` | first match. |
| `runtime_regex_find_all` | `LimeList (i8* compiled, i8* text)` | all matches. |
| `runtime_regex_replace` | `i8* (i8* compiled, i8* text, i8* replacement)` | replace first. |
| `runtime_regex_replace_all` | `i8* (i8* compiled, i8* text, i8* replacement)` | replace all. |
| `runtime_regex_split` | `LimeList (i8* compiled, i8* text)` | split by pattern. |
| `runtime_process_spawn` | `i64 (i8* command, LimeList args)` | spawn process, return PID. |
| `runtime_process_run` | `i8* (i8* command, LimeList args)` | run and return stdout. |
| `runtime_process_output` | `i8* (i8* command, LimeList args)` | run and capture output. |
| `runtime_process_wait` | `i64 (i64 pid)` | wait for process. |
| `runtime_process_kill` | `int (i64 pid)` | kill process. |
| `runtime_process_status` | `i8* (i64 pid)` | get process status. |
| `runtime_process_args` | `LimeList ()` | command-line arguments. |
| `runtime_requests_*` | (many) | HTTP client functions (see `docs/requests.md`). |

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
