# Lime LLVM Backend Design (Step 10)

このドキュメントは Step 10 の LLVM バックエンド設計である。
実装ではなく「設計」を定める。目標: Interpreter 依存から Compiler Backend へ移行し、
`AST → Typed AST → Memory Analysis → LLVM IR` の流れを確立する。

制約（維持）:
- GC なし
- borrow checker / lifetime 構文をユーザーに公開しない
- Runtime 依存は最小限（Lime ランタイム = 小さな C 相当のヘルパのみ）
- 既存の Lexer/Parser/AST/TypeChecker/Generic/Interface/Async/Memory Analysis を流用

---

## 0. 現在の状態と移行方針

現在の `src/main.rs` は単一パス Interpreter:
```
tokenize → parse → collect_defs → (operators) → type_check → memory_analyze → execute
```
実行は `Value`  enum（i64/f64/String/Bool/Array/Struct/State/Option/Future…）を Rust の
ヒープ（`Box`/ `Vec`/`HashMap`）で保持する。

LLVM 移行後:
```
tokenize → parse → collect_defs
         → type_check        (既存: Typed 情報を AST に付与/検証)
         → memory_analyze    (既存: 各 let を Stack/Heap に決定)
         → lower_to_llvm     (新規: LLVM IR 生成)
         → (MC/obj) → 実行 / または ORC JIT
```

原則:
- AST ノードに新フィールドを追加しない。Typed/Memory 情報は別パスで `Defs` に集約
  されたものを参照する（現在の `resolved_operator` のように）。
- Interpreter は削除せず、**Phase ごとに並行稼働**させ、LLVM 出力と Interpreter 出力を
  比較する差分テストで正当性を保証する（Nightly テスト化）。
- 最初は `main` のみをコンパイルし、徐々に機能を広げる。

---

## 1. LLVM IR 生成方式

### 1.1 利用手段
- **Inkwell**（`llvm-sys` ラッパ）を `Cargo.toml` に追加。ターゲットはホストネイティブ
  （`llvm-sys` はシステムの LLVM 共有庫に依存するため、開発環境に LLVM が必要）。
- テスト駆動で進めるため、**最初の数 Phase は ORC JIT で直接実行**し、のちに
  `TargetMachine` 経由でオブジェクト/実行ファイル生成へ拡張する。

### 1.2 モジュール構成
```
src/
  main.rs              (既存: Lexer/Parser/AST/TC/Memory/Interp)
  codegen/
    mod.rs             (CodegenContext, Module/Builder/TargetData 保持)
    types.rs           (Lime Type → LLVM Type マッピング)
    value.rs           (Sized/Unsized 値の表現, 一時値スタック)
    fn_builder.rs      (関数ごとの IR 生成, 基本ブロック管理)
    structs.rs         (struct / state / variant 型レイアウト)
    calls.rs           (call / call_method / builtin → runtime FFI)
    generic.rs         (型引数単体化 monomorphization)
    interface.rs       (vtable / fat-pointer dispatch)
    async_rt.rs        (Future 構造体 + 状態マシン)
    runtime/
      runtime.c/.h     (アロケータ, print, List/String ヘルパ, 例外なし)
      runtime.rs       (extern "C" 宣言, 組込みシンボル)
```

### 1.3 関数モデル
- 各 Lime 関数 → LLVM `Function`。引数は値またはポインタで渡す（§4/§3 参照）。
- 戻り値は ABI により:
  - スカラ（i64/f64/i1）→ レジスタ返し。
  - 集約型（struct/State/Option/List/String/Future）→ 呼び出し側が確保した
    sret 領域へのポインタを第1暗黙引数として渡す（`sret` 規約）。
- 制御フローは `basic block` で表現。`if/while/for/match` はブロック分岐へ。

---

## 2. Typed AST から LLVM への変換責務

