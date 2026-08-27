# Lime Grammar Specification (EBNF + Type Rules)

## Purpose

To formalize the grammar in EBNF and define type rules that are consistent with
the current prototype (Lexer / Parser / AST / TypeChecker / Interpreter).
Syntax follows existing specs and decided items.

Preserved principles:
- Easy. Simple. Fast.
- Concise syntax / readability first
- No Rust-ification / No C++-ification / No self-this / No impl
- No implicit type conversions (numeric→bool also forbidden)
- No GC / no compiler automatic memory management
- Match exhaustive matching required / `else` forbidden / `Ignore` for discards
- No string-specific operators (use String API instead)

---

## 1. Lexical Rules (Lexer exists, changes forbidden)

| Category | Example |
|----------|---------|
| Identifier | `[A-Za-z_][A-Za-z0-9_]*` |
| Keywords | fn struct interface state match if else return let mut lime await unsafe |
| Integer literal | `123` / `0xFF` (later) |
| Float literal | `1.5` / `.5` |
| String literal | `"..."` |
| Operators | `+ - * / % == != < > <= >= && \|\| ! = += -=` |
| Delimiters | `( ) [ ] { } : , . .. ; ->` |
| Indentation | Indent / Dedent / Newline |

---

## 2. Program Structure (EBNF)

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
                 (indent_block contains only fn_decl signatures)

struct_decl    ::= "struct" ident type_params? ":" indent_block
                 (No explicit `implements` required. If a struct has
                   matching signatures for all methods of an interface,
                   it is automatically considered an implementation = implicit implementation)

state_decl     ::= "state" ident type_params? ":" indent_block
                 (indent_block contains variant names + optional payloads)

let_stmt       ::= "let" "mut"? type ":" ident "=" expr
                 | "let" "mut"? ident "=" expr

if_stmt        ::= "if" expr ":" block ("else" ":" block)?

match_stmt     ::= "match" expr ":" indent_block
                 (each arm: variant_pattern ":" block)
variant_pattern ::= ident ("(" binding_list? ")")?
binding_list   ::= ident ("," ident)* | "Ignore" ("," "Ignore")*

return_stmt    ::= "return" expr?

