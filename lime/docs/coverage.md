# Lime 適用領域カバー一覧（Step 1）

## 目的

Rust / Go がカバーする「作れるソフトウェアの範囲」に Lime も到達するため、
必要な能力・標準ライブラリ・ランタイム・ツールチェーンを洗い出す。

この文書は **構文を決めるものではない**。
「何ができる必要があるか」の能力整理であり、構文は後続ステップで決定する。

既存の Lime 理念・仕様を維持する:
- Easy. Simple. Fast.
- 簡潔な構文 / 可読性重視
- Rust 化しない（'a / lifetime 注釈 / borrow checker をユーザーに要求しない）
- C++ 化しない（継承 / template / operator 過剰）
- self / this なし、impl ブロック形式なし
- 暗黙型変換禁止
- GC なし、コンパイラ自動 Memory 管理

---

## 0. 現状の Lime がカバーできる範囲（試作ベース）

- 変数（let / let mut）
- 関数（引数型: 名前、戻り値型後置）
- Struct（フィールド + メソッド、self/this なし）
- State + Match（完全網羅必須、else 禁止、Ignore 破棄）
- 基本型: int / float / bool / str
- StringBuilder
- Type Checker（実行前型検査）
- Interpreter 実行

→ 小規模なスクリプト・CLI 雛形・データ変換程度は可能。
→ Web / 並行 / ファイルシステム / パッケージ分割は不可。

---

## 1. モジュールシステム / パッケージ管理

- **目的**: 1 ファイルを超える中〜大規模ソフトウェアの構成。
- **必要性**: Rust(crate) / Go(package) 相当。無ければ実用的なアプリが書けない。
- **優先度**: 高
- **既存仕様への影響**: `lime.toml` の `[imports]` 管理（既決定）。ソース分割はコンパイラが複数 `.lime` を 1 単位に統合。AST への import 構文は不要。TypeChecker は複数ファイルの `Defs` をマージ。
- **実装難易度**: 中（コンパイラ側のファイル統合 + Defs マージ）

---

## 2. 標準ライブラリ（Core）

- **目的**: 実用アプリに必須の組込み機能。
- **必要性**: fs / os / path / json / requests / time / datetime / math / random / string / collections / async / thread / logger が無いと何も作れない。
- **優先度**: 高
- **既存仕様への影響**: `requests` は標準搭載（既決定）。Core + citrus 分離（既決定）。String API（`.bytes()/.chars()/.slice()`）の確定が先決。
- **実装難易度**: 中〜高（Interpreter 試作では組込み関数として暫定実装可能）

---

## 3. 明示型変換 API

- **目的**: 型間の意図的な変換手段。
- **必要性**: 暗黙変換禁止のため、int↔float↔str 等の変換は必須。Rust(`str::from_utf8`) / Go(`strconv`) 相当。
- **優先度**: 高（標準ライブラリ記述に必須）
- **決定事項**:
  - 関数形式: `int(x)` / `float(x)` / `str(x)`（bool 以外の基本型間）
  - **数値 → bool 変換は禁止**（暗黙・明示ともに）:
    - 禁止: `bool(0)` / `bool(1)` / `bool(123)`
  - bool 変換は専用値・明示的な真偽値表現のみ許可（例: 比較演算の結果、または専用構文で真偽を得る）。
  - 目的: 意図しない条件判定の防止、可読性重視・曖昧性排除の理念維持。
- **既存仕様への影響**: 暗黙変換禁止は維持。組込み変換関数は `int(x)` / `float(x)` / `str(x)` のみ（bool は対象外）。TypeChecker で戻り型定義。
- **実装難易度**: 低

---

## 4. String 操作 API

