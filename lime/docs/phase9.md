# Phase 9 — LLVM Backend & Runtime

**状態: 完了（Phase 1 / テキスト IR 段階）**

Phase 9 では、Lime に対して動作する LLVM codegen（コード生成）と C runtime を導入しました。

これにより、過去のフェーズで追加された以下の機能を LLVM 側で扱えるようになりました。

* aggregate（構造型）
* collection（コレクション）
* interface（インターフェース）

実行自体は引き続きインタプリタ主体です。

`codegen` はテキスト形式の `.ll` LLVM IR を生成し、その構造をインタプリタの意味論と比較することで検証しています。

---

# 目標と成果

| 目標                | 結果                                                                              |
| ----------------- | ------------------------------------------------------------------------------- |
| Struct codegen    | ✅ 完了。constructor（`insertvalue`）、フィールドアクセス（`extractvalue`）、メソッド呼び出し（`%S* self`）。 |
| State codegen     | ✅ 完了。タグ付き union struct と、tag を利用した `switch` による `match`。                        |
| List codegen      | ✅ 完了。リテラル生成、`len` / `get` / `add` / `set` を runtime 経由で処理。print 対応。             |
| String codegen    | ✅ 完了。`len` / `slice` / `concat` / `chars` / `bytes`、print 対応。                   |
| Interface codegen | ✅ 完了。vtable global と `%LimeIface` fat pointer による dispatch。                     |
| Runtime layer     | ✅ 完了。`runtime.c/.h`、Rust mirror、ABI テスト。                                        |
| テスト拡張             | ✅ 完了。integration test 7件 + unit test 6件。                                        |
| ドキュメント            | ✅ 完了。`docs/runtime.md`、`docs/codegen.md`、`docs/phase9.md`。                      |

---

# 成果物

## Runtime

配置：

```
src/codegen/runtime/
```

ファイル：

```
runtime.c
runtime.h
```

Rust 側：

```
src/codegen/runtime.rs
```

---

# Codegen 機能

対象：

```
src/codegen/fn_builder.rs
src/codegen/mod.rs
```

追加された主な機能：

---

## `codegen_for`

対応：

* range
* list / array

を LLVM 側で `while` 相当へ変換。

---

## `infer_return_type` / `call_ret_type`

戻り値型を書かない関数に対する型推論。

例：

```lime
fn add(int:a, int:b):
    return a+b
```

のような関数でも、LLVM の関数シグネチャを正しく生成できます。

---

## print 系

追加：

* `emit_print_value`
* `codegen_list_print`
* `codegen_struct_print`

対応：

* 基本型
* list
* struct

の出力。

---

## Interface codegen

追加：

* `emit_vtable_decls`
* `box_to_iface`
* `codegen_interface_method_call`
* `codegen_arg_coerce`

内容：

* vtable の生成
* struct → interface boxing
* vtable 経由のメソッド dispatch
* 引数型変換

---

## List 用 LLVM global

追加：

```llvm
@.str.lbracket
@.str.rbracket
@.str.space
```

list print 用の文字列定数。

---

# Parser / Type Checker 修正

`type_from_str` を修正。

以前：

```
i
f
b
s
```

などの短縮型が `Type::Var` に落ちていた。

そのため：

* print
* LLVM IR 生成
* struct field

などで問題が発生していた。

修正後：

| shorthand | 型     |
| --------- | ----- |
| `i`       | int   |
| `f`       | float |
| `b`       | bool  |
| `s`       | str   |

として正しく concrete type に変換される。

---

# Examples

インタプリタによる検証済み：

```
examples/phase9_demo/
examples/iface_demo/
```

---

# Tests

## Integration test

`tests/integration.rs`

追加テスト：

```
stdlib_string_math
collections_demo
emit_llvm_smoke
phase9_demo
iface_demo
emit_llvm_interface
emit_llvm_phase9_demo
```

合計：

```
7 integration tests
```

---

## Unit test

`main.rs`

対象：

* type_from_str
* optimizer folding
* parser

---

## Runtime ABI test

`runtime.rs`

確認内容：

* C runtime
* Rust mirror

間の layout ABI 一致。

---

# 検証結果

## Example 出力

### phase9_demo

```
3,30,3,4,5,hello world,42
```

---

### iface_demo

```
woof,4,meow,woof,meow
```

---

## Cargo test

実行：

```sh
cargo test --workspace
```

結果：

```
13 passed
```

内訳：

```
6 unit tests
7 integration tests
```

---

## Cargo build

実行：

```sh
cargo build --workspace
```

結果：

成功。

warning のみ：

```
RUNTIME_C
RUNTIME_H
```

の path constant が意図的に未使用。

`#[allow(dead_code)]` で許可済み。

---

# 後続フェーズで対応予定の項目

## 1. runtime_free の自動挿入

現在：

* メモリ解放なし
* leak を許容

予定：

* escape analysis
* linear lifetime

による最後の使用箇所での `runtime_free` 挿入。

---

## 2. LLVM IR の実行可能バイナリ化

現在：

```
.l l
```

生成まで。

予定：

```
LLVM IR
 ↓
assemble
 ↓
link
 ↓
native binary
```

まで対応。

※ system LLVM が必要。

---

## 3. Async (`lime` / `await`) codegen

現在：

* interpreter のみ
* LLVM では未対応

予定：

* async lowering
* coroutine/state machine

---

## 4. Aggregate 内の unsized interface 対応

現在：

interface は fat pointer boxing 経由のみ。

未対応：

```lime
struct Container:
    Animal: value
```

のような aggregate 内への interface 格納。

---

# まとめ

Phase 9 により Lime は、

* Struct
* State
* List
* String
* Interface
* Runtime ABI

を含む主要な高レベル構造を LLVM IR へ落とせる段階になりました。

現在は **「LLVM backend の意味検証段階（textual IR stage）」** であり、次の大きなステップは **生成した IR を実際のネイティブバイナリへ変換する段階**です。
