# Lime 繝励Ο繧ｰ繝ｩ繝溘Φ繧ｰ繧ｬ繧､繝・

Lime 縺ｯ縲√ヤ繝ｪ繝ｼ繧ｦ繧ｩ繝ｼ繧ｭ繝ｳ繧ｰ蝙九う繝ｳ繧ｿ繝励Μ繧ｿ縺ｨ繝阪う繝・ぅ繝悶さ繝ｼ繝・LLVM 繝舌ャ繧ｯ繧ｨ繝ｳ繝峨ｒ謖√▽縲∝ｰ剰ｦ乗ｨ｡縺ｪ髱咏噪蝙倶ｻ倥″繝励Ο繧ｰ繝ｩ繝溘Φ繧ｰ險隱槭〒縺吶・

縺薙・繧ｬ繧､繝峨〒縺ｯ縲∫樟蝨ｨ螳溯｣・＆繧後※縺・ｋ Lime 縺ｮ莉墓ｧ倥ｒ隱ｬ譏弱＠縺ｾ縺吶ゆｻ･荳九・縺吶∋縺ｦ縺ｮ讒区枚繝ｻ萓九・迴ｾ蝨ｨ縺ｮ繧ｳ繝ｳ繝代う繝ｩ縺ｧ遒ｺ隱肴ｸ医∩縺ｧ縺吶・

繧ゅ＠蜍穂ｽ懊＠縺ｪ縺・ｾ九ｒ隕九▽縺代◆蝣ｴ蜷医√◎繧後・縺薙・繝峨く繝･繝｡繝ｳ繝医・繝舌げ縲√∪縺溘・繧ｳ繝ｳ繝代う繝ｩ蛛ｴ縺ｮ蛻ｶ髯舌〒縺吶ょｱ蜻翫＠縺ｦ縺上□縺輔＞縲・

縺薙・繧ｬ繧､繝峨〒縺ｯ縲∽ｺ亥ｮ壹＆繧後※縺・ｋ讖溯・繧・炊諠ｳ逧・↑莉墓ｧ倥〒縺ｯ縺ｪ縺上・*迴ｾ蝨ｨ螳溯｣・ｸ医∩縺ｮ讖溯・縺ｮ縺ｿ**繧定ｪｬ譏弱＠縺ｾ縺吶・

---

# 逶ｮ谺｡

* 繧､繝ｳ繧ｹ繝医・繝ｫ縺ｨ螳溯｡・
* 蝓ｺ譛ｬ讒区枚
* 蛟､縺ｨ蝙・
* 螟画焚
* 髢｢謨ｰ
* 蠑上→貍皮ｮ怜ｭ・
* 蛻ｶ蠕｡繝輔Ο繝ｼ
* 譁・ｭ怜・
* 繝ｪ繧ｹ繝・
* Struct 縺ｨ繝｡繧ｽ繝・ラ
* Interface
* Generics
* Async 縺ｨ await
* 繧ｳ繝ｳ繝代う繝ｫ繧ｿ繝ｼ繧ｲ繝・ヨ
* 繧ｳ繝ｳ繝代う繝ｩ繧ｨ繝ｩ繝ｼ
* 讖溯・莠呈鋤陦ｨ

---

# 繧､繝ｳ繧ｹ繝医・繝ｫ縺ｨ螳溯｡・

繧ｳ繝ｳ繝代う繝ｩ縺ｯ Rust 繝励Ο繧ｰ繝ｩ繝縺ｧ縺吶・

Cargo 縺ｧ繝薙Ν繝峨＠縺ｾ縺吶・

```sh
cargo build --release
```

縺薙ｌ縺ｫ繧医ｊ `lime` 螳溯｡後ヵ繧｡繧､繝ｫ縺檎函謌舌＆繧後∪縺吶・

Windows 縺ｮ蝣ｴ蜷茨ｼ・

```
target\release\lime.exe
```

---

## 繧ｳ繝槭Φ繝峨Λ繧､繝ｳ繧､繝ｳ繧ｿ繝ｼ繝輔ぉ繝ｼ繧ｹ