現在 `type_check` はエラー検出のみで、型を AST に書き戻さない。LLVM には「型付き情報」
が必要であるため、軽量な **Typed AST（T-IR）** の中間層を導入する。

### 2.1 Typed AST の形（最小追加）
新しい列挙を `codegen` 専用に用意（既存 `Expr` を壊さない）:
```
TypedExpr = TInt(i64) | TFloat(f64) | TString(&str) | TBool(bool)
          | TVar(String, Type)                      // 変数参照 + その型
          | TBinOp(Box, op, Box, ResolvedOperator)  // 既存 resolved_operator を流用
          | TCall { func, args: Vec<TypedExpr>, ret: Type }
          | TMethodCall { .. , ret: Type }
          | TFieldAccess { .. , field_ty: Type }
          | TAwait(Box, ret: Type)
          | ...
```
- `type_check` の最後に `lower_to_typed(stmts, defs) -> Vec<TypedStmt>` を走らせる
  （既存 `infer_type` / `check_expr` の成果を再利用）。
- `Memory Analysis` 結果（`let name -> Stack/Heap`）は `Defs` または別マップ
  `memory: HashMap<(fn, var), MemoryPlace>` に保持し、codegen が参照。

### 2.2 変換責務の分割
| 責務 | 層 |
|------|----|
| 式の型決定 | TypeChecker（既存） |
| 配置決定 (Stack/Heap) | Memory Analysis（既存） |
| 型→LLVM型 | `types.rs` |
| 式→IR | `fn_builder.rs`（`visit_expr`） |
| 文→IR | `fn_builder.rs`（`visit_stmt`） |
| 宣言→IR | `structs.rs` / `calls.rs` |
| 単体化 | `generic.rs` |
| 多相 dispatch | `interface.rs` |
| 非同期 | `async_rt.rs` |

---

## 3. Stack/Heap Memory 情報の LLVM 反映

Memory Analysis の出力（各 `let` が Stack か Heap か）をそのまま allocation 戦略へ。

- **Stack**: `alloca` で関数フレームに確保。lifetime は基本ブロックスコープ。
  escape しないためポインタは関数内のみで有効。`alloca` は LLVM が自動で
  レジスタに昇格（mem2reg）するため、事実上スタックでもレジスタでも OK。
- **Heap**: `runtime_alloc(size, align)`（§9 FFI）を呼び、返った `i8*` を
  適切な構造体ポインタ型に `bitcast` して使用。
- **明示 `heap`**: 常に `runtime_alloc`。
- **明示 `stack` だが escape**: Memory Analysis でコンパイルエラー済（Step 9）。
  よって codegen 到達時点では「stack で escape する」ケースは存在しない。
- **async 内で await 以降に使用される値**: Memory Analysis で Heap 決定済。
  Future frame（§8）のヒープ領域に配置。

LLVM 上の「所有」概念は存在しない。単に「値の生存期間に応じ alloca or malloc」
を選ぶだけ。ユーザーには一切見せない（＝設計通り）。

> 注意: GC/RC なし。Compiler 内部では単一所有モデルを利用する。
> 値型はコピー、String/List などのヒープ型は Compiler が内部的に最適な移動・共有方式を
> 選択する。ユーザーには copy/move 概念を公開しない。

---

## 4. Struct 表現

現在の `Value::Struct { name, fields: Vec<(String,Value)> }` はタグ付き動的タプル。
LLVM では各 struct を **名前付き LLVM StructType** としてレイアウトする。

### 4.1 レイアウト
- `struct User { str: name }` →
  `%User = type { i8* }` （`str` は §5 の String = `i8*` 管理）。
- フィールド順は `StructDef.fields` の順。パディングは LLVM が `TargetData` で決定。
- Generic struct `Vec2(T)` → 型引数ごとに **monomorphize**（§6）して
  `%Vec2_i64` / `%Vec2_f64` を生成。

