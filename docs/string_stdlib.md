# String Standard Library

The `std.string` package provides utilities for string manipulation. All functions are implemented as runtime builtins with thin package wrappers.

## Package Import

```lime
import std.string
```

## Core Operations

| Function | Signature | Description |
|----------|-----------|-------------|
| `len` | `fn len(str: s) -> int` | Number of Unicode characters |
| `byte_len` | `fn byte_len(str: s) -> int` | Number of bytes |
| `is_empty` | `fn is_empty(str: s) -> bool` | Whether the string is empty |
| `contains` | `fn contains(str: s, str: sub) -> bool` | Whether `sub` occurs inside `s` |
| `starts_with` | `fn starts_with(str: s, str: prefix) -> bool` | Whether `s` starts with `prefix` |
| `ends_with` | `fn ends_with(str: s, str: suffix) -> bool` | Whether `s` ends with `suffix` |
| `find` | `fn find(str: s, str: sub) -> int` | Byte offset of first occurrence of `sub`, or -1 |
| `count` | `fn count(str: s, str: sub) -> int` | Number of non-overlapping occurrences of `sub` |

## Transformation

| Function | Signature | Description |
|----------|-----------|-------------|
| `trim` | `fn trim(str: s) -> str` | Trim leading/trailing ASCII whitespace |
| `trim_start` | `fn trim_start(str: s) -> str` | Trim leading ASCII whitespace |
| `trim_end` | `fn trim_end(str: s) -> str` | Trim trailing ASCII whitespace |
| `replace` | `fn replace(str: s, str: from, str: to) -> str` | Replace every occurrence of `from` with `to` |
| `slice` | `fn slice(str: s, int: start, int: end) -> str` | Substring [start, end) by Unicode character index |
| `substring` | `fn substring(str: s, int: start, int: end) -> str` | Alias for `slice` |
| `to_upper` | `fn to_upper(str: s) -> str` | Uppercase |
| `to_lower` | `fn to_lower(str: s) -> str` | Lowercase |
| `repeat` | `fn repeat(str: s, int: times) -> str` | Repeat `s` `times` times |

## Join / Split

| Function | Signature | Description |
|----------|-----------|-------------|
| `split` | `fn split(str: s, str: sep) -> list(str)` | Split `s` on `sep` into List(str) |
| `join` | `fn join(str: sep, list(str): parts) -> str` | Join `parts` with `sep` between each element |

## Conversion

| Function | Signature | Description |
|----------|-----------|-------------|
| `to_int` | `fn to_int(str: s) -> int` | Parse as signed integer. Returns 0 on failure. |
| `to_float` | `fn to_float(str: s) -> float` | Parse as float. Returns 0.0 on failure. |

## Comparison

| Function | Signature | Description |
|----------|-----------|-------------|
| `equals` | `fn equals(str: s, str: other) -> bool` | Case-sensitive equality |
| `compare` | `fn compare(str: s, str: other) -> int` | Lexicographic comparison. Returns -1, 0, or 1. |

## Examples

```lime
import std.string

fn main():
    let s = "  hello world  "
    
    // Trim
    println(std.string.trim(s))        // "hello world"
    println(std.string.trim_start(s))  // "hello world  "
    println(std.string.trim_end(s))    // "  hello world"
    
    // Search
    println(std.string.contains("hello", "ell"))  // true
    println(std.string.find("hello", "ll"))       // 2
    println(std.string.count("hello", "l"))       // 2
    println(std.string.is_empty(""))              // true
    
    // Case
    println(std.string.to_upper("abc"))  // "ABC"
    println(std.string.to_lower("ABC"))  // "abc"
    
    // Join / Split
    let parts = std.string.split("a,b,c", ",")
    println(std.string.join("-", parts))  // "a-b-c"
    
    // Conversion
    println(std.string.to_int("42"))      // 42
    println(std.string.to_int("abc"))     // 0 (failure)
    println(std.string.to_float("3.14"))  // 3.14
    
    // Comparison
    println(std.string.equals("a", "a"))  // true
    println(std.string.compare("a", "b")) // -1
    return
```

## Runtime Builtins

All package functions map to runtime builtins in `src/codegen/runtime/runtime.c`:

| Package Function | Runtime Builtin |
|------------------|-----------------|
| `len` | `runtime_str_slice` (via `strlen`) |
| `byte_len` | `strlen` |
| `is_empty` | `runtime_str_is_empty` |
| `contains` | `runtime_str_contains` |
| `starts_with` | `runtime_str_starts_with` |
| `ends_with` | `runtime_str_ends_with` |
| `find` | `runtime_str_find` |
| `count` | `runtime_str_count` |
| `trim` | `runtime_str_trim` |
| `trim_start` | `runtime_str_trim_start` |
| `trim_end` | `runtime_str_trim_end` |
| `replace` | `runtime_str_replace` |
| `slice` / `substring` | `runtime_str_slice` |
| `split` | `runtime_str_split` |
| `join` | `runtime_str_join` |
| `to_upper` | `runtime_str_to_upper` |
| `to_lower` | `runtime_str_to_lower` |
| `repeat` | `runtime_str_repeat` |
| `to_int` | `runtime_str_to_int` |
| `to_float` | `runtime_str_to_float` |
| `equals` | `runtime_str_equals` |
| `compare` | `runtime_str_compare` |

## Codegen

Each builtin has a corresponding codegen case in `src/codegen/fn_builder.rs` that emits a direct LLVM IR call to the runtime function. The LLVM IR declarations are in `src/codegen/mod.rs`.