```
lime build <path> [--emit-ll] [--emit-object] [--release]
    繝舌う繝翫Μ繧偵ン繝ｫ繝・

lime run <path> [--emit-ll]
    繝薙Ν繝峨＠縺ｦ螳溯｡鯉ｼ医・繝ｭ繧ｸ繧ｧ繧ｯ繝医〒縺ｯ髱樊耳螂ｨ・・

lime check <path>
    蝙九メ繧ｧ繝・け縺ｮ縺ｿ

lime fmt <file.lime> [--write]
    繧ｽ繝ｼ繧ｹ繧ｳ繝ｼ繝峨ｒ謨ｴ蠖｢

lime <path> [--emit-ll]
    run 縺ｮ遏ｭ邵ｮ蠖｢
```

---

繝励Ο繧ｸ繧ｧ繧ｯ繝医ン繝ｫ繝峨↓縺ｯ `citrus` CLI 繧ゅ≠繧翫∪縺吶・

縺薙ｌ縺ｯ lime 縺ｮ繝ｩ繝・ヱ繝ｼ縺ｧ縺吶・

```
citrus new <name>
    譁ｰ縺励＞繝励Ο繧ｸ繧ｧ繧ｯ繝医ｒ菴懈・

citrus build [--release]
    繝励Ο繧ｸ繧ｧ繧ｯ繝医ｒ繝薙Ν繝・

citrus run [--release]
    繝励Ο繧ｸ繧ｧ繧ｯ繝医ｒ繝薙Ν繝峨＠縺ｦ螳溯｡・
```

`lime run <path>` 縺ｯ莠呈鋤諤ｧ邯ｭ謖√・縺溘ａ谿九＆繧後※縺・∪縺吶′縲～citrus.toml` 繧剃ｽｿ縺・・繝ｭ繧ｸ繧ｧ繧ｯ繝医〒縺ｯ髱樊耳螂ｨ縺ｧ縺吶・

繝励Ο繧ｸ繧ｧ繧ｯ繝医〒縺ｯ `citrus run` 繧剃ｽｿ逕ｨ縺励※縺上□縺輔＞縲・

---

`<path>` 縺ｯ莉･荳九・縺ｩ縺｡繧峨°縺ｧ縺吶・

* 蜊倅ｸ縺ｮ `.lime` 繧ｽ繝ｼ繧ｹ繝輔ぃ繧､繝ｫ
* `citrus.toml` 繝励Ο繧ｸ繧ｧ繧ｯ繝医・繝九ヵ繧ｧ繧ｹ繝・

---

# 蜊倅ｸ繝輔ぃ繧､繝ｫ繝励Ο繧ｰ繝ｩ繝

萓具ｼ・

`hello.lime`

```lime
fn main():
    println("hello, world")
    return
```

繧､繝ｳ繧ｿ繝励Μ繧ｿ縺ｧ螳溯｡鯉ｼ・

```sh
lime run hello.lime
```

蜃ｺ蜉幢ｼ・

```
hello, world
```

---

`lime run`・医∪縺溘・ `lime hello.lime`・峨・莉･荳九ｒ陦後＞縺ｾ縺吶・

1. 繝代・繧ｹ
2. 蝙九メ繧ｧ繝・け
3. 繝・Μ繝ｼ繧ｦ繧ｩ繝ｼ繧ｭ繝ｳ繧ｰ蝙九う繝ｳ繧ｿ繝励Μ繧ｿ縺ｧ螳溯｡・

`main` 髢｢謨ｰ縺後↑縺上※繧ゅヨ繝・・繝ｬ繝吶Ν譁・・螳溯｡後＆繧後∪縺吶・

萓具ｼ・

```lime
println("no main needed")
```

---

蝙九メ繧ｧ繝・け・・

```sh
lime check hello.lime
```

謨ｴ蠖｢・・

```sh
lime fmt hello.lime
```

讓呎ｺ門・蜉帙∈陦ｨ遉ｺ・・

```sh
lime fmt hello.lime
```

繝輔ぃ繧､繝ｫ繧呈嶌縺肴鋤縺茨ｼ・

```sh
lime fmt hello.lime --write
```

---

# 繝励Ο繧ｸ繧ｧ繧ｯ繝・

繝励Ο繧ｸ繧ｧ繧ｯ繝医・ `citrus.toml` 繧呈戟縺､繝・ぅ繝ｬ繧ｯ繝医Μ縺ｧ縺吶・

萓具ｼ・

```toml
[package]
name = "my_app"
version = "0.1.0"

