# Regex Package (Phase C-1.10)

Regular expression utilities for Lime. Provides pattern matching, searching, and replacement operations.

## Usage

Add `regex = "v0.1.0"` to your `citrus.toml` dependencies:

```toml
[dependencies]
regex = "v0.1.0"
```

## API Reference

### `regex.is_match(pattern, text)`

Check if the pattern matches the entire string.

```lime
println(regex.is_match("[0-9]+", "abc123"))   // true
println(regex.is_match("[0-9]+", "abc"))       // false
println(regex.is_match("^hello$", "hello"))    // true
println(regex.is_match("^hello$", "hello world")) // false
```

### `regex.find(pattern, text)`

Find the first match in the string. Returns `Option(String)`.

```lime
let result = regex.find("[0-9]+", "abc123def456")
// result = Some("123")

let no_match = regex.find("[0-9]+", "abcdef")
// no_match = None
```

### `regex.find_all(pattern, text)`

Find all non-overlapping matches. Returns `List(String)`.

```lime
let all = regex.find_all("[0-9]+", "a1 b2 c3")
// all = ["1", "2", "3"]

let words = regex.find_all("[a-z]+", "Hello 123 World 456")
// words = ["Hello", "World"]
```

### `regex.replace(pattern, text, replacement)`

Replace the first match with replacement.

```lime
println(regex.replace("[0-9]+", "abc123", "X"))    // abcX
println(regex.replace("cat", "The cat sat", "dog")) // The dog sat
```

### `regex.replace_all(pattern, text, replacement)`

Replace all non-overlapping matches with replacement.

```lime
println(regex.replace_all("[0-9]+", "a1 b2 c3", "X"))  // aX bX cX
println(regex.replace_all("a", "banana", "o"))          // bonono
```

### `regex.split(pattern, text)`

Split the string by the pattern. Returns `List(String)`.

```lime
let parts = regex.split("[ ,]+", "hello, world  foo")
// parts = ["hello", "world", "foo"]

let nums = regex.split("\\d+", "abc123def456ghi")
// nums = ["abc", "def", "ghi"]
```

## Supported Regex Syntax

### Literal Characters
Any character matches itself: `a`, `1`, `!`, etc.

### Wildcard
`.` matches any character except newline.

### Anchors
- `^` — start of string
- `$` — end of string

### Character Classes
- `[abc]` — matches a, b, or c
- `[a-z]` — matches any lowercase letter
- `[A-Za-z0-9]` — matches letters and digits
- `[^abc]` — matches anything except a, b, or c

### Quantifiers
- `*` — zero or more
- `+` — one or more
- `?` — zero or one
- `{n}` — exactly n times
- `{n,m}` — between n and m times
- `{n,}` — n or more times

### Groups and Alternation
- `(abc)` — capturing group
- `(?:abc)` — non-capturing group
- `a|b` — alternation (match a or b)

### Escape Sequences
- `\d` — digit (`[0-9]`)
- `\D` — non-digit
- `\w` — word character (`[a-zA-Z0-9_]`)
- `\W` — non-word character
- `\s` — whitespace
- `\S` — non-whitespace
- `\b` — word boundary
- `\B` — non-word boundary
- `\\` — literal backslash

### Inline Flags
- `(?i)` — case-insensitive matching

## Error Behavior

Invalid patterns return standard failure values:
- `regex.is_match()` returns `false` for invalid patterns
- `regex.find()` returns `None` for invalid patterns
- `regex.find_all()` returns an empty list for invalid patterns
- `regex.replace()` returns the original text for invalid patterns
- `regex.replace_all()` returns the original text for invalid patterns
- `regex.split()` returns the original text as a single-element list

## Implementation Notes

- **Interpreter**: Uses the Rust `regex` crate for full regex support.
- **Native codegen (C runtime)**: Uses a built-in recursive backtracking regex engine supporting the features listed above.
- The C runtime engine supports case-insensitive matching via `(?i)` flag.
- Zero-length matches are handled to prevent infinite loops.

## Future Improvements

- Capture group extraction API
- Named capture groups
- Lookahead/lookbehind assertions
- Streaming regex for large inputs
- Regex compilation caching
- Performance optimizations for common patterns
