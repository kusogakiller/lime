# Lime Programming Guide

Lime is a small, statically-typed programming language with a tree-walking
interpreter and a native-code LLVM backend. This guide describes the language
as it is implemented today: every construct and every example below has been
checked against the current compiler.

If you find an example that does not work, it is a bug in this document (or a
limitation of the compiler) — please report it. Nothing in this guide describes
a planned or idealized feature.

## Table of contents

- [Installation and running](#installation-and-running)
- [Basic syntax](#basic-syntax)
- [Values and types](#values-and-types)
- [Variables](#variables)
- [Functions](#functions)
- [Expressions and operators](#expressions-and-operators)
- [Control flow](#control-flow)
- [Strings](#strings)
- [Lists](#lists)
- [Structs and methods](#structs-and-methods)
- [Interfaces](#interfaces)
- [Generics](#generics)
- [Async and await](#async-and-await)
- [Compilation targets](#compilation-targets)
- [Compiler errors](#compiler-errors)
- [Feature compatibility table](#feature-compatibility-table)

---

## Installation and running

The compiler is a Rust program. Build it with Cargo:

```sh
cargo build --release
```

This produces a `lime` executable (on Windows, `target\release\lime.exe`).

The command-line interface is:

```
lime build <path> [--emit-ll] [--emit-object] [--release]  Build to binary
lime run   <path> [--emit-ll]                              Build and execute (deprecated for projects)
lime check <path>                                          Type-check only
lime fmt   <file.lime> [--write]                           Format source
lime <path> [--emit-ll]                                    Shorthand for `run`
```

For project builds there is also the `citrus` CLI (a wrapper around lime):

```
citrus new <name>        Create a new project
citrus build [--release] Build a project (citrus.toml in the current directory)
citrus run  [--release]  Build and run a project
```

`lime run <path>` still works for compatibility but is deprecated for projects
(`citrus.toml`) — use `citrus run` instead.

`<path>` is either a single `.lime` source file or a `citrus.toml` project
manifest.

### Single-file programs

A file `hello.lime`:

```lime
fn main():
    println("hello, world")
    return
```

Run it with the interpreter:

```sh
lime run hello.lime
# hello, world
```

`lime run` (or the `lime hello.lime` shorthand) parses, type-checks, and
executes the program with the tree-walking interpreter. If the file has no
`main` function, the top-level statements still run:

```lime
println("no main needed")
```

```sh
lime check hello.lime      # type-check only, prints "type-checks cleanly"
lime fmt hello.lime        # prints formatted source to stdout
lime fmt hello.lime --write  # rewrites the file in place
```

### Projects

A project is a directory with a `citrus.toml` manifest that lists the source
files:

```toml
[package]
name = "my_app"
version = "0.1.0"

[files]
main = "main.lime"
```

Build the whole project by pointing the CLI at the manifest file:

```sh
lime run path/to/dir/citrus.toml
```

### Notes on source files

- Source files must use **LF line endings**. A file saved with CRLF endings
  fails to tokenize (`Unexpected character`).
- Source files must be **ASCII**. Non-ASCII bytes (for example in comments)
  also fail to tokenize.
- The interpreter does not need an LLVM installation. Native compilation
  (`--emit-object`) requires `clang`, `llvm-as`, and `lld-link` from LLVM 22
  on your `PATH` (or the `LIME_LLVM_PREFIX` / `LLVM_SYS_221_PREFIX` environment
  variable pointing at the LLVM install directory).

---

## Basic syntax

Blocks are introduced by a colon at the end of a line and an indented body.
The indentation level defines the block; there are no braces.

```lime
fn double(int: x):
    return x * 2

fn main():
    let y = double(21)
    println(y)
    return
```

Comments start with `//` and run to the end of the line.

```lime
// this is a comment
println("hi")  // trailing comments work too
```

Statements end at the end of a line. A semicolon is optional and has no
semantic effect:

```lime
let a = 1
let b = 2;
```

---

## Values and types

Lime has the following primitive types.

| Type | Aliases | Notes |
|------|---------|-------|
| `int` | `i32`, `i` | 64-bit signed integer in the interpreter |
| `long` | `i64`, `l` | 64-bit integer; literals use the `L` suffix |
| `float` | `f64`, `f` | 64-bit float |
| `bool` | `i1`, `b` | `true` / `false` |
| `str` | `s` | string (UTF-8, immutable) |

The unit type exists internally (functions with no return value return unit), but
it is inferred and cannot be written in a type annotation — omit the annotation
and let Lime infer the type. The aliases `void`, `unit`, and `u` are rejected in
annotations for this reason.

Integer literals: `42`, `-1`. Long literals carry an `L` suffix: `42L`.
Float literals need digits on both sides of the dot: `3.14`, but `.5` is not
accepted.

Boolean literals are `true` and `false`. String literals use double quotes:
`"hello"`. Standard escapes such as `\n` and `\t` work inside strings.

Compound types:

- `Option(T)` or the shorthand `T?` — either a value of type `T` or nothing.
  Values are written `Some(value)` or `None`.
- `List(T)` — a growable list of `T` (also called an array).
- `HashMap(K, V)` — a hash map.
- `HashSet(T)` — a hash set.
- Tuples `(A, B, C)` — a fixed-size heterogeneous sequence.
- User-defined types: `struct`, `state`/`enum`, and `interface` (see below).

Examples:

```lime
let Option(int): maybe = Some(10)
let maybe2 = None            // type is Option(unknown)
let List(int): nums = [1, 2, 3]
let pair = (1, "one")
```

Note that a typed annotation for a tuple (`let (int, str): pair = ...`) is not
accepted; tuples are bound without a type annotation. Similarly, `None` alone
has the type `Option(unknown)`, so it cannot be bound to an annotated
`Option(int)` binding (`let int?: m = None` is a type error); bind it without
an annotation instead.

---

## Variables

Use `let` to bind a value to a name. Bindings are immutable by default; add
`mut` to allow reassignment.

```lime
let x = 10
let mut total = 0
total = total + x
```

The type of a binding is normally inferred. You can write the type explicitly
between the `let` and the name, separated by a colon:

```lime
let int: x = 10
let str: name = "lime"
let List(int): nums = [1, 2, 3]
```

Type aliases are accepted in this position (`let i32: x = 10`). The `Option`
shorthand (`T?`) also works as an annotation, but `None` alone has the type
`Option(unknown)`, so a value like `Some(10)` is required to satisfy it.

Destructuring into separate names with `let (a, b) = ...` is parsed but not
type-checked, so it does not actually work today. Bind the whole tuple and
destructure it with `match` instead:

```lime
let pair = (1, "one")
match pair:
    try (a, b):
        println(a)   // 1
        println(b)   // one
    catch:
        println("not a tuple")
```

A bare `(a, b)` arm is not accepted — tuple arms must be written as
`try (a, b)`. The `catch:` arm replaces the old `_` wildcard and matches
anything.

An explicit memory placement can be requested after the type — `let int(heap): x`
or `let int(stack): x`. This is an advanced hint and not required.

Assignments use `=`. The compound operators (`+=`, `-=`, `*=` and `/=` are
tokenized but are not yet accepted by the parser — use the long form):

```lime
let mut n = 1
n = n + 1
```

---

## Functions

Functions are declared at the top level. Parameters are written `Type: name`
and are separated by commas. There is no return type in the signature; the
return type is inferred.

```lime
fn add(int: a, int: b):
    return a + b

fn main():
    println(add(1, 2))
    return
```

Parameters may be left untyped; their type is inferred from call sites and the
body:

```lime
fn identity(value):
    return value

fn main():
    println(identity("works"))
    return
```

A function returns with `return expr`. You may annotate the return type
explicitly, but it must still match the expression:

```lime
fn f():
    return int: 42
```

A bare `return` (with no expression) ends the function and returns the unit
value. A function whose last statement is a plain expression implicitly
returns that expression's value.

### Recursion

Functions can call themselves (and each other); all functions in a file are
visible to one another regardless of declaration order.

```lime
fn fact(int: n):
    if n <= 1:
        return 1
    else:
        return n * fact(n - 1)

fn main():
    println(fact(5))
    return
```

---

## Expressions and operators

Arithmetic: `+`, `-`, `*`, `/`, `%`. Integer division truncates (`10 / 3` is
`3`). The `+` operator also concatenates strings.

```lime
println(1 + 2 * 3)   // 7
println(10 / 3)      // 3
println(10 % 3)      // 1
println("a" + "b")   // ab
```

Comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`. These produce booleans.

```lime
println(1 < 2)    // true
println(2 <= 1)   // false
```

Boolean logic uses the words `and`, `or`, `not`. The symbol forms `&&`, `||`,
and `!` are also accepted.

```lime
println(5 > 3 and 2 < 4)   // true
println(not true)          // false
```

Conversions: `int(x)`, `float(x)`, and `str(x)` convert between numeric types,
to strings, and back.

```lime
println(int("42"))    // 42
println(float(7))     // 7
println(str(3.14))    // 3.14
```

`len(x)` returns the byte length of a string or the length of a list.

Function calls use `name(args)`. Struct construction is also a call:
`Point(3, 4)`. Variant construction works the same way: `Some(10)`, `Success(42)`.

Member access uses a dot: `p.x` reads a field, `nums.push(1)` calls a method
(see the sections on structs and lists).

List indexing and slicing:

```lime
let nums = [10, 20, 30]
println(nums[0])    // 10
println(nums[1])    // 20
let sliced = nums[0:2]
println(sliced)     // Slice[10, 20]
```

(Slicing a list produces a `Slice` value, printed as `Slice[10, 20]`.)

---

## Control flow

### if / else

There is no `elif`. Nest `else` blocks for multiple conditions.

```lime
fn classify(int: x):
    if x > 0:
        return "positive"
    else:
        if x < 0:
            return "negative"
        else:
            return "zero"

fn main():
    println(classify(5))   // positive
    return
```

The `else` branch is optional. The condition must be a boolean.

### while

```lime
let mut i = 0
while i < 3:
    println(i)
    i = i + 1
```

There is no `break` or `continue`.

### for

`for` iterates over a list or over an integer range `a..b` (from `a`
inclusive to `b` exclusive).

```lime
for n in [10, 20, 30]:
    println(n)

for i in 0..3:
    println(i)
```

### match

`match` destructures a value against patterns. The patterns are:

- `try (a, b)` — a tuple pattern (with nested patterns allowed); the bare
  `(a, b)` form is not accepted
- `catch:` — catch-all, matches anything (replaces the old `_` wildcard)
- `Some(v)` / `None` — `Option` variants
- `Variant(a, b)` — struct-like state/enum variants
- `try (v):` plus `error:` — matches the `Success`/`Error` variants of a
  result-like state; `error:` binds the failure payload to `error` and is only
  valid in a match that already has a `try (...)` arm

There is no `else` arm: `match` must be exhaustive.

```lime
match Some(5):
    Some(v):
        println(v)
    None:
        println("none")

let t = (1, 2)
match t:
    try (x, y):
        println(x + y)
    catch:
        println("wildcard")
```

### defer

`defer:` schedules a block to run when the current function returns. Multiple
defers run in the order they were scheduled.

```lime
fn main():
    defer:
        println("cleanup")
    println("before")
    return
```

Output:

```
before
cleanup
```

---

## Strings

Strings support method calls and a `string.` module. Methods are non-mutating;
they return new strings (or other values) rather than changing the receiver.

Method form:

```lime
let str: text = "Hello Lime"
println(text.length())     // 10 (characters)
println(text.to_upper())   // HELLO LIME
println(text.to_lower())   // hello lime
println(text.repeat(2))    // Hello LimeHello Lime
println(text.slice(0, 5))  // Hello
println(text.len())        // 10 (bytes)
```

The methods available on strings are `len`, `byte_len`, `length`, `chars`,
`bytes`, `slice`, `to_upper`, `to_lower`, `repeat`, `read`, `write`, `exists`,
`remove`, `append`, and `metadata`. Predicate methods such as `contains` do
**not** exist as methods — use the module form below.

Module form (equivalent), which lives in the `string` package and therefore
requires a `citrus.toml` project with `[import] string = "v0.1.0"`:

```lime
let str: s = "  Hello, Lime  "
println(string.trim(s))          // Hello, Lime
println(string.to_upper(s))      //   HELLO, LIME  (leading/trailing spaces preserved)
println(string.contains(s, "Li"))  // true
println(string.starts_with(s, "  He"))  // true
println(string.ends_with(s, "  "))      // true
println(string.replace("a-b-c", "-", "_"))  // a_b_c
println(string.repeat("ab", 3))            // ababab
let List(str): parts = string.split("x,y,z", ",")
println(parts)                            // [x, y, z]
println(string.slice("abcdef", 1, 4))     // bcd
```

`len(s)` gives the byte length; `.length()` gives the character count. For
ASCII text they agree.

StringBuilder appends incrementally:

```lime
let sb = StringBuilder()
sb.add("a")
sb.add("b")
println(sb.build())   // ab
```

---

## Lists

List literals use square brackets. Empty lists need a type annotation so the
element type can be inferred.

```lime
let List(int): nums = []
let xs = [1, 2, 3]
let ys = ["a", "b"]
```

Method form (non-mutating: `push` returns a new list, so reassign the binding):

```lime
let List(int): nums = []
nums = nums.push(1)
nums = nums.push(2)
nums = nums.push(3)
println(nums)                 // [1, 2, 3]
println(nums.length())        // 3
println(nums.first())         // 1
println(nums.last())          // 3
println(nums.contains(2))     // true
println(nums.index_of(2))     // 1
println(nums.reverse())       // [3, 2, 1]
println(nums.pop())           // 3
```

`len(nums)` gives the same value as `nums.length()`.

Module form (`collections.`), which requires a `citrus.toml` project with
`[import] collections = "v0.1.0"`:

```lime
let List(i): nums = [1, 2, 3]
println(collections.first(nums))    // 1
println(collections.last(nums))     // 3
println(collections.length(nums))   // 3
println(collections.contains(nums, 3))   // true
println(collections.index_of(nums, 2))   // 1
```

Indexing and slicing use the same syntax as strings:

```lime
let nums = [10, 20, 30]
println(nums[0])      // 10
let sliced = nums[0:2]
println(sliced)       // Slice[10, 20]
```

### HashMap and HashSet

These types come from the `collections` package, so this code also requires a
`citrus.toml` project with `[import] collections = "v0.1.0"`.

```lime
let HashMap(str, int): scores = collections.make_hash_map()
scores = scores.insert("math", 95)
scores = scores.insert("english", 88)
println(scores.get("math"))       // option.Some(...)
println(scores.contains("math"))  // true
println(scores.length())          // 2

let HashSet(str): tags = collections.make_hash_set()
tags = tags.add("compiler")
tags = tags.add("language")
println(tags.contains("compiler"))  // true
println(tags.length())              // 2
```

`get` returns an `Option` value from the `option` package (printed as
`option.Some(...)`). It can only be inspected by printing; the current `match`
does not destructure this package's `Some`.

The module forms are `collections.hashmap_insert`, `collections.hashmap_get`,
`collections.hashmap_contains_key`, `collections.hashmap_remove`,
`collections.hashmap_len`, `collections.hashset_add`,
`collections.hashset_contains`, `collections.hashset_remove`,
`collections.hashset_len`.

---

## Structs and methods

A struct declares fields as `Type: name` lines and can also declare methods.

```lime
struct Point:
    int: x
    int: y
    fn magnitude():
        return x * x + y * y

fn main():
    let p = Point(3, 4)
    println(p.x)          // 3
    println(p.magnitude())  // 25
    return
```

- Construct with `Name(arg1, arg2)`.
- Read fields with `p.field`.
- Methods are written like functions inside the struct body and can access the
  struct's fields directly (as in the `magnitude` example above).
- Methods can take arguments, including the struct itself.

There are also several standard library struct types: `Instant` and
`Duration` (time), and `FileMetadata` (filesystem). They are returned by the
`time.*` and `fs.*` functions below.

---

## Interfaces

An interface declares a set of method signatures. A struct conforms to an
interface by declaring methods with the same names and types — no explicit
`implements` clause is needed. The signature syntax puts the return type after
the parameter list, before the colon:

```lime
interface Animal:
    fn speak(str): str:
    fn legs(): int:

struct Dog:
    str: name
    int: legs
    fn speak(str):
        return "woof"
    fn legs():
        return 4

struct Cat:
    str: name
    int: legs
    fn speak(str):
        return "meow"
    fn legs():
        return 4

fn make_sound(Animal: a):
    print(a.speak(""))

fn main():
    let d = Dog("Rex", 4)
    make_sound(d)   // woof
    let c = Cat("Mimi", 4)
    make_sound(c)   // meow
    return
```

A parameter typed as an interface accepts any struct that conforms to it, and
method calls on that parameter dispatch to the concrete type.

---

## Generics

Functions, structs, and enums can take type parameters. A generic function
declares its type parameters between the name and the parameter list:

```lime
fn swap(T, U)(T: a, U: b):
    return (b, a)

fn main():
    let s = swap(1, "hi")
    println(s)   // (hi, 1)
    return
```

Generic structs and enums:

```lime
struct Box(T):
    T: value
    fn get():
        return value

enum Maybe(T):
    Just(T)
    Nothing

fn main():
    let b = Box(7)
    println(b.get())   // 7
    let Maybe(int): m = Just(42)
    match m:
        Just(v):
            println(v)
        Nothing:
            println("nothing")
    return
```

Type parameters with constraints use the `Where` clause (see the tests in
`tests/integration.rs` for examples).

---

## Async and await

Functions declared with the keyword `lime` instead of `fn` are async
functions. Calling one produces a future; `await` forces it and returns the
result.

In the current interpreter, `await` runs the async function to completion
immediately (synchronously). There is no parallelism and no suspension.

```lime
lime double(int: n):
    return n * 2

fn main():
    let result = await double(21)
    println(result)   // 42
    return
```

The LLVM backend lowers `await` to a direct synchronous call, matching the
interpreter's synchronous execution. There is no async runtime, scheduler, or
coroutine lowering; async (`lime`) functions are emitted as ordinary LLVM
functions.

---

## Compilation targets

Besides the interpreter (`lime run`), the compiler can emit native code.

### Emitting LLVM IR

```sh
lime build hello.lime --emit-ll
```

This writes `hello.ll` (textual LLVM IR) next to the source. This stage does
not require an LLVM toolchain.

### Emitting an object file and executable

```sh
lime build hello.lime --emit-object
```

This writes `hello.ll`, compiles it with `clang`, and links it into an
executable. It requires `clang`, `llvm-as`, and `lld-link` (LLVM 22) on your
`PATH` (or `LIME_LLVM_PREFIX` / `LLVM_SYS_221_PREFIX` set to the LLVM install
directory).

Currently the backend only lowers a subset of the language. A function whose
body uses a construct the backend cannot lower is emitted as a stub, the
compiler prints codegen warnings, and object emission is refused for safety.

Known backend limitations at the time of writing:

- `await` is lowered to a direct synchronous call; there is no real
  parallelism or async runtime.
- `long` literals (`42L`) are not lowered.
- State/enum and `Some`/`None` construction is not lowered.
- A `let` that binds a plain scalar (`let x = 1`) currently produces invalid
  IR for the store (`store i64 i64 1`), so programs that bind integer or float
  variables fail to compile with `--emit-object`. Programs that only print
  literals (strings) compile and run correctly.
- Top-level statements other than declarations are ignored when emitting
  native code; only definitions are emitted.

`--release` enables `-O2`-equivalent optimization during object emission.
`build` always runs a dead-code-elimination pass and reports how many unused
functions were removed.

---

## Compiler errors

Errors are printed with error codes, file locations, and source snippets.
There are four main categories:

**Lexer errors** (`error[E0001]`) — the source cannot be tokenized:

```
error[E0001] hello.lime: Invalid integer literal: 999999999999999999999
```

**Parser errors** (`error[E0101]`) — the tokens do not form valid syntax:

```
error[E0101] hello.lime: Expected variable name, got Assign (at line 2, col 5)
```

**Type errors** (`error[E02xx]`) — the program is well-formed but does not
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

**Runtime errors** (`error[E0601]`) — the interpreter hits a problem while
executing (for example, `Undefined variable: Nothing`).

`lime check` reports whether a file type-checks cleanly:

```
ok: hello.lime type-checks cleanly
```

---

## Feature compatibility table

Legend: **Yes** = fully supported, **No** = rejected or not implemented,
**Partial** = works in common cases but with documented gaps.

| Feature | Interpreter | LLVM Backend | Notes |
|---------|-------------|--------------|-------|
| `let` bindings | Yes | Partial | Scalar `let x = 1` produces invalid IR today |
| `let mut` + reassignment | Yes | Partial | Same scalar-store limitation |
| Type annotations / aliases | Yes | Yes | `int`, `i32`, `i`, `long`… |
| `Option(T)` / `T?`, `Some`/`None` | Yes | No | Not lowered by the backend |
| Tuples and destructuring | Partial | No | `let (a, b) = ...` is parsed but not type-checked; use `match` patterns |
| Functions, recursion | Yes | Yes | Calls are lowered |
| Generic functions/structs | Yes | Partial | — |
| `fn` params `Type: name` / untyped | Yes | Yes | — |
| `return Type: expr` | Yes | Yes | — |
| Arithmetic `+ - * / %` | Yes | Yes | — |
| Comparison `< <= > >= == !=` | Yes | Yes | — |
| `and` / `or` / `not` | Yes | Yes | — |
| String concatenation `+` | Yes | Yes | — |
| String indexing/slicing | Yes | No | — |
| List indexing `list[i]` | Yes | Partial | — |
| List slicing `list[a:b]` | Yes | No | — |
| if / else | Yes | Yes | No `elif` |
| while | Yes | Yes | No `break`/`continue` |
| for over list / range | Yes | Partial | — |
| match + patterns | Yes | Partial | — |
| defer | Yes | No | — |
| struct + fields | Yes | Yes | — |
| struct methods | Yes | Yes | — |
| interfaces | Yes | Partial | — |
| state / enum variants | Yes | No | Construction not lowered |
| String methods | Yes | Partial | — |
| `string.*` module | Yes | No | — |
| List methods (`push`, `pop`, …) | Yes | Partial | — |
| `collections.*` module | Yes | No | — |
| HashMap / HashSet | Yes | No | — |
| StringBuilder | Yes | No | — |
| `int()` / `float()` / `str()` | Yes | No | — |
| `len()` | Yes | Partial | — |
| `print` / `println` | Yes | Yes | Both print each arg on its own line |
| `time.*` module | Yes | No | `now()`, `elapsed()`, `sleep()` |
| `fs.*` module | Yes | No | `write`, `exists`, `metadata`, … |
| `math.*` module | Yes | No | `abs`, `max`, `min`, `sqrt`, `pow` |
| `lime` async fn + `await` | Yes | Yes | `await` lowers to a direct synchronous call; no parallelism |
| Type-check (`lime check`) | Yes | — | Same checker feeds both paths |