expr_stmt      ::= expr
```

---

## 3. Expressions (EBNF)

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

Note: String concatenation via the `+` operator is allowed (when both sides are
`str`). This extends the existing `+` usage; no new string-specific operators
are added.
Numeric types use arithmetic operators. No string-specific operators (custom
symbols etc.) exist.

---

## 4. Type Rules

### 4.1 Basic Types

| Type Name | Meaning |
|-----------|---------|
| `int` | Signed integer |
| `float` | Floating point |
| `bool` | Boolean |
| `str` | UTF-8 string |
| `byte` | One byte (for UTF-8 operations) |
| `char` | Unicode codepoint unit |

### 4.2 Composite Types

| Type Syntax | Meaning |
|-------------|---------|
| `List(T)` | List (unified fixed/variable length. E.g., `List(int)`) |
| `Map(K, V)` | Map |
| `Set(T)` | Set |
| `Tuple(T1, T2, ...)` | Tuple |
| `Option(T)` | Null safety (`T?` shorthand allowed) |
| `Result(T, E)` | Success/failure result type. A pair of `Success(T)` / `Error(E)` (`Ok`/`Err` not used). Built-in without requiring a `state` declaration (implemented via existing State + Generic mechanism). |
| `State` derived | Defined via `state Name(T):` |
| `Struct` derived | Defined via `struct User:` |
| `Interface` derived | Defined via `interface Animal:` |
| `T*` | Pointer (unsafe only) |

Note: Arrays and lists are unified as `List(T)` (fixed/variable length distinction is unnecessary or determined by the Runtime).

### 4.3 Type Conversion Rules

- **Implicit conversion: completely forbidden**.
- **Explicit conversion (function form)**:
  - `int(x)`: x is float/str → int
  - `float(x)`: x is int/str → float
  - `str(x)`: any displayable value → str
  - `bool(x)`: **forbidden** (numeric→bool is not allowed)
  - bool is only available via comparison operator results or dedicated boolean expressions.
- **`int(float)` conversion rules (fixed)**:
  - The fractional part is **truncated toward 0**.
  - Example: `int(2.9) = 2` / `int(-2.9) = -2`
  - Same semantics as Rust's `as i64` (`f64 as i64`).
- On conversion failure, returns an `Error` State (to be decided together with Error propagation later).

### 4.4 String API Types

| Method | Return Type |
|--------|-------------|
| `.bytes()` | `Array(byte)` |
| `.chars()` | `Array(char)` |
| `.slice(a, b)` | `str` |
| `.len()` | `int` (Unicode character count) |
| `.byte_len()` | `int` (byte length) |

### 4.5 Operator Interface (for user-defined types)

| Interface | Operators | Resolved Method | Return Value Interpretation |
|-----------|-----------|-----------------|----------------------------|
| `Add` | `+` | `add` | As-is (struct) |
| `Equal` | `==` `!=` | `equal` | bool (`!=` is negation) |
| `Compare` | `<` `>` `<=` `>=` | `compare` | int sign comparison (`<` : sign<0 etc.) |

`Sub` / `Mul` / `Div` will be added later (incremental extension).
Users only need to implement `fn add(...)` etc. to use operators (implicit
implementation).
Strings are excluded (use String API instead). Built-in numeric, bool, and
string types maintain their existing built-in operations.

Naming prioritizes clarity for beginners (Easy. Simple. Fast.):
- `Eq` was not adopted because the abbreviation is not intuitive; `Equal` was chosen.
- `Ord` was not adopted because "Order" abbreviation is unclear; `Compare` was chosen.

#### Static Type Resolution (AST Storage Method)

Operator resolution is performed **exclusively by the TypeChecker**, and the
result is stored in `BinOp.resolved_operator` in the AST. The Interpreter /
future LLVM Backend reads only this information to execute, and **performs no
runtime type searches or Struct-name-based Interface searches**.

```
BinOp:
  - left expression
  - operator
  - right expression
  - resolved_operator: Option<ResolvedOperator>

ResolvedOperator:
  - Builtin                         # Existing operation for built-in types (int/float/str etc.)
  - MethodCall { method, op }       # Via Operator Interface (e.g., Add.add / Equal.equal / Compare.compare)
```

- Only user-defined types (both sides are the same struct) are resolved via Operator Interface.
- `!=` negates the `equal()` result; `<` `>` `<=` `>=` compare the `compare()` return int against 0 to produce a bool.
- Built-in types are not resolved and remain `Builtin` (maintaining existing operations).

### 4.6 Generic / Constraint

```
type_params    ::= "(" ident ("," ident)* ")"
constraint     ::= ident "where" ident ":" ident ("," ident ":" ident)*
```

Example: `fn max(List(T where T: Compare)): T:`

---

## 5. Control Structure Types

### 5.1 if

- The condition **must always be wrapped in parentheses**: `if (cond):`
- The condition type must be `bool` (no implicit conversion).
- The last expression in the then/else block must match the return value type.

### 5.2 match

- The subject expression type must be a `State` derivative.
- All variants must be covered (missing variants cause a compile error).
- `else` is forbidden.
- Each arm's binding can be discarded with `Ignore`.

### 5.3 Loops (later implementation)

```
loop_stmt     ::= "for" ident "in" expr ":" block
               | "for" ident "in" range ":" block
               | "while" "(" expr ")" ":" block
range         ::= expr ".." expr
```

---

## 6. Operator Specification

### 6.1 Operator List

Arithmetic operators:
- `+` `-` `*` `/` `%`

Comparison operators:
- `==` `!=` `<` `>` `<=` `>=`

Logical operators (word form):
- `and` `or` `not`
- `&&` `||` `!` are not used.

### 6.2 Parentheses Required for Conditions

Conditions in `if` / `while` etc. must always be wrapped in parentheses.

Example:
```
if (a >= 10 and b != 0):
    ...

while (count < 10):
    ...