### 4.2 コンストラクタ
`User("Alice")` →
1. `alloca %User`（配置に従い stack/heap）
2. フィールド初期化 GEP+store
3. 値として `%User*` または sret コピー

### 4.3 フィールドアクセス
`u.name` → `getelementptr` でフィールドポインタ → `load`。ポインタ経由のため
値型コピーセマンティクスは `load` のタイミングで発生。

### 4.4 State / Variant
`Result(T,E)` は現在 `State { name, values }`。LLVM では:
- 各 `State` を **タグ付きユニオン** として `{ i32 tag; [N x i8] payload }` または
  `i32` + 最大幅バリアントの構造体で表現。
- `Success(v)` / `Error(e)` → tag 書込 + payload 書込。
- `match` → tag 比較で基本ブロック分岐。網羅性は TypeChecker 済。

---

## 5. List / String Runtime 設計

GC なし・最小 Runtime の核心。

### 5.1 String
- 表現: `i8*` + 長さ の **fat pointer** または `%LimeStr = type { i8*, i64 len }`。
- 不変（immutable）セマンティクス: 連結 `+` は新規バッファ確保（既存 Interpreter と同じ）。
- 生成: `runtime_str_from_utf8(ptr, len)` / 連結 `runtime_str_concat(a,b)`。
- UTF-8 保証はコンパイル時（リテラル）＋ 実行時エントリで検証（エラー時は
  `Result`/`State` 経由、例外なし）。

### 5.2 List
- 現在: `Value::Array(Vec<Value>)`。LLVM では:
  - header: `%LimeList = type { i8* data; i64 len; i64 cap }`（data はヒープ）。
  - 要素 `T` の配列を `runtime_alloc` で確保（要素サイズ×cap）。
  - `List(T)` は monomorphize で `T` を固定。
- Buffer は常に Heap（Memory Analysis で List 値は heap 扱いの方針を採用してもよい；
  あるいは header は stack、buffer は heap と分離。§3 の通り「内部 buffer は heap」）。
- インデクス / `for` イテレーション: GEP + bounds check（失敗は `State` 経由で
  `Error` を返すか、トラップ。仕様上は Result を返す API にする）。

### 5.3 ライフタイム（Runtime 側）
- Lime は単一所有者・コピーセマンティクス。参照カウント（Rc/Arc）は**使わない**。
- 関数 return / スコープ終了で生きている Heap 値をどう解放するか？
  - Phase 1（Step 10 前半）: **解放しない（リーク許容）** でコンパイラを完成させる。
  - Phase 2: 線形スコープ解析（DOM ベースの最後使用位置）で `runtime_free` を
    自動挿入。Escape Analysis 結果を流用し、「最後の使用」後に free を emit。
  - GC / RC は禁止。あくまでコンパイラが静的挿入。

---

## 6. Generic の扱い

現在: `Type::Var(T)` を `type_check` 時に制約解決し、実体は単一化されているわけでは
ない（インタプリタは実行時 `Value` で多相を吸収）。
LLVM では **Monomorphization（型引数単体化）** を採用。

- `fn max<T>(...)` が `T=i64` と `T=f64` で呼ばれるなら、それぞれ
  `@max_i64` / `@max_f64` を生成。
- 手順:
  1. `collect_defs` で generic 関数を「テンプレート」として保持。
  2. 呼び出し site（`TCall`）から実引数の具象型を取得。
  3. 未生成なら `instantiate(fn, [具象型])` を実行:
     - `Type::Var(T)` を具象型へ置換（AST の型注釈・制約を書き換え）。
     - 制約 `T: Compare` は具象型が Interface を満たすか静的検査（既存ロジック流用）。
     - 単体化された `FunctionDef` を codegen する。
  4. 同じ具象型ならキャッシュ（再生成しない）。
- Interface 制約を持つ generic は §7 の vtable へ。

> Generic 専用 Memory 規則は追加しない（Step 9 決定通り）。単体化後の具象型で
> 通常の Memory Analysis を適用する。