[files]
main = "main.lime"
```

---

繝槭ル繝輔ぉ繧ｹ繝医ｒ謖・ｮ壹＠縺ｦ繝薙Ν繝峨＠縺ｾ縺吶・

```sh
lime run path/to/dir/citrus.toml
```

---

# 繧ｽ繝ｼ繧ｹ繝輔ぃ繧､繝ｫ縺ｮ豕ｨ諢丈ｺ矩・

* 繧ｽ繝ｼ繧ｹ繝輔ぃ繧､繝ｫ縺ｯ **LF 謾ｹ陦・*縺ｧ縺ｪ縺代ｌ縺ｰ縺ｪ繧翫∪縺帙ｓ縲・

  * CRLF 縺ｧ縺ｯ tokenizer 縺悟､ｱ謨励＠縺ｾ縺吶・

萓具ｼ・

```
Unexpected character
```

* 繧ｽ繝ｼ繧ｹ繝輔ぃ繧､繝ｫ縺ｯ **ASCII 縺ｮ縺ｿ蟇ｾ蠢・*縺ｧ縺吶・

  * 繧ｳ繝｡繝ｳ繝亥・縺ｮ髱・ASCII 譁・ｭ励ｂ tokenizer 繧ｨ繝ｩ繝ｼ縺ｫ縺ｪ繧翫∪縺吶・

* 繧､繝ｳ繧ｿ繝励Μ繧ｿ螳溯｡後↓縺ｯ LLVM 縺ｯ荳崎ｦ√〒縺吶・

* 繝阪う繝・ぅ繝悶さ繝ｳ繝代う繝ｫ・・--emit-object`・峨↓縺ｯ LLVM 22 縺ｮ莉･荳九′蠢・ｦ√〒縺吶・

```
clang
llvm-as
lld-link
```

縺ｾ縺溘・・・

```
LIME_LLVM_PREFIX
LLVM_SYS_221_PREFIX
```

縺ｧ LLVM 繧､繝ｳ繧ｹ繝医・繝ｫ蝣ｴ謇繧呈欠螳壹〒縺阪∪縺吶・

---

# 蝓ｺ譛ｬ讒区枚

Lime 縺ｮ繝悶Ο繝・け縺ｯ陦梧忰縺ｮ `:` 縺ｨ繧､繝ｳ繝・Φ繝医↓繧医▲縺ｦ陦ｨ迴ｾ縺輔ｌ縺ｾ縺吶・

豕｢諡ｬ蠑ｧ `{}` 縺ｯ縺ゅｊ縺ｾ縺帙ｓ縲・

萓具ｼ・

```lime
fn double(int: x):
    return x * 2

fn main():
    let y = double(21)
    println(y)
    return
```

---

繧ｳ繝｡繝ｳ繝医・ `//` 縺九ｉ陦梧忰縺ｾ縺ｧ縺ｧ縺吶・

```lime
// comment
println("hi")  // trailing comment
```

---

譁・・蝓ｺ譛ｬ逧・↓陦梧忰縺ｧ邨ゆｺ・＠縺ｾ縺吶・

繧ｻ繝溘さ繝ｭ繝ｳ縺ｯ莉ｻ諢上〒縺吶′諢丞袖縺ｯ縺ゅｊ縺ｾ縺帙ｓ縲・

```lime
let a = 1
let b = 2;
```

---

# 蛟､縺ｨ蝙・

Lime 縺ｮ蝓ｺ譛ｬ蝙具ｼ・