- **目的**: UTF-8 テキストの安全な操作。
- **必要性**: バイト/文字アクセス・スライス・長さが無いとテキスト処理不可。
- **優先度**: 高
- **決定事項**:
  - `.bytes() -> Array(byte)`：バイト列
  - `.chars() -> Array(char)`：文字列（Unicode 文字単位）
  - `.slice(a, b) -> str`：部分文字列
  - `.len() -> int`：**文字数（Unicode 文字単位）**
  - `.byte_len() -> int`：バイト長（低レベル UTF-8 操作向け）
  - `text[0]` 禁止維持（曖昧性回避）
  - Encoding 指定 `str(utf16)` 維持
  - StringBuilder 既存
  - **文字列専用 Operator は不要**。文字列操作は Operator ではなく String API（メソッド）で提供。理由: 演算子増加による複雑化回避・可読性維持・特殊構文を増やさない。
- **既存仕様への影響**: TypeChecker で戻り型 `Array(byte)` / `Array(char)` / `str` を定義。Operator Interface（`Add` 等）は数値型のみ対象とし、文字列結合は API 経由。
- **実装難易度**: 低〜中

---

## 5. Collections

- **目的**: 汎用データ構造。
- **必要性**: リスト・マップ・集合・タプルが無いと実用的な処理不可。Rust(vector/map) / Go(slice/map) 相当。
- **優先度**: 高
- **決定事項**: 配列・リストは **`List(T)` に統一**（固定長/可変長の区別は Runtime が判断）。`Map(K,V)` / `Set(T)` / `Tuple(...)` も併存。
- **既存仕様への影響**: Generic `List(T)` 採用（既決定）。TypeChecker に `List(Type)` / `Map(K,V)` / `Set(T)` / `Tuple(...)` 追加。角括弧リテラルは `List` リテラルとして扱う。
- **実装難易度**: 中

---

## 6. Generic（型パラメータ）

- **目的**: 再利用可能な抽象（Result(T) / List(T) / Option(T) 等）。
- **必要性**: 抽象なしでは標準ライブラリも書けない。Rust / Go generics 相当。
- **優先度**: 高
- **既存仕様への影響**: `state Result(T):` / `struct Box(T):` / `fn max(List(T where T: Comparable)):`（既決定）。Parser / AST / TypeChecker の拡張。Constraint は Interface 方式。
- **実装難易度**: 高

---

## 7. Option 型

- **目的**: Null 安全。
- **必要性**: Rust `Option` 相当。Lime は Option 型で確定（既決定）。
- **優先度**: 中〜高
- **既存仕様への影響**: `let User?: user` = 内部 `Option(User)`。State と別物。TypeChecker に `Option(Type)` 追加。Match で処理。
- **実装難易度**: 中

---

## 8. Interface（暗黙実装）

- **目的**: 継承禁止の代替となるポリモーフィズム。
- **必要性**: 引数に `Animal` を渡す等の抽象が必要。Rust trait / Go interface 相当（但し Lime は暗黙実装）。
- **優先度**: 中〜高
- **既存仕様への影響**: `interface Animal: fn speak()`、実装型はメソッドを持つだけで暗黙適合（既決定）。Operator も Interface 方式（`Add` 等、対象は数値型）。文字列は Operator ではなく String API で提供（項目 4 参照）。TypeChecker で構造的適合判定。
- **実装難易度**: 高

---

## 9. ループ構文

- **目的**: 反復処理。
- **必要性**: 現在 if / match のみでは繰り返し不可。Rust / Go の for/loop 相当。
- **優先度**: 高（基本制御構造）
- **既存仕様への影響**: `for x in list:` / `for i in 0..n:` / `while cond:`。Range `..` 演算子既存。Parser / AST / Interpreter / TypeChecker 拡張。
- **実装難易度**: 中

---

## 10. 並行処理（thread / async runtime）

- **目的**: Web サーバ・バックグラウンド処理等の同時実行。
- **必要性**: Go goroutine / Rust async 相当。現代開発で頻出。
- **優先度**: 中
- **決定事項（Async 構文）**:
  - 通常関数: `fn function():`
  - 非同期関数: `lime function():`
  - await: `let data = await request("url")`
  - 例: `fn main():` / `lime main():`
  - 通常関数（`fn`）は非同期処理へ参加不可。`await` の使用は `lime` 関数内のみ許可。
  - await 可能な呼び出し規則・Runtime 詳細は後続の Async / Runtime 設計で具体化。
  - 理由: Lime 独自構文として同期/非同期を明確化。`async` 予約語を増やさず、Rust/JS 風コピーを避ける。