```

Reasons:
- Reduce dependency on operator precedence
- Improve readability
- Simplify parser implementation
- Align with beginner-friendly design

### 6.3 Operator Precedence (recommended spec)

From highest to lowest:
1. `not`
2. `*` `/` `%`
3. `+` `-`
4. `<` `>` `<=` `>=`
5. `==` `!=`
6. `and`
7. `or`

However, explicit specification via parentheses is recommended.

---

## 7. Async (Decided Items)

Regular functions can also be asynchronous. To align with the Collection spec
unified under `List()`, async functions are declared with the `lime` keyword.

```
async_fn      ::= "lime" ident "(" param_list? ")" type? ":" block
await_expr    ::= "await" call_expr
```

- Regular functions: `fn function():`
- Async functions: `lime function():`
- `let data = await request("url")`
- The `async` reserved keyword is not used (Lime uses its own syntax).
- Regular functions (`fn`) cannot participate in async processing. `await` usage is only allowed inside `lime` functions.
- Callable rules and detailed Runtime behavior for await will be specified in the Async / Runtime design.

---

## 7. Memory Specification (implemented: Step 9)

Basic principle: No GC. Users are not required to write ownership/lifetime
annotations. The compiler handles automatic memory management internally.
By default, Escape Analysis determines Stack / Heap placement.
Memory information is determined at compile time; no Runtime lookups.

Placement rules:
- Stack: values used only within the function / not returned / not passed to callbacks / not stored in Heap data.
- Heap: values that escape the function / values held across async processing / long-lived data.

Syntax (explicit specification):
- `let User(heap): user = User("Alice")`  → always Heap
- `let User(stack): point = Point(1, 2)` → always Stack
- No explicit spec `let User: user = ...`          → automatic via Escape Analysis

Escape determination (compiler internal):
- Value is `return`ed → Escape (Heap)
- Passed as argument to `await` in a `lime` function / used after `await` → held in Heap frame
- Since closures capture variables by value, actual arguments of
  normal function calls are not considered Escape (compiler manages intelligently)

Constraints (compile errors):
- If a `stack`-specified value escapes (return / await hold),
  `Memory error: '<name>' is explicitly placed on the stack but escapes ...` is produced.

Struct: primarily value type. Non-Escape = Stack; only Heap on Escape.
List: header can typically be Stack; internal buffer is Heap-managed (treated as `List(T)`).
Option / Result: follows the Memory properties of the internal value, no special handling (maintains existing State + Generic).
Generic: Memory analysis is performed after type substitution. No generic-specific Memory rules are added.
Async: Future / Async frames are Heap-allocated. State-holding areas in lime functions are Runtime-managed.

Analysis flow:
```
AST → TypeChecker → Memory Analysis (Escape Analysis) → (LLVM)
```

---

## 8. Prohibitions (maintained in this spec)

- Rust-ification ('a / lifetime annotations / borrow checker exposure)
- C++-ification (inheritance / free template definitions / operator overuse)
- self / this
- impl block form
- Implicit type conversions (including numeric→bool)
- String-specific operators
- match else
- `_` for Ignore (use `Ignore` instead)

---

## 9. Type Checker Consistency

The current prototype TypeChecker covers:
- Basic type literals / variables / Binary / Call / Struct constructor /
  FieldAccess / MethodCall / State constructor type checking
- Struct field type checking
- Function argument / return value type checking
- Match exhaustiveness checking

Not yet implemented (later steps):
- Pointer / unsafe types
- Built-in type definitions for explicit conversions like `int(x)`
- LLVM integration (currently Memory analysis results are reported as debug output `=== Memory ===`)

---

## 10. Next Steps (implementation order)

Based on this EBNF + type rule, implement the prototype in the following order:
1. Explicit conversion API (int/float/str)
2. String API (.len/.byte_len/.chars/.bytes/.slice)
3. Collections (List unification + literals + types)
4. Loops (for / while / range)
5. Option (T? / Match)
6. Generic (Result(T) / Box(T) / Constraint)
7. Interface (implicit implementation + Operator Interface)
 8. Async (lime functions + await) [implemented]
 9. Memory analysis (Escape / Lifetime / Stack-Heap) [implemented]
 10. LLVM integration

Prohibitions are maintained at all stages.
