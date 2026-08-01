# Lime プログラミングガイド

Lime は、ツリーウォーキング型インタプリタとネイティブコード LLVM バックエンドを持つ、小規模な静的型付きプログラミング言語です。

このガイドでは、現在実装されている Lime の仕様を説明します。以下のすべての構文・例は現在のコンパイラで確認済みです。

もし動作しない例を見つけた場合、それはこのドキュメントのバグ、またはコンパイラ側の制限です。報告してください。

このガイドでは、予定されている機能や理想的な仕様ではなく、**現在実装済みの機能のみ**を説明します。

---

# 目次

* インストールと実行
* 基本構文
* 値と型
* 変数
* 関数
* 式と演算子
* 制御フロー
* 文字列
* リスト
* Struct とメソッド
* Interface
* Generics
* Async と await
* コンパイルターゲット
* コンパイラエラー
* 機能互換表

---

# インストールと実行

コンパイラは Rust プログラムです。

Cargo でビルドします。

```sh
cargo build --release
```

これにより `lime` 実行ファイルが生成されます。

Windows の場合：

```
target\release\lime.exe
```

---

## コマンドラインインターフェース

```
lime build <path> [--emit-ll] [--emit-object] [--release]
    バイナリをビルド

lime run <path> [--emit-ll]
    ビルドして実行（プロジェクトでは非推奨）

lime check <path>
    型チェックのみ

lime fmt <file.lime> [--write]
    ソースコードを整形

lime <path> [--emit-ll]
    run の短縮形
```

---

プロジェクトビルドには `citrus` CLI もあります。

これは lime のラッパーです。

```
citrus new <name>
    新しいプロジェクトを作成

citrus build [--release]
    プロジェクトをビルド

citrus run [--release]
    プロジェクトをビルドして実行
```

`lime run <path>` は互換性維持のため残されていますが、`citrus.toml` を使うプロジェクトでは非推奨です。

プロジェクトでは `citrus run` を使用してください。

---

`<path>` は以下のどちらかです。

* 単一の `.lime` ソースファイル
* `citrus.toml` プロジェクトマニフェスト

---

# 単一ファイルプログラム

例：

`hello.lime`

```lime
fn main():
    println("hello, world")
    return
```

インタプリタで実行：

```sh
lime run hello.lime
```

出力：

```
hello, world
```

---

`lime run`（または `lime hello.lime`）は以下を行います。

1. パース
2. 型チェック
3. ツリーウォーキング型インタプリタで実行

`main` 関数がなくてもトップレベル文は実行されます。

例：

```lime
println("no main needed")
```

---

型チェック：

```sh
lime check hello.lime
```

整形：

```sh
lime fmt hello.lime
```

標準出力へ表示：

```sh
lime fmt hello.lime
```

ファイルを書き換え：

```sh
lime fmt hello.lime --write
```

---

# プロジェクト

プロジェクトは `citrus.toml` を持つディレクトリです。

例：

```toml
[package]
name = "my_app"
version = "0.1.0"

[files]
main = "main.lime"
```

---

マニフェストを指定してビルドします。

```sh
lime run path/to/dir/citrus.toml
```

---

# ソースファイルの注意事項

* ソースファイルは **LF 改行**でなければなりません。

  * CRLF では tokenizer が失敗します。

例：

```
Unexpected character
```

* ソースファイルは **ASCII のみ対応**です。

  * コメント内の非 ASCII 文字も tokenizer エラーになります。

* インタプリタ実行には LLVM は不要です。

* ネイティブコンパイル（`--emit-object`）には LLVM 22 の以下が必要です。

```
clang
llvm-as
lld-link
```

または：

```
LIME_LLVM_PREFIX
LLVM_SYS_221_PREFIX
```

で LLVM インストール場所を指定できます。

---

# 基本構文

Lime のブロックは行末の `:` とインデントによって表現されます。

波括弧 `{}` はありません。

