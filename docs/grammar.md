# Lime 仮文法仕様書（EBNF + 型規則）

## 目的

現在の試作（Lexer / Parser / AST / TypeChecker / Interpreter）と整合する
正式な文法を EBNF で固定し、型規則を定義する。
構文は既存仕様・決定事項に従う。

維持事項:
- Easy. Simple. Fast.
- 簡潔構文 / 可読性重視
- Rust 化しない / C++ 化しない / self-this なし / impl なし
- 暗黙型変換禁止（数値→bool 変換も禁止）
- GC なし / コンパイラ自動 Memory 管理
- Match 完全網羅必須 / else 禁止 / Ignore 破棄
- 文字列演算子なし（String API 経由）

---

## 1. 字句規則（Lexer 既存、変更禁止）

| カテゴリ | 例 |
|----------|-----|
| 識別子 | `[A-Za-z_][A-Za-z0-9_]*` |
| キーワード | fn struct interface state match if else return let mut lime await unsafe |
| 整数リテラル | `123` / `0xFF`（後続） |
| 浮動小数 | `1.5` / `.5` |
| 文字列 | `"..."` |
| 演算子 | `+ - * / % == != < > <= >= && || ! = += -=` |
| 区切り | `( ) [ ] { } : , . .. ; ->` |
| インデント | Indent / Dedent / Newline |

---

## 2. プログラム構造（EBNF）

```
program        ::= statement*

statement      ::= fn_decl
                 | struct_decl
                 | interface_decl
                 | state_decl
                 | let_stmt
                 | if_stmt
                 | match_stmt
                 | return_stmt
                 | expr_stmt

fn_decl        ::= "fn" ident "(" param_list? ")" type? ":" block
                 | "lime" ident "(" param_list? ")" type? ":" block

param_list     ::= param ("," param)*
param          ::= type ":" ident

struct_decl    ::= "struct" ident type_params? ":" indent_block
indent_block   ::= Newline Indent (statement | field_decl)* Dedent

field_decl     ::= type ":" ident

interface_decl ::= "interface" ident ":" indent_block
                 （indent_block 内は fn_decl のシグネチャのみ）

struct_decl    ::= "struct" ident type_params? ":" indent_block
                 （明示的な implements は不要。interface の全メソッドを
                   満たす署名で持っていれば自動的に実装とみなす＝暗黙実装）

state_decl     ::= "state" ident type_params? ":" indent_block
                 （indent_block 内は variant 名 + 任意 payload）

let_stmt       ::= "let" "mut"? type ":" ident "=" expr
                 | "let" "mut"? ident "=" expr

if_stmt        ::= "if" expr ":" block ("else" ":" block)?

match_stmt     ::= "match" expr ":" indent_block
                 （各腕: variant_pattern ":" block）
variant_pattern ::= ident ("(" binding_list? ")")?
binding_list   ::= ident ("," ident)* | "Ignore" ("," "Ignore")*

return_stmt    ::= "return" expr?

expr_stmt      ::= expr
```

---

## 3. 式（EBNF）

```
expr           ::= binary_expr

binary_expr    ::= unary_expr (bin_op unary_expr)*
unary_expr     ::= un_op unary_expr | primary
primary        ::= literal
                 | ident
                 | call_expr
                 | method_expr
                 | field_expr
                 | array_expr
                 | "(" expr ")"

call_expr      ::= ident "(" arg_list? ")"
method_expr    ::= primary "." ident "(" arg_list? ")"
field_expr     ::= primary "." ident
array_expr     ::= "[" expr_list? "]"

arg_list       ::= expr ("," expr)*
expr_list      ::= expr ("," expr)*

literal        ::= int_lit | float_lit | str_lit | bool_lit

bin_op         ::= "+" | "-" | "*" | "/" | "%"
                 | "==" | "!=" | "<" | ">" | "<=" | ">="
                 | "and" | "or"
un_op         ::= "-" | "not"
```

注意: 文字列連結は `+` 演算子で許可（両辺 str の場合）。これは既存の `+` の用途拡張であり、新規の文字列専用演算子は追加しない。
数値型同士は算術演算。文字列専用演算子（独自記号等）は存在しない。

---

## 4. 型規則

### 4.1 基本型

| 型名 | 意味 |
|------|------|
| `int` | 符号付き整数 |
| `float` | 浮動小数点 |
| `bool` | 真偽値 |
| `str` | UTF-8 文字列 |
| `byte` | 1 バイト（UTF-8 操作向け） |
| `char` | Unicode 文字単位 |

### 4.2 複合型

| 型構文 | 意味 |
|--------|------|
| `List(T)` | リスト（固定長・可変長を統一。例: `List(int)`） |
| `Map(K, V)` | マップ |
| `Set(T)` | 集合 |
| `Tuple(T1, T2, ...)` | タプル |
| `Option(T)` | Null 安全（`T?` 省略記法可） |
| `State` 派生 | `state Result(T):` で定義 |
| `Struct` 派生 | `struct User:` で定義 |
| `Interface` 派生 | `interface Animal:` で定義 |
| `T*` | Pointer（unsafe のみ） |

注意: 配列・リストは `List(T)` に統一（固定長/可変長の区別は不要または Runtime が判断）。

### 4.3 型変換規則

- **暗黙変換: 全面禁止**。
- **明示変換（関数形式）**:
  - `int(x)`: x が float/str を受容 → int
  - `float(x)`: x が int/str → float
  - `str(x)`: 任意の表示可能値 → str
  - `bool(x)`: **禁止**（数値→bool は不可）
  - bool は比較演算の結果、または専用真偽値表現のみ。
