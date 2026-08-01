# Lime Runtime (Phase 9)

Lime runtime は、`src/codegen` が出力する LLVM IR と一緒にリンクされる、最小限の C ABI ヘルパーライブラリです。

LLVM IR だけでは表現が難しい以下の処理のみを担当します。

* ヒープメモリ確保
* 文字列操作
* リスト（バッファ）管理

---

# ファイル構成

| ファイル                            | 役割                                                                                                                 |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `src/codegen/runtime/runtime.h` | すべての runtime symbol の C 宣言。                                                                                        |
| `src/codegen/runtime/runtime.c` | runtime の実装。                                                                                                       |
| `src/codegen/runtime.rs`        | Rust 側定義。`extern "C"` 宣言、`LimeList` の `repr(C)` ミラー、将来的な `cc` / link step で使用する `RUNTIME_C` / `RUNTIME_H` パス定数を保持。 |

---

# 値の表現規約

Lime は、

* GC なし
* 参照カウントなし
* single-owner
* copy-on-use

の言語です。

（詳細は `docs/llvm_backend.md` §5.3）

すべての runtime value は、SSA register または list slot に格納できるように、固定幅の flat word として保持されます。

---

## 型マッピング

| Lime 型    | 表現                                                 |
| --------- | -------------------------------------------------- |
| `Int`     | `i64`                                              |
| `Float`   | `double`（list 保存時は `i64` に bitcast）                |
| `Bool`    | `i1`（list 保存時は `i64` に zero extend）                |
| `String`  | `i8*`（NUL 終端 UTF-8、list 保存時は `ptrtoint` で `i64` 化） |
| `List(T)` | `%LimeList = { i8* data; i64 len; i64 cap }`       |

---

# LimeList ABI

LLVM 側の `%LimeList` は C struct と完全一致します。

C 定義：

```c
typedef struct {
    char *data;   // cap 個の int64_t 要素を持つ heap 配列
    int64_t len;
    int64_t cap;
} LimeList;
```

---

Rust 側：

```rust
codegen::runtime::LimeList
```

は：

```rust
#[repr(C)]
```

として定義され、フィールド順も一致します。

配置：

| Field  | Offset |
| ------ | ------ |
| `data` | 0      |
| `len`  | 8      |
| `cap`  | 16     |

---

ABI の一致は unit test：

```
runtime::tests::lime_list_layout_matches_llvm
```

によって保証されます。

---

# Runtime Symbol

| Symbol               | Signature                         | 説明                                    |
| -------------------- | --------------------------------- | ------------------------------------- |
| `runtime_alloc`      | `i8* (i64 size, i64 align)`       | `malloc`。メモリ不足時は abort。               |
| `runtime_free`       | `void (i8*)`                      | `free`。Phase 1 では自動挿入されないため leak を許容。 |
| `runtime_panic`      | `void (i8* msg)`                  | メッセージを表示して `abort()`。                 |
| `runtime_print`      | `void (i8*)`                      | NUL 終端文字列を stdout に出力。                |
| `runtime_str_slice`  | `i8* (i8* s, i64 start, i64 end)` | 部分文字列 `[start,end)` を取得（byte offset）。 |
| `runtime_str_concat` | `i8* (i8* a, i8* b)`              | immutable な文字列結合。                     |
| `runtime_str_chars`  | `LimeList (i8* s)`                | UTF-8 codepoint の list を生成。           |
| `runtime_str_bytes`  | `LimeList (i8* s)`                | byte 値の list を生成。                     |
| `runtime_list_empty` | `LimeList ()`                     | 空 list を生成。                           |
| `runtime_list_add`   | `LimeList (LimeList, i64)`        | append（容量は x2 で拡張）。                   |
| `runtime_list_set`   | `LimeList (LimeList, i64, i64)`   | 範囲チェック付き置換。                           |

---

# メモリ管理方針

## Phase 1 の仕様

* runtime 自身は allocation を解放しません。
* compiler が heap value の最後の使用箇所で `runtime_free` を生成する責任を持ちます。

これは後続フェーズで：

* linear escape analysis
* lifetime analysis

を利用して実装予定です。

---

## String

`runtime_str_*` が生成する文字列：

* 常に新規 allocation
* caller が所有

します。

対象：

```
runtime_str_slice
runtime_str_concat
```

など。

---

## Runtime の依存

runtime は意図的に依存を最小化しています。

必要なのは libc のみ：

```
malloc
free
stdio
string
```

---

# Runtime のビルド / テスト

runtime は通常の C99 コードであり、単独コンパイルできます。

例：

```sh
cc -std=c99 -c src/codegen/runtime/runtime.c -o /tmp/runtime.o
```

---

注意：

Lime コンパイラ本体をビルドするために C コンパイラは必要ありません。

C runtime が必要になるのは、生成された LLVM IR から **ネイティブ実行ファイルを作成するときだけ**です。

---

# まとめ

Phase 9 runtime は Lime の LLVM backend における **最小実行基盤**です。

役割は明確に分離されています。

* LLVM IR
  → 制御フロー、型、計算、構造体操作

* C runtime
  → LLVM IR では扱いづらい低レベル処理

という構成になっています。

現在は GC や自動解放を持たないため、メモリ管理は compiler 側の責務として後続フェーズで拡張されます。
