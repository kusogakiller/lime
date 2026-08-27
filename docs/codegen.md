# Lime LLVM Codegen (`src/codegen`)

`src/codegen` は LLVM **バックエンド**です。

すでに型付け・メモリ解析が完了したプログラムを、テキスト形式の LLVM IR（`emit_llvm` → `--emit-ll`）へ変換します。

Inkwell / `llvm-sys` への依存はなく、システム LLVM も必要ありません。

エミッタは `.ll` テキストを生成し、以下を可能にします。

* ターゲットレイアウトの確認
* LLVM IR の差分比較
* インタプリタとの意味的な正しさの検証

実行結果の基準（execution oracle）は引き続きインタプリタです。

---

# パイプライン

```
run_compilation
  → type_check        （変更なしのインタプリタ型チェック）
  → memory_analyze    （変更なし。各 let ごとに Stack / Heap を解析）
  → optimize_program  （定数畳み込み、演算子解決）
  → emit_llvm         （テキスト形式 LLVM IR 出力）
```

---

# モジュール

| モジュール           | 責務                                                                                                                                     |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `mod.rs`        | `emit_llvm` のメインドライバ。runtime 宣言、文字列 global、aggregate（struct/state/list/interface）宣言、vtable global、モノモーフィゼーション、関数出力、`main` wrapper を担当。 |
| `types.rs`      | `Type` から LLVM 型名への変換（`llvm_type_name`）、alignment / size / zero 値生成ヘルパー。                                                               |
| `fn_builder.rs` | 関数単位の LLVM IR 生成。block、`let`（stack/heap）、`if` / `while` / `for`、call、struct、state、list、string、interface、print を担当。                     |
| `runtime.rs`    | C runtime ABI の Rust 側 mirror（`docs/runtime.md` 参照）。                                                                                   |
| `runtime/`      | `runtime.c` / `runtime.h`。                                                                                                             |

---

# Lime 型 → LLVM 型対応

| Lime 型                 | LLVM IR                                              |
| ---------------------- | ---------------------------------------------------- |
| `Int`（`i` / `int`）     | `i64`                                                |
| `Float`（`f` / `float`） | `double`                                             |
| `Bool`（`b` / `bool`）   | `i1`                                                 |
| `String`（`s` / `str`）  | `i8*`                                                |
| `Struct S`             | `%S`（名前付き struct、field 順）                            |
| `State R`              | `%R = { i32 tag; [4 x i64] payload }`（tagged union）  |
| `List(T)`              | `%LimeList = { i8* data; i64 len; i64 cap }`         |
| `Interface I`          | `%LimeIface = { i8* data; i8* vtable }`（fat pointer） |

---

# 対応構文・機能

| 機能                                                               | 状態                                                    |
| ---------------------------------------------------------------- | ----------------------------------------------------- |
| リテラル（int / float / bool / string）                                | ✅                                                     |
| `let` / `let mut`（stack + heap）                                  | ✅                                                     |
| 代入、`return`                                                      | ✅                                                     |
| `if` / `else`、`while`、`for`（range + list）                        | ✅                                                     |
| 二項演算（`+ - * / % == != < > <= >= and or`）、`not`                   | ✅                                                     |
| 関数呼び出し（user / monomorphized generic / struct ctor / state ctor）  | ✅                                                     |
| struct constructor / field access / method call                  | ✅（`self` は `%S*` ポインタ渡し）                              |
| state constructor + `match`（tagged union の `switch`）             | ✅                                                     |
| list literal / `len` / `get` / `add` / `set`                     | ✅                                                     |
| string `len` / `slice` / `concat` / `chars` / `bytes`            | ✅                                                     |
| interface dispatch（vtable fat pointer）                           | ✅                                                     |
| `print` / `println`（int / float / bool / string / list / struct） | ✅                                                     |
| async（`lime` / `await`）                                          | ✅（`await` は直接同期呼び出しへ変換。interpreter の `force_run` と一致） |

---

# 戻り値型推論

明示的な戻り値型を持たない関数でも、LLVM の関数シグネチャが正しくなるよう戻り値型を推論します。

例：

```lime
fn add(i: a, i: b):
    return a + b
```

このような関数では：

`infer_return_type`

が関数本体を探索します。

処理：

1. 最初の `return <expr>` を取得
2. `infer_expr_type` で型推論
3. パラメータ環境を考慮して型を決定

---

呼び出し側では：

```rust
call_ret_type
```

を使用します。

これにより：

```lime
let x = f()
```

のようなコードで戻り値が正しく保持されます。

以前のように誤って：

```
void
```

として扱われることを防ぎます。

---

# Interface ABI

## Vtable 生成

interface `I` を実装する struct `S` ごとに定数 vtable を生成します。

形式：

```llvm
@vtable_<S>_<I> =
    private constant [N x i8*]
    [
        i8* bitcast(@<S>_<m>) to i8*,
        ...
    ]
```

特徴：

* `I.methods` の宣言順に配置
* 1 method = 1 slot

---

# Struct → Interface boxing

具体的な struct 値は `%LimeIface` fat pointer に変換されます。

`box_to_iface` の内容：

```
data
 └─ bitcast(%S* → i8*)

vtable
 └─ vtable global を i8* 化
```

boxing が発生する場所：

* 関数呼び出し
* `let`
* 代入

interface 型が要求されているが、struct 値が渡された場合に実行されます。

---

# Interface dispatch

`codegen_interface_method_call` の流れ：

1. `data` と `vtable` を取得
2. vtable を `i8**` に変換
3. method slot へ `gep`
4. `i8*` function pointer を load
5. 関数型へ bitcast

形式：

```
(i8* data, args...) -> ret
```

6. `data` を第1引数として呼び出す

struct method は：

```
self: %S*
```

で受け取るため、data pointer と ABI 的に互換になります。

---

# 既知の制限事項（Phase 1）

## runtime_free 自動挿入なし

現在：

* heap allocation は解放されない
* memory leak を許容

将来的には：

* escape analysis
* lifetime analysis

によって最後の使用箇所へ `runtime_free` を挿入予定。

---

## Async 未実装

現在：

```lime
lime
await
```

は通常の LLVM function として出力されます。

`await` は：

```
async call
 ↓
direct synchronous call
```

へ変換されるだけです。

未対応：

* coroutine
* state machine
* async runtime

---

## Aggregate 内 interface 未対応

現在 interface 値は：

```
struct value
    ↓
fat pointer boxing
```

経由のみ生成されます。

未対応：

```lime
struct Container:
    InterfaceType: value
```

のような aggregate 内への unsized interface 格納。

---

## LLVM IR の実行形式化未対応

現在：

```
Lime
 ↓
LLVM IR (.ll)
```

まで。

未対応：

```
.ll
 ↓
assemble
 ↓
link
 ↓
native executable
```

理由：

開発環境に system LLVM toolchain がないため。

正しさは：

```
interpreter output
        vs
expected semantics
```

を比較して検証しています。

---

# まとめ

`src/codegen` は Lime の LLVM backend として、

* 型付き Lime AST
* メモリ解析結果
* 最適化済み IR

を受け取り、

* LLVM textual IR
* runtime ABI 呼び出し
* vtable dispatch
* aggregate 表現

へ変換する層です。