- **`int(float)` の変換規則（固定）**:
  - 小数部分は **0 方向へ切り捨て（truncate）**。
  - 例: `int(2.9) = 2` / `int(-2.9) = -2`
  - Rust の `as i64`（`f64 as i64`）と同セマンティクス。
- 変換失敗時は `Error` State を返す（後続で Error 伝播と合わせ決定）。

### 4.4 String API の型

| メソッド | 戻り型 |
|----------|--------|
| `.bytes()` | `Array(byte)` |
| `.chars()` | `Array(char)` |
| `.slice(a, b)` | `str` |
| `.len()` | `int`（Unicode 文字数） |
| `.byte_len()` | `int`（バイト長） |

### 4.5 Operator Interface（数値型のみ）

| Interface | 対応演算子 |
|-----------|-----------|
| `Add` | `+` |
| `Equal` | `==` `!=` |
| `Compare` | `<` `>` `<=` `>=` |

`Sub` / `Mul` / `Div` は今後追加（順次拡張）。
ユーザーは `fn add(...)` 等を実装するだけで演算子が使用可能。
文字列は対象外（String API 経由）。

命名は初心者にも役割が伝わること（Easy. Simple. Fast.）を優先：
- `Eq` は略語で直感的でないため採用せず `Equal`。
- `Ord` は "Order" の略で意味が伝わりにくいため採用せず `Compare`。

### 4.6 Generic / Constraint

```
type_params    ::= "(" ident ("," ident)* ")"
constraint     ::= ident "where" ident ":" ident ("," ident ":" ident)*
```

例: `fn max(List(T where T: Compare)): T:`

---

## 5. 制御構造の型

### 5.1 if

- 条件式は **必ず括弧で囲む**: `if (cond):`
- 条件式の型は `bool` 必須（暗黙変換なし）。
- then / else ブロックの最後の式が戻り値型に整合すれば OK。

### 5.2 match

- 対象式の型は `State` 派生必須。
- 全 variant を網羅（不足はコンパイルエラー）。
- `else` 禁止。
- 各腕の binding は `Ignore` で破棄可能。

### 5.3 ループ（後続実装）

```
loop_stmt     ::= "for" ident "in" expr ":" block
               | "for" ident "in" range ":" block
               | "while" "(" expr ")" ":" block
range         ::= expr ".." expr
```

---

## 6. 演算子仕様

### 6.1 演算子一覧

算術演算子:
- `+` `-` `*` `/` `%`

比較演算子:
- `==` `!=` `<` `>` `<=` `>=`

論理演算子（単語形式）:
- `and` `or` `not`
- `&&` `||` `!` は使用しない。

### 6.2 条件式の括弧必須

`if` / `while` 等の条件式は必ず括弧で囲む。

例:
```
if (a >= 10 and b != 0):
    ...

while (count < 10):
    ...
```

理由:
- 演算子優先順位への依存を減らす
- 可読性向上
- パーサ実装の単純化
- 初心者向け設計と一致

### 6.3 演算子優先順位（推奨仕様）

高い方から:
1. `not`
2. `*` `/` `%`
3. `+` `-`
4. `<` `>` `<=` `>=`
5. `==` `!=`
6. `and`
7. `or`

ただし括弧による明示指定を推奨する。

---

## 7. Async（決定事項）

一般関数も非同期になり得る。`List()` に統一された Collection 仕様に合わせ、
非同期関数は `lime` キーワードで宣言。

```
async_fn      ::= "lime" ident "(" param_list? ")" type? ":" block
await_expr    ::= "await" call_expr
```

- 通常関数: `fn function():`
- 非同期関数: `lime function():`
- `let data = await request("url")`
- `async` 予約語は使用しない（Lime 独自構文）。
- 通常関数（`fn`）は非同期処理へ参加不可。`await` の使用は `lime` 関数内のみ許可。
- await 可能な呼び出し規則や Runtime 上の詳細な扱いは、後続の Async / Runtime 設計で具体化。

---

## 7. Memory 指定（決定済み）

- `User(heap)` / `User(stack)`
- 明示なし: Escape 解析で自動判定
- ユーザー明示時はコンパイラは尊重

---

## 8. 禁止事項（この仕様書でも維持）

- Rust 化（'a / lifetime 注釈 / borrow checker 公開）
- C++ 化（継承 / template 自由定義 / operator 過剰）
- self / this
- impl ブロック形式
- 暗黙型変換（数値→bool 含む）
- 文字列専用演算子
- match else
- `_` による Ignore（代わりに `Ignore`）

---

## 9. Type Checker との整合

現在の試作 TypeChecker は以下をカバー:
- 基本型リテラル / 変数 / Binary / Call / Struct constructor /
  FieldAccess / MethodCall / State constructor の型検査
- Struct フィールド型検査
- 関数引数・戻り値型検査
- Match 網羅性検査

未実装（後続ステップ）:
- Generic / Option / Interface 適合判定
- Collections リテラル型
- ループ型検査
- Async / await 型
- Pointer / unsafe 型
- 明示変換 `int(x)` 等の組込み型定義

---

## 10. 次ステップ（実装順）

この EBNF + 型規則を基に、試作へ以下の順で実装:
1. 明示変換 API（int/float/str）
2. String API（.len/.byte_len/.chars/.bytes/.slice）
3. Collections（List 統一 + リテラル + 型）
4. ループ（for / while / range）
5. Option（T? / Match）
6. Generic（Result(T) / Box(T) / Constraint）
7. Interface（暗黙実装 + Operator Interface）
8. Async（lime 関数 + await）
9. Memory 解析（Escape / Lifetime / Stack-Heap）
10. LLVM 統合

禁止事項は全段階で維持。