---

## 7. Interface Dispatch 方式

現在: `Type::Interface(name, [Type])`、暗黙実装、`resolved_operator` で静的決定。
LLVM では2択:

### 7.1 静的単体化（優先・Phase 1）
- 呼び出し site で receiver の具象型が決まる場合（多くの Lime コードは静的に決まる）、
  `resolved_operator` と同じく**直接その具象メソッドを呼ぶ**（devirtualize）。
- Memory Analysis と同様に「呼ばれる interface メソッドは具象型で決定」する。

### 7.2 vtable / fat-pointer（必要時・Phase 2）
- 具象型が静的に決まらない場合（コレクション格納、引数経由で unknown な場合）のみ
  **fat pointer** を使用: `InterfaceValue = { i8* data; vtable* vp }`。
  - `vtable = { fn ptr, fn ptr, ... }`（各メソッド1エントリ）。
  - メソッド呼び出し: `vp->slot[k](data, args...)`。
- ユーザーには trait/object 構文を見せない。コンパイラ内部の dispatch のみ。
- Operator Interface（`Add`/`Equal`/`Compare`）は既存 `resolved_operator` を LLVM の
  `call` にそのまま落とす（静的決定済）。

---

## 8. Async / Future 表現

現在: `lime` 関数 → `Value::Future{func,args}`、`await` で force-run（Interpreter）。
LLVM では真の非同期ランタイム（ステートマシン）または **簡易同期展開** のいずれか。

### 8.1 Phase 1: 同期展開（シングルスレッド協調）
- `lime` 関数を「Future を返す関数」としてコードゲン:
  - `Future` 構造体 = `{ i32 state; i8* frame; fn* resume }`。
  - `frame` は Heap（Memory Analysis で async 内 escape 値は Heap 決定済）。
- `await e`:
  1. `e` を評価 → `Future f`。
  2. 現在の状態を `frame` に保存し、`f` の `resume()` を実行。
  3. `f` 完了まで **簡易イベントループ / 同期ポーリング**（単一スレッド）で進行。
- 例外機構なし。失敗は `Result(T,E)` / `State` 値として伝播（既存仕様維持）。

### 8.2 Phase 2: 真のステートマシン（LLVM coroutine / 手書き分割）
- `lime` 関数本体を `await` 境界で基本ブロックに分割し、C++20 coroutine 相当の
  ステートマシンを生成（あるいは `llvm.coro.*`  intrinsics を利用）。
- `Future` はヒープ確保された resume 状態保持領域。
- スレッドプール / ランタイム scheduler は §9 の minimal runtime に追加。

### 8.3 設計原則
- `async` 予約語・キーワードは追加しない（`await` のみ）。
- `fn` と `lime` は戻り値型システムを完全共有（既存）。codegen でも同じ戻り値 ABI。
- 非同期専用の特別な型システムは作らない（Runtime 実行モデルのみ）。

---

## 9. FFI / Runtime 設計

最小 Runtime（`runtime.c` + `runtime.rs` extern 宣言）。全て `extern "C"`、
`#[no_mangle]`、C ABI。

| Runtime シンボル | 役割 |
|------------------|------|
| `runtime_alloc(size, align) -> i8*` | Heap 配置（§3） |
| `runtime_free(i8*)` | 線形解放（§5.3 Phase 2） |
| `runtime_str_from_utf8 / concat / len / slice` | String 操作 |
| `runtime_list_new / push / get / len` | List 操作 |
| `runtime_print(i8*, len)` | 標準出力（既存 `print` の背後） |
| `runtime_panic(msg)` | 到達不能/オーバーフロー等（例外なし、abort） |
| `runtime_async_schedule(Future*)` | 非同期スケジューラ（§8） |

- 言語組込み `print/len/StringBuilder/int/float...` はすべて上記 Runtime への
  `call` に lowering する（既存 Interpreter の builtin マッチと 1:1 対応）。