例：

```lime
fn double(int: x):
    return x * 2

fn main():
    let y = double(21)
    println(y)
    return
```

---

コメントは `//` から行末までです。

```lime
// comment
println("hi")  // trailing comment
```

---

文は基本的に行末で終了します。

セミコロンは任意ですが意味はありません。

```lime
let a = 1
let b = 2;
```

---

# 値と型

Lime の基本型：

| 型       | 別名         | 説明                   |
| ------- | ---------- | -------------------- |
| `int`   | `i32`, `i` | 64bit 符号付き整数         |
| `long`  | `i64`, `l` | 64bit 整数（`L` suffix） |
| `float` | `f64`, `f` | 64bit 浮動小数           |
| `bool`  | `i1`, `b`  | 真偽値                  |
| `str`   | `s`        | UTF-8 不変文字列          |

---

unit 型は内部的には存在します。

ただし型注釈には書けません。

以下は禁止：

```
void
unit
u
```

戻り値がない場合は Lime が推論します。

---

## リテラル

整数：

```lime
42
-1
```

long：

```lime
42L
```

float：

```lime
3.14
```

※ `.5` は不可。

bool：

```lime
true
false
```

string：

```lime
"hello"
```

`\n` や `\t` などのエスケープ対応。

---

# 複合型

## Option

```lime
Option(T)
```

または：

```lime
T?
```

値：

```lime
Some(value)
None
```

---

## List

```lime
List(T)
```

可変長配列。

---

## その他

* `HashMap(K, V)`
* `HashSet(T)`
* Tuple `(A, B, C)`
* struct
* state / enum
* interface

---

例：

```lime
let Option(int): maybe = Some(10)

let List(int): nums = [1,2,3]

let pair = (1,"one")
```

---

# 変数

`let` で変数を作ります。

デフォルトでは immutable です。

変更可能にするには `mut`。

```lime
let x = 10

let mut total = 0

total = total + x
```

---

型指定：

```lime
let int: x = 10

let str: name = "lime"

let List(int): nums = [1,2,3]
```

---

型推論も可能です。

---

# 関数

関数はトップレベルに宣言します。

戻り値型は自動推論されます。

```lime
fn add(int: a, int: b):
    return a + b
```

---

引数型も省略可能です。

```lime
fn identity(value):
    return value
```

---

戻り値：

```lime
return expr
```

何も返さない場合：

```lime
return
```

---

最後の式は暗黙 return になります。

---

# 再帰

関数は自分自身や他関数を呼び出せます。

宣言順は関係ありません。

例：

```lime
fn fact(int:n):
    if n <= 1:
        return 1
    else:
        return n * fact(n-1)
```

---

# 式と演算子

## 算術

```
+
-
*
/
%
```

整数除算は切り捨て。

```lime
10 / 3
```

結果：

```
3
```

`+` は文字列結合にも使用可能。

---

## 比較

```
==
!=
<
<=
>
>=
```

結果は bool。

---

## 論理演算

単語形式：

```
and
or
not
```

記号形式も使用可能：

```
&&
||
!
```

---

## 型変換

```lime
int(x)

float(x)

str(x)
```

---

## len

文字列の byte 長、または list の長さを返します。

---

# 制御フロー

## if / else

`elif` はありません。

```lime
if x > 0:
    ...
else:
    ...
```

---

## while

```lime
while i < 3:
    println(i)
```

`break` / `continue` はありません。

---

## for

list または range。

```lime
for n in [1,2,3]:
    println(n)
```

range：

```lime
for i in 0..3:
    println(i)
```

`0` 以上 `3` 未満。

---

# match

パターンマッチ。

対応：

* tuple:

  ```
  try (a,b)
  ```

* 全一致:

  ```
  catch:
  ```

* Option:

  ```
  Some(v)
  None
  ```

* state variant:

  ```
  Variant(a,b)
  ```

---

例：