- **既存仕様への影響**: `lime` キーワードで非同期関数を宣言。`await` は既決定。Runtime 未設計（別途決定）。Memory 解析と強連動。
- **実装難易度**: 高（Runtime 設計含む）

---

## 11. unsafe / Pointer / C ABI（FFI）

- **目的**: OS API・C ライブラリ連携・SIMD 等低レベル制御。
- **必要性**: Rust / Go も FFI を持つ。システム領域への到達に必須。
- **優先度**: 中（システム領域向け）
- **既存仕様への影響**: `unsafe:` ブロック、`User*` / `&user`、C ABI（cdecl / repr(C) 互換）。TypeChecker に Pointer 型。Memory 解析と強連動。
- **実装難易度**: 高

---

## 12. エラー伝播構文

- **目的**: Result を返す関数での冗長な記述の削減。
- **必要性**: Match + State で代替可能だが冗長。Go `if err != nil` / Rust `?` 相当。
- **優先度**: 中
- **既存仕様への影響**: 仕様未決（候補: `?` / `raise`）。TypeChecker で伝播型判定。
- **実装難易度**: 中

---

## 13. テストフレームワーク

- **目的**: 品質担保。
- **必要性**: Rust `cargo test` / Go `go test` 相当。
- **優先度**: 中
- **既存仕様への影響**: `lime test` コマンド + テスト注釈。コンパイラ / CLI 機能。
- **実装難易度**: 中

---

## 14. フォーマッタ / ドキュメント生成

- **目的**: 保守性向上。
- **必要性**: Go `gofmt` / `godoc` 相当。
- **優先度**: 低
- **既存仕様への影響**: `lime fmt` / `lime doc`。ツール系。
- **実装難易度**: 低

---

## 15. ランタイム

- **目的**: 実行基盤（非同期スケジューラ・タスク管理・標準ライブラリ基盤）。
- **必要性**: Interpreter 試作はランタイムを持たない。AOT 化・並行・標準ライブラリ稼働に必須。
- **優先度**: 高（並行・標準ライブラリの前提）
- **既存仕様への影響**: Async Runtime / Task API は未設計（別途決定）。Memory 解析と連動。
- **実装難易度**: 高

---

## 16. ツールチェーン（compiler / package manager）

- **目的**: ビルド・実行・依存管理。
- **必要性**: `lime build/run/debug`、`citrus init/add/rem/update/install/pub/search`（既決定）。
- **優先度**: 高（配布・再利用の前提）
- **既存仕様への影響**: コンパイラ名 `lime`、パッケージマネージャ名 `citrus`（責務分離、既決定）。
- **実装難易度**: 中

---

## 優先度サマリ

| 優先度 | 項目 |
|--------|------|
| 高 | 1 モジュール / 2 標準ライブラリ / 3 明示変換 / 4 String API / 5 Collections / 6 Generic / 9 ループ / 15 ランタイム / 16 ツールチェーン |
| 中高 | 7 Option / 8 Interface |
| 中 | 10 並行 / 11 unsafe・FFI / 12 エラー伝播 / 13 テスト |
| 低 | 14 fmt / doc |

---

## 次ステップへの注記

- この文書は「能力整理」であり構文を固定していない。
- 未決定事項（明示変換 / String API / Operator Interface 名 / Async 構文）は決定済み。仮文法仕様書 `grammar.md` を作成済み。
- 実装順（推奨）:
  1. 明示変換 API
  2. String API
  3. Collections（List 統一）
  4. ループ
  5. Option
  6. Generic
  7. Interface
  8. Async
  9. Memory 解析
  10. LLVM
- 全項目において既存禁止事項（Rust 化 / C++ 化 / self-this / impl / 暗黙変換 / 文字列演算子 / match else）を維持。