| 蝙・      | 蛻･蜷・        | 隱ｬ譏・                  |
| ------- | ---------- | -------------------- |
| `int`   | `i32`, `i` | 64bit 隨ｦ蜿ｷ莉倥″謨ｴ謨ｰ         |
| `long`  | `i64`, `l` | 64bit 謨ｴ謨ｰ・・L` suffix・・|
| `float` | `f64`, `f` | 64bit 豬ｮ蜍募ｰ乗焚           |
| `bool`  | `i1`, `b`  | 逵溷⊃蛟､                  |
| `str`   | `s`        | UTF-8 荳榊､画枚蟄怜・          |

---

unit 蝙九・蜀・Κ逧・↓縺ｯ蟄伜惠縺励∪縺吶・

縺溘□縺怜梛豕ｨ驥医↓縺ｯ譖ｸ縺代∪縺帙ｓ縲・

莉･荳九・遖∵ｭ｢・・

```
void
unit
u
```

謌ｻ繧雁､縺後↑縺・ｴ蜷医・ Lime 縺梧耳隲悶＠縺ｾ縺吶・

---

## 繝ｪ繝・Λ繝ｫ

謨ｴ謨ｰ・・

```lime
42
-1
```

long・・

```lime
42L
```

float・・

```lime
3.14
```

窶ｻ `.5` 縺ｯ荳榊庄縲・

bool・・

```lime
true
false
```

string・・

```lime
"hello"
```

`\n` 繧・`\t` 縺ｪ縺ｩ縺ｮ繧ｨ繧ｹ繧ｱ繝ｼ繝怜ｯｾ蠢懊・

---

# 隍・粋蝙・

## Option

```lime
Option(T)
```

縺ｾ縺溘・・・

```lime
T?
```

蛟､・・

```lime
Some(value)
None
```

---

## List

```lime
List(T)
```

蜿ｯ螟蛾聞驟榊・縲・

---

## 縺昴・莉・

* `HashMap(K, V)`
* `HashSet(T)`
* Tuple `(A, B, C)`
* struct
* state / enum
* interface

---

萓具ｼ・

```lime
let Option(int): maybe = Some(10)

let List(int): nums = [1,2,3]

let pair = (1,"one")
```

---

# 螟画焚

`let` 縺ｧ螟画焚繧剃ｽ懊ｊ縺ｾ縺吶・

繝・ヵ繧ｩ繝ｫ繝医〒縺ｯ immutable 縺ｧ縺吶・

螟画峩蜿ｯ閭ｽ縺ｫ縺吶ｋ縺ｫ縺ｯ `mut`縲・

```lime
let x = 10

let mut total = 0

total = total + x
```

---

蝙区欠螳夲ｼ・

```lime
let int: x = 10

let str: name = "lime"

let List(int): nums = [1,2,3]
```

---

蝙区耳隲悶ｂ蜿ｯ閭ｽ縺ｧ縺吶・

---

# 髢｢謨ｰ

髢｢謨ｰ縺ｯ繝医ャ繝励Ξ繝吶Ν縺ｫ螳｣險縺励∪縺吶・

謌ｻ繧雁､蝙九・閾ｪ蜍墓耳隲悶＆繧後∪縺吶・

```lime
fn add(int: a, int: b):
    return a + b
```

---

蠑墓焚蝙九ｂ逵∫払蜿ｯ閭ｽ縺ｧ縺吶・

```lime
fn identity(value):
    return value
```

---

謌ｻ繧雁､・・

```lime
return expr
```

菴輔ｂ霑斐＆縺ｪ縺・ｴ蜷茨ｼ・

```lime
return
```

---

譛蠕後・蠑上・證鈴ｻ・return 縺ｫ縺ｪ繧翫∪縺吶・

---

# 蜀榊ｸｰ

髢｢謨ｰ縺ｯ閾ｪ蛻・・霄ｫ繧・ｻ夜未謨ｰ繧貞他縺ｳ蜃ｺ縺帙∪縺吶・

螳｣險鬆・・髢｢菫ゅ≠繧翫∪縺帙ｓ縲・

萓具ｼ・

```lime
fn fact(int:n):
    if n <= 1:
        return 1
    else:
        return n * fact(n-1)
```

---

# 蠑上→貍皮ｮ怜ｭ・

## 邂苓｡・

```
+
-
*
/
%
```

謨ｴ謨ｰ髯､邂励・蛻・ｊ謐ｨ縺ｦ縲・

```lime
10 / 3
```

邨先棡・・

```
3
```

`+` 縺ｯ譁・ｭ怜・邨仙粋縺ｫ繧ゆｽｿ逕ｨ蜿ｯ閭ｽ縲・

---

## 豈碑ｼ・

```
==
!=
<
<=
>
>=
```

邨先棡縺ｯ bool縲・

---

## 隲也炊貍皮ｮ・

蜊倩ｪ槫ｽ｢蠑擾ｼ・

```
and
or
not
```

險伜捷蠖｢蠑上ｂ菴ｿ逕ｨ蜿ｯ閭ｽ・・

```
&&
||
!
```

---

## 蝙句､画鋤

```lime
int(x)