```lime
match Some(5):
    Some(v):
        println(v)
    None:
        println("none")
```

---

# defer

関数終了時に実行する処理。

```lime
defer:
    println("cleanup")
```

複数ある場合は登録順。

---

# Strings

文字列は immutable。

メソッドは新しい値を返します。

例：

```lime
text.length()

text.to_upper()

text.slice(0,5)
```

---

利用可能メソッド：

```
len
byte_len
length
chars
bytes
slice
to_upper
to_lower
repeat
read
write
exists
remove
append
metadata
```

---

# Lists

リスト：

```lime
let xs = [1,2,3]
```

空の場合：

```lime
let List(int): xs = []
```

---

主なメソッド：

```
push
pop
first
last
contains
index_of
reverse
length
```

---

# Struct

例：

```lime
struct Point:
    int:x
    int:y

    fn magnitude():
        return x*x+y*y
```

生成：

```lime
Point(3,4)
```

アクセス：

```lime
p.x
p.magnitude()
```

---

# Interface

interface はメソッド集合です。

明示的な `implements` は不要。

一致するメソッドを持つ struct は自動的に適合します。

---

例：

```lime
interface Animal:
    fn speak(str): str:
```

struct 側：

```lime
struct Dog:
    fn speak(str):
        return "woof"
```

---

# Generics

関数：

```lime
fn swap(T,U)(T:a,U:b):
    return (b,a)
```

struct：

```lime
struct Box(T):
    T:value
```

---

# Async / await

`fn` の代わりに `lime` を使います。

```lime
lime double(int:n):
    return n*2
```

呼び出し：

```lime
let result = await double(21)
```

現在の実装では同期実行です。

* 並列処理なし
* coroutine なし
* scheduler なし

LLVM backend でも通常関数として生成されます。

---

# コンパイルターゲット

## LLVM IR 出力

```sh
lime build hello.lime --emit-ll
```

生成：

```
hello.ll
```

LLVM 環境不要。

---

## Object / executable

```sh
lime build hello.lime --emit-object
```

必要：

```
clang
llvm-as
lld-link
LLVM 22
```

---

現在の backend 制限：

* await は同期呼び出し
* long literal 未対応
* state / enum 未対応
* Some / None 未対応
* scalar let の LLVM store に問題あり
* top-level statement は native code では無視

---

# コンパイラエラー

4種類あります。

## Lexer error

文字解析失敗。

例：

```
Lexer error: Unexpected character '#'
```

---

## Parser error

構文エラー。

---

## Type error

型エラー。

例：

```
Type error: undefined variable 'e'
did you mean 'b'?
```

---

## Runtime error

実行時エラー。

例：

```
Undefined variable: Nothing
```

---

# 機能互換表（概要）

| 機能            | Interpreter | LLVM Backend |
| ------------- | ----------- | ------------ |
| let           | Yes         | Partial      |
| let mut       | Yes         | Partial      |
| 型注釈           | Yes         | Yes          |
| Option        | Yes         | No           |
| Tuple         | Partial     | No           |
| Function      | Yes         | Yes          |
| Generic       | Yes         | Partial      |
| Arithmetic    | Yes         | Yes          |
| String        | Yes         | Partial      |
| List          | Yes         | Partial      |
| if            | Yes         | Yes          |
| while         | Yes         | Yes          |
| for           | Yes         | Partial      |
| match         | Yes         | Partial      |
| defer         | Yes         | No           |
| struct        | Yes         | Yes          |
| interface     | Yes         | Partial      |
| enum/state    | Yes         | No           |
| collections   | Yes         | No           |
| HashMap       | Yes         | No           |
| StringBuilder | Yes         | No           |
| print/println | Yes         | Yes          |
| async/await   | Yes         | Yes          |

---

Lime の現在の設計では、**インタプリタが完全な意味論の基準（reference implementation）であり、LLVM backend は段階的に対応範囲を広げる構成**になっています。
