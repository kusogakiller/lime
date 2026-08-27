# Lime Runtime (Phase 9)

Lime runtime 縺ｯ縲～src/codegen` 縺悟・蜉帙☆繧・LLVM IR 縺ｨ荳邱偵↓繝ｪ繝ｳ繧ｯ縺輔ｌ繧九∵怙蟆城剞縺ｮ C ABI 繝倥Ν繝代・繝ｩ繧､繝悶Λ繝ｪ縺ｧ縺吶・

LLVM IR 縺縺代〒縺ｯ陦ｨ迴ｾ縺碁屮縺励＞莉･荳九・蜃ｦ逅・・縺ｿ繧呈球蠖薙＠縺ｾ縺吶・

* 繝偵・繝励Γ繝｢繝ｪ遒ｺ菫・
* 譁・ｭ怜・謫堺ｽ・
* 繝ｪ繧ｹ繝茨ｼ医ヰ繝・ヵ繧｡・臥ｮ｡逅・

---

# 繝輔ぃ繧､繝ｫ讒区・

| 繝輔ぃ繧､繝ｫ                            | 蠖ｹ蜑ｲ                                                                                                                 |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `src/codegen/runtime/runtime.h` | 縺吶∋縺ｦ縺ｮ runtime symbol 縺ｮ C 螳｣險縲・                                                                                       |
| `src/codegen/runtime/runtime.c` | runtime 縺ｮ螳溯｣・・                                                                                                      |
| `src/codegen/runtime.rs`        | Rust 蛛ｴ螳夂ｾｩ縲Ａextern "C"` 螳｣險縲～LimeList` 縺ｮ `repr(C)` 繝溘Λ繝ｼ縲∝ｰ・擂逧・↑ `cc` / link step 縺ｧ菴ｿ逕ｨ縺吶ｋ `RUNTIME_C` / `RUNTIME_H` 繝代せ螳壽焚繧剃ｿ晄戟縲・|

---

# 蛟､縺ｮ陦ｨ迴ｾ隕冗ｴ・

Lime 縺ｯ縲・

* GC 縺ｪ縺・
* 蜿ら・繧ｫ繧ｦ繝ｳ繝医↑縺・
* single-owner
* copy-on-use

縺ｮ險隱槭〒縺吶・

・郁ｩｳ邏ｰ縺ｯ `docs/llvm_backend.md` ﾂｧ5.3・・

縺吶∋縺ｦ縺ｮ runtime value 縺ｯ縲ヾSA register 縺ｾ縺溘・ list slot 縺ｫ譬ｼ邏阪〒縺阪ｋ繧医≧縺ｫ縲∝崋螳壼ｹ・・ flat word 縺ｨ縺励※菫晄戟縺輔ｌ縺ｾ縺吶・

---

## 蝙九・繝・ヴ繝ｳ繧ｰ

| Lime 蝙・   | 陦ｨ迴ｾ                                                 |
| --------- | -------------------------------------------------- |
| `Int`     | `i64`                                              |
| `Float`   | `double`・・ist 菫晏ｭ俶凾縺ｯ `i64` 縺ｫ bitcast・・               |
| `Bool`    | `i1`・・ist 菫晏ｭ俶凾縺ｯ `i64` 縺ｫ zero extend・・               |
| `String`  | `i8*`・・UL 邨らｫｯ UTF-8縲〕ist 菫晏ｭ俶凾縺ｯ `ptrtoint` 縺ｧ `i64` 蛹厄ｼ・|
| `List(T)` | `%LimeList = { i8* data; i64 len; i64 cap }`       |

---

# LimeList ABI

LLVM 蛛ｴ縺ｮ `%LimeList` 縺ｯ C struct 縺ｨ螳悟・荳閾ｴ縺励∪縺吶・

C 螳夂ｾｩ・・

```c
typedef struct {
    char *data;   // cap 蛟九・ int64_t 隕∫ｴ繧呈戟縺､ heap 驟榊・
    int64_t len;
    int64_t cap;
} LimeList;
```

---

Rust 蛛ｴ・・

```rust
codegen::runtime::LimeList
```

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
縺ｯ・・

```rust
#[repr(C)]
```

縺ｨ縺励※螳夂ｾｩ縺輔ｌ縲√ヵ繧｣繝ｼ繝ｫ繝蛾・ｂ荳閾ｴ縺励∪縺吶・

驟咲ｽｮ・・

| Field  | Offset |
| ------ | ------ |
| `data` | 0      |
| `len`  | 8      |
| `cap`  | 16     |

---

ABI 縺ｮ荳閾ｴ縺ｯ unit test・・

```
runtime::tests::lime_list_layout_matches_llvm
```

縺ｫ繧医▲縺ｦ菫晁ｨｼ縺輔ｌ縺ｾ縺吶・

---

# Runtime Symbol

| Symbol               | Signature                         | 隱ｬ譏・                                   |
| -------------------- | --------------------------------- | ------------------------------------- |
| `runtime_alloc`      | `i8* (i64 size, i64 align)`       | `malloc`縲ゅΓ繝｢繝ｪ荳崎ｶｳ譎ゅ・ abort縲・              |
| `runtime_free`       | `void (i8*)`                      | `free`縲１hase 1 縺ｧ縺ｯ閾ｪ蜍墓諺蜈･縺輔ｌ縺ｪ縺・◆繧・leak 繧定ｨｱ螳ｹ縲・|
| `runtime_panic`      | `void (i8* msg)`                  | 繝｡繝・そ繝ｼ繧ｸ繧定｡ｨ遉ｺ縺励※ `abort()`縲・                |
| `runtime_print`      | `void (i8*)`                      | NUL 邨らｫｯ譁・ｭ怜・繧・stdout 縺ｫ蜃ｺ蜉帙・               |
| `runtime_str_slice`  | `i8* (i8* s, i64 start, i64 end)` | 驛ｨ蛻・枚蟄怜・ `[start,end)` 繧貞叙蠕暦ｼ・yte offset・峨・|
| `runtime_str_concat` | `i8* (i8* a, i8* b)`              | immutable 縺ｪ譁・ｭ怜・邨仙粋縲・                    |
| `runtime_str_chars`  | `LimeList (i8* s)`                | UTF-8 codepoint 縺ｮ list 繧堤函謌舌・          |
| `runtime_str_bytes`  | `LimeList (i8* s)`                | byte 蛟､縺ｮ list 繧堤函謌舌・                    |
| `runtime_list_empty` | `LimeList ()`                     | 遨ｺ list 繧堤函謌舌・                          |
| `runtime_list_add`   | `LimeList (LimeList, i64)`        | append・亥ｮｹ驥上・ x2 縺ｧ諡｡蠑ｵ・峨・                  |
| `runtime_list_set`   | `LimeList (LimeList, i64, i64)`   | 遽・峇繝√ぉ繝・け莉倥″鄂ｮ謠帙・                          |

---

# 繝｡繝｢繝ｪ邂｡逅・婿驥・

## Phase 1 縺ｮ莉墓ｧ・

* runtime 閾ｪ霄ｫ縺ｯ allocation 繧定ｧ｣謾ｾ縺励∪縺帙ｓ縲・
* compiler 縺・heap value 縺ｮ譛蠕後・菴ｿ逕ｨ邂・園縺ｧ `runtime_free` 繧堤函謌舌☆繧玖ｲｬ莉ｻ繧呈戟縺｡縺ｾ縺吶・

縺薙ｌ縺ｯ蠕檎ｶ壹ヵ繧ｧ繝ｼ繧ｺ縺ｧ・・

* linear escape analysis
* lifetime analysis

繧貞茜逕ｨ縺励※螳溯｣・ｺ亥ｮ壹〒縺吶・

---

## String

`runtime_str_*` 縺檎函謌舌☆繧区枚蟄怜・・・

* 蟶ｸ縺ｫ譁ｰ隕・allocation
* caller 縺梧園譛・

縺励∪縺吶・

蟇ｾ雎｡・・

```
runtime_str_slice
runtime_str_concat
```

縺ｪ縺ｩ縲・

---

## Runtime 縺ｮ萓晏ｭ・

runtime 縺ｯ諢丞峙逧・↓萓晏ｭ倥ｒ譛蟆丞喧縺励※縺・∪縺吶・

蠢・ｦ√↑縺ｮ縺ｯ libc 縺ｮ縺ｿ・・

```
malloc
free
stdio
string
```

---

# Runtime 縺ｮ繝薙Ν繝・/ 繝・せ繝・

runtime 縺ｯ騾壼ｸｸ縺ｮ C99 繧ｳ繝ｼ繝峨〒縺ゅｊ縲∝腰迢ｬ繧ｳ繝ｳ繝代う繝ｫ縺ｧ縺阪∪縺吶・

萓具ｼ・

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
---

豕ｨ諢擾ｼ・

Lime 繧ｳ繝ｳ繝代う繝ｩ譛ｬ菴薙ｒ繝薙Ν繝峨☆繧九◆繧√↓ C 繧ｳ繝ｳ繝代う繝ｩ縺ｯ蠢・ｦ√≠繧翫∪縺帙ｓ縲・

C runtime 縺悟ｿ・ｦ√↓縺ｪ繧九・縺ｯ縲∫函謌舌＆繧後◆ LLVM IR 縺九ｉ **繝阪う繝・ぅ繝門ｮ溯｡後ヵ繧｡繧､繝ｫ繧剃ｽ懈・縺吶ｋ縺ｨ縺阪□縺・*縺ｧ縺吶・

---

# 縺ｾ縺ｨ繧・

Phase 9 runtime 縺ｯ Lime 縺ｮ LLVM backend 縺ｫ縺翫￠繧・**譛蟆丞ｮ溯｡悟渕逶､**縺ｧ縺吶・

蠖ｹ蜑ｲ縺ｯ譏守｢ｺ縺ｫ蛻・屬縺輔ｌ縺ｦ縺・∪縺吶・

* LLVM IR
  竊・蛻ｶ蠕｡繝輔Ο繝ｼ縲∝梛縲∬ｨ育ｮ励∵ｧ矩菴捺桃菴・

* C runtime
  竊・LLVM IR 縺ｧ縺ｯ謇ｱ縺・▼繧峨＞菴弱Ξ繝吶Ν蜃ｦ逅・

縺ｨ縺・≧讒区・縺ｫ縺ｪ縺｣縺ｦ縺・∪縺吶・

迴ｾ蝨ｨ縺ｯ GC 繧・・蜍戊ｧ｣謾ｾ繧呈戟縺溘↑縺・◆繧√√Γ繝｢繝ｪ邂｡逅・・ compiler 蛛ｴ縺ｮ雋ｬ蜍吶→縺励※蠕檎ｶ壹ヵ繧ｧ繝ｼ繧ｺ縺ｧ諡｡蠑ｵ縺輔ｌ縺ｾ縺吶・