float(x)

str(x)
```

---

## len

譁・ｭ怜・縺ｮ byte 髟ｷ縲√∪縺溘・ list 縺ｮ髟ｷ縺輔ｒ霑斐＠縺ｾ縺吶・

---

# 蛻ｶ蠕｡繝輔Ο繝ｼ

## if / else

`elif` 縺ｯ縺ゅｊ縺ｾ縺帙ｓ縲・

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

`break` / `continue` 縺ｯ縺ゅｊ縺ｾ縺帙ｓ縲・

---

## for

list 縺ｾ縺溘・ range縲・

```lime
for n in [1,2,3]:
    println(n)
```

range・・

```lime
for i in 0..3:
    println(i)
```

`0` 莉･荳・`3` 譛ｪ貅縲・

---

# match

繝代ち繝ｼ繝ｳ繝槭ャ繝√・

蟇ｾ蠢懶ｼ・

* tuple:

  ```
  try (a,b)
  ```

* 蜈ｨ荳閾ｴ:

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

萓具ｼ・

```lime
match Some(5):
    Some(v):
        println(v)
    None:
        println("none")
```

---

# defer

髢｢謨ｰ邨ゆｺ・凾縺ｫ螳溯｡後☆繧句・逅・・

```lime
defer:
    println("cleanup")
```

隍・焚縺ゅｋ蝣ｴ蜷医・逋ｻ骭ｲ鬆・・

---

# Strings

譁・ｭ怜・縺ｯ immutable縲・

繝｡繧ｽ繝・ラ縺ｯ譁ｰ縺励＞蛟､繧定ｿ斐＠縺ｾ縺吶・

萓具ｼ・

```lime
text.length()

text.to_upper()

text.slice(0,5)
```

---

蛻ｩ逕ｨ蜿ｯ閭ｽ繝｡繧ｽ繝・ラ・・

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

繝ｪ繧ｹ繝茨ｼ・

```lime
let xs = [1,2,3]
```

遨ｺ縺ｮ蝣ｴ蜷茨ｼ・

```lime
let List(int): xs = []
```

---

荳ｻ縺ｪ繝｡繧ｽ繝・ラ・・

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

萓具ｼ・

```lime
struct Point:
    int:x
    int:y

    fn magnitude():
        return x*x+y*y
```

逕滓・・・

```lime
Point(3,4)
```

繧｢繧ｯ繧ｻ繧ｹ・・

```lime
p.x
p.magnitude()
```

---

# Interface

interface 縺ｯ繝｡繧ｽ繝・ラ髮・粋縺ｧ縺吶・

譏守､ｺ逧・↑ `implements` 縺ｯ荳崎ｦ√・

荳閾ｴ縺吶ｋ繝｡繧ｽ繝・ラ繧呈戟縺､ struct 縺ｯ閾ｪ蜍慕噪縺ｫ驕ｩ蜷医＠縺ｾ縺吶・

---

萓具ｼ・

```lime
interface Animal:
    fn speak(str): str:
```

struct 蛛ｴ・・

```lime
struct Dog:
    fn speak(str):
        return "woof"
```

---

# Generics

髢｢謨ｰ・・

```lime
fn swap(T,U)(T:a,U:b):
    return (b,a)
```

struct・・

```lime
struct Box(T):
    T:value
```

---

# Async / await

`fn` 縺ｮ莉｣繧上ｊ縺ｫ `lime` 繧剃ｽｿ縺・∪縺吶・

```lime
lime double(int:n):
    return n*2