- 浮動小数点 / 整数演算は LLVM IR のネイティブ命令（`add`/`fadd` 等）。
- 演算子の `resolved_operator`（Operator Interface）は、そのまま具象関数の `call` へ。

---

## 10. Phase 分割（段階的実装計画）

### Phase 0 — 基盤（非破壊）
- `Cargo.toml` に `inkwell` 追加。`codegen/mod.rs` で `Context/Module/Builder/
  TargetMachine` を初期化し、空の `main` を生成して実行可能ファイルを吐くところ
  まで確認（Hello-world 相当）。
- `type_check` 後に `lower_to_typed` を通し Typed AST を作る（まずは式のみ）。
- 差分テスト基盤: Interpreter 出力と LLVM 出力を比較。

### Phase 1 — スカラ + 制御流れ
- `int/float/bool/str` リテラル、`let`、代入、`if/else`、`while`、`for`(range)、
  `return`、二項演算（静的決定済のもの）、`print`。
- Memory: 全 `let` を `alloca`（escape は後で heap 化）。まずは Stack のみで通す。
- 目標: 既存 `steptest_*` のスカラ部分を LLVM で実行し Interpreter と一致。

### Phase 2 — Struct / State / Match
- `struct` レイアウト・コンストラクタ・フィールドアクセス（§4）。
- `State`/`Result`/`Option` タグ付きユニオン + `match` 分岐（§4.4）。
- 網羅性は既存 TypeChecker 結果を信頼。

### Phase 3 — Heap + Memory Analysis 反映
- §3 の通り `runtime_alloc`/`free` を emit。escape する値・明示 heap を heap 化。
- §5.3 の線形 free 挿入（最後の使用後に `runtime_free`）。

### Phase 4 — List / String Runtime
- §5 の Runtime 関数を実装し、List/Array/Range イテレーションと String API を
  LLVM へ lowering。

### Phase 5 — Generic (Monomorphization)
- §6 の `instantiate` を実装。`List(T)`/`Result(T,E)`/`Vec2(T)` 等を単体化コード生成。

### Phase 6 — Interface Dispatch
- §7.1 静的 devirtualize を実装。必要なら §7.2 vtable/fat-pointer。

### Phase 7 — Async (同期展開)
- §8.1 の Future 構造体 + 簡易スケジューラで `lime`/`await` をコード生成。
- 目標: `steptest_async.lime` を LLVM で実行し Interpreter と一致。

### Phase 8 — 最適化 + 真の非同期 + 実行ファイル
- `PassManager` で O2 相当最適化。
- §8.2 の coroutine ステートマシン化（任意）。
- `TargetMachine` で `.o` / 実行ファイル生成、`clang`/`cc` で Runtime とリンク。

---

## 11. リスクと決定保留事項

- **LLVM バージョン依存**: `inkwell`/`llvm-sys` はシステム LLVM に依存。
  CI に LLVM 導入が必要。代替として `#[cfg]` で Interpreter を fallback に保つ。
- **メモリ解放ポリシー**: Phase 1 はリーク許容。最終は線形 free 挿入（RC/GC なし）。
- **非同期の並行度**: Phase 1 は単一スレッド同期展開。真の並行は Phase 8 以降。
- **Generic のコードバロン**: monomorphization はバイナリを膨らますが、Lime は
  小規模言語のため許容。共有化（generic 関数内で interface 経由）は vtable で抑制。

---

## 12. 次アクション

1. Phase 0 の `Cargo.toml` / `codegen/mod.rs` 雛形を作成し、空 `main` 実行を確認。
2. 差分テスト基盤（Interpreter vs LLVM）を `sandbox/` に追加。
3. Phase 1 から順に実装し、各 Phase で `steptest_*.lime` の一致を確認。
4. 各 Phase 完了ごとに `git commit`。
