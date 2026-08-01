# Lime LLVM Codegen (`src/codegen`)

`src/codegen` は LLVM **バックエンド**です。すでに型付け・メモリ解析済みのプログラムを、テキスト形式の LLVM IR（`emit_llvm` → `--emit-ll`）へ変換します。

Inkwell/`llvm-sys` への依存はなく、システム LLVM も必要ありません。エミッタは `.ll` テキストを生成し、ターゲットレイアウトを確認・記録できるほか、インタプリタとの diff によって正しさを検証できます（実行結果の基準となるオラクルは引き続きインタプリタです）。

## パイプライン

```
run_compilation
  → type_check        （変更なしのインタプリタ型検査）
  → memory_analyze    （変更なし。各 let に対して Stack/Heap を解析）
  → optimize_program  （定数畳み込み、演算子解決）
  → emit_llvm         （テキスト形式 LLVM IR 出力）
```

## モジュール

| モジュール           | 責務                                                                                                                   |
| --------------- | -------------------------------------------------------------------------------------------------------------------- |
| `mod.rs`        | `emit_llvm` のドライバ。ランタイム宣言、文字列グローバル、集約型（struct/state/list/interface）の宣言、vtable グローバル、モノモーフィゼーション、関数出力、`main` ラッパーを担当。 |
| `types.rs`      | `Type` から LLVM 型名への変換（`llvm_type_name`）、アライメント・サイズ・ゼロ値生成ヘルパー。                                                        |
| `fn_builder.rs` | 関数単位の IR 生成。ブロック、`let`（stack/heap）、`if`/`while`/`for`、呼び出し、struct、state、list、string、interface、print などを担当。           |
| `runtime.rs`    | C ランタイム ABI（`docs/runtime.md` 参照）の Rust 側ミラー。                                                                        |
| `runtime/`      | `runtime.c` / `runtime.h`。                                                                                           |

---

## 型 → LLVM マッピング

| Lime 型                 | LLVM IR                                              |
| ---------------------- | ---------------------------------------------------- |
| `Int`（`i` / `int`）     | `i64`                                                |
| `Float`（`f` / `float`） | `double`                                             |
| `Bool`（`b` / `bool`）   | `i1`                                                 |
| `String`（`s` / `str`）  | `i8*`                                                |
| `Struct S`             | `%S`（名前付き struct、フィールド順）                             |
| `State R`              | `%R = { i32 tag; [4 x i64] payload }`（タグ付き union）    |
| `List(T)`              | `%LimeList = { i8* data; i64 len; i64 cap }`         |
| `Interface I`          | `%LimeIface = { i8* data; i8* vtable }`（fat pointer） |

---

## 構文・機能対応状況

| 機能                                                          | 状態                                                 |
| ----------------------------------------------------------- | -------------------------------------------------- |
| リテラル（int/float/bool/string）                                 | ✅                                                  |
| `let` / `let mut`（stack + heap）                             | ✅                                                  |
| 代入、`return`                                                 | ✅                                                  |
| `if` / `else`、`while`、`for`（range・list）                     | ✅                                                  |
| 二項演算（`+ - * / % == != < > <= >= and or`）、`not`              | ✅                                                  |
| 呼び出し（ユーザー関数 / モノモーフ化された generic / struct ctor / state ctor） | ✅                                                  |
| struct 生成 / フィールドアクセス / メソッド呼び出し                            | ✅（`self` はポインタ `%S*` として渡される）                      |
| state 生成 + `match`（タグ付き union の `switch`）                   | ✅                                                  |
| list リテラル / `len` / `get` / `add` / `set`                   | ✅                                                  |
| string の `len` / `slice` / `concat` / `chars` / `bytes`     | ✅                                                  |
| interface dispatch（vtable fat pointer）                      | ✅                                                  |
| `print` / `println`（int/float/bool/string/list/struct）      | ✅                                                  |
| async（`lime` / `await`）                                     | ✅（`await` は直接同期呼び出しへ変換される。インタプリタの `force_run` と一致） |

---

## 戻り値型推論

明示的な戻り値型を指定していない関数でも、LLVM の関数シグネチャが正しくなるように戻り値型を推論します。

例：

```lime
fn add(i: a, i: b):
    return a + b
```

このような関数では、`infer_return_type` が関数本体を探索し、最初の `return <expr>` を取得します。

推論にはパラメータ環境を考慮した `infer_expr_type` を使用します。

また、呼び出し側では `call_ret_type` を利用することで、

```lime
let x = f()
```

のようなコードで、戻り値が正しく値型として保持され、誤って `void` として扱われることを防ぎます。

---

# Interface ABI

* interface `I` を実装する各 struct `S` は、定数 vtable を生成します。

形式：

```llvm
@vtable_<S>_<I> =
    private constant [N x i8*]
    [
        i8* bitcast(@<S>_<m>) to i8*,
        ...
    ]
```

interface の `methods` 宣言順に、1 メソッドにつき 1 スロットを持ちます。

---

## Struct → Interface boxing

具体的な struct 値は `%LimeIface` fat pointer に box 化されます。

`box_to_iface` の内容：

* `data`

  * `%S*` を `i8*` に bitcast したもの
* `vtable`

  * vtable global を `i8*` に bitcast したもの

boxing は、interface 型が期待される以下の場所で発生します。

* 関数呼び出し
* `let`
* 代入

struct のメソッドは `self` をポインタ（`%S*`）で受け取るため、data pointer はそのまま ABI 互換になります。

---

## Interface dispatch

`codegen_interface_method_call` の処理：

1. `data` と `vtable` を取り出す
2. vtable を `i8**` に bitcast
3. 対応するメソッドスロットへ `gep`
4. `i8*` の関数ポインタを load
5. `(i8* data, args...) -> ret` 形式へ bitcast
6. `data` を第1引数として呼び出す

---

# 既知の制限事項（Phase 1）

* `runtime_free` の自動挿入はまだ未実装
  → 現在はメモリリークを許容。

* async（`lime`）関数は通常の LLVM 関数として出力される。

  * `await` は内部呼び出しを直接同期呼び出しへ変換するだけ。
  * coroutine / state machine 化や async runtime は存在しない。

* interface 値は fat pointer boxing 経由でのみ生成される。

  * aggregate（struct/list など）の内部に unsized interface を格納する機能は未実装。

* 出力された IR はまだアセンブル・リンクされ、実行可能バイナリにはなっていない。

  * 開発環境にシステム LLVM ツールチェーンがないため。
  * 正しさは、インタプリタの出力と期待される意味論を比較することで検証している。