```

蜻ｼ縺ｳ蜃ｺ縺暦ｼ・

```lime
let result = await double(21)
```

迴ｾ蝨ｨ縺ｮ螳溯｣・〒縺ｯ蜷梧悄螳溯｡後〒縺吶・

* 荳ｦ蛻怜・逅・↑縺・
* coroutine 縺ｪ縺・
* scheduler 縺ｪ縺・

LLVM backend 縺ｧ繧る壼ｸｸ髢｢謨ｰ縺ｨ縺励※逕滓・縺輔ｌ縺ｾ縺吶・

---

# 繧ｳ繝ｳ繝代う繝ｫ繧ｿ繝ｼ繧ｲ繝・ヨ

## LLVM IR 蜃ｺ蜉・

```sh
lime build hello.lime --emit-ll
```

逕滓・・・

```
hello.ll
```

LLVM 迺ｰ蠅・ｸ崎ｦ√・

---

## Object / executable

```sh
lime build hello.lime --emit-object
```

蠢・ｦ・ｼ・

```
clang
llvm-as
lld-link
LLVM 22
```

---

迴ｾ蝨ｨ縺ｮ backend 蛻ｶ髯撰ｼ・

Errors are printed with error codes, file locations, and source snippets.
There are four main categories:

**Lexer errors** (`error[E0001]`) 窶・the source cannot be tokenized:

```
error[E0001] hello.lime: Invalid integer literal: 999999999999999999999
```

**Parser errors** (`error[E0101]`) 窶・the tokens do not form valid syntax:

```
error[E0101] hello.lime: Expected variable name, got Assign (at line 2, col 5)
```

**Type errors** (`error[E02xx]`) 窶・the program is well-formed but does not
type-check. These include source snippets with caret pointers:

```
error[E0201] hello.lime:2:1
  |
2 | println(xyz)
  | ^
Type error: undefined variable 'xyz'
  = help: did you mean 'x'?
```

and type mismatches, which print the expected and received types:

```
error[E0208] hello.lime:3:1
  |
3 | let y = x + "s"
  | ^
Type error: binary '+' type mismatch

expected:
    int

received:
    str
```

**Runtime errors** (`error[E0601]`) 窶・the interpreter hits a problem while
executing (for example, `Undefined variable: Nothing`).

`lime check` reports whether a file type-checks cleanly:

```
ok: hello.lime type-checks cleanly
* await 縺ｯ蜷梧悄蜻ｼ縺ｳ蜃ｺ縺・
* long literal 譛ｪ蟇ｾ蠢・
* state / enum 譛ｪ蟇ｾ蠢・
* Some / None 譛ｪ蟇ｾ蠢・
* scalar let 縺ｮ LLVM store 縺ｫ蝠城｡後≠繧・
* top-level statement 縺ｯ native code 縺ｧ縺ｯ辟｡隕・

---

# 繧ｳ繝ｳ繝代う繝ｩ繧ｨ繝ｩ繝ｼ

4遞ｮ鬘槭≠繧翫∪縺吶・

## Lexer error

譁・ｭ苓ｧ｣譫仙､ｱ謨励・

萓具ｼ・

```
Lexer error: Unexpected character '#'
```

---

## Parser error

讒区枚繧ｨ繝ｩ繝ｼ縲・

---

## Type error

蝙九お繝ｩ繝ｼ縲・

萓具ｼ・

```
Type error: undefined variable 'e'
did you mean 'b'?
```

---

## Runtime error

螳溯｡梧凾繧ｨ繝ｩ繝ｼ縲・

萓具ｼ・

```
Undefined variable: Nothing
```

---

# 讖溯・莠呈鋤陦ｨ・域ｦりｦ・ｼ・

| 讖溯・            | Interpreter | LLVM Backend |
| ------------- | ----------- | ------------ |
| let           | Yes         | Partial      |
| let mut       | Yes         | Partial      |
| 蝙区ｳｨ驥・          | Yes         | Yes          |
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

Lime 縺ｮ迴ｾ蝨ｨ縺ｮ險ｭ險医〒縺ｯ縲・*繧､繝ｳ繧ｿ繝励Μ繧ｿ縺悟ｮ悟・縺ｪ諢丞袖隲悶・蝓ｺ貅厄ｼ・eference implementation・峨〒縺ゅｊ縲´LVM backend 縺ｯ谿ｵ髫守噪縺ｫ蟇ｾ蠢懃ｯ・峇繧貞ｺ・￡繧区ｧ区・**縺ｫ縺ｪ縺｣縺ｦ縺・∪縺吶・
