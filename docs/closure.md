# Closure ABI

Lime closures use a uniform ABI based on the `%LimeClosure` fat pointer
type. All closure-compatible functions share the same calling convention.

## Closure Type

```
%LimeClosure = type { i8* fn_ptr, i8* env_ptr }
```

- `fn_ptr` (i8*): pointer to the closure's function code
- `env_ptr` (i8*): pointer to the environment array (captured values),
  or NULL for closures with no captures

## Function Signature

All closure-compatible functions use this signature:

```
define i64 @func_name(i8* %env, i8* %packed_args)
```

- `%env`: pointer to the closure's environment array (captured values)
- `%packed_args`: pointer to a heap-allocated i64 array of arguments

## Argument Packing

Arguments are packed into a heap i64 array, one i64 per argument, in
order. Each argument is stored as its LLVM type cast to i64:

| Lime Type | LLVM Type | Packing |
|-----------|-----------|---------|
| int       | i64       | direct  |
| float     | double    | bitcast double -> i64 |
| bool      | i1        | zext i1 -> i64 |
| str       | i8*       | ptr -> i64 (truncate) |

## Environment Layout

Captured values are stored in a heap i64 array, one i64 per capture, in
order of appearance in the closure body. Each capture is converted to i64
using the same rules as argument packing.

### Unpacking in the Closure Function

The generated closure function unpacks captures from the env array:

```llvm
; GEP into env to get i64* for capture i
; Load i64 from env array
; Convert from i64 to actual type:
;   float: bitcast i64 -> double
;   bool:  trunc i64 -> i1
;   other: add i64 0 (identity)
; Store into alloca for the body to reference
```

## Closure Creation

### With Captures

```llvm
; Pack captures into heap i64 array
; Call runtime_make_closure(fn_ptr, env_ptr)
```

### Without Captures

```llvm
; Call runtime_make_fn_ref(fn_ptr) — env_ptr is NULL
```

## Calling Closures

### From Native Code

```llvm
; %closure is a %LimeClosure*
; Extract fn_ptr and env_ptr
; Call the appropriate runtime helper
```

### From the Interpreter

The interpreter dispatches through `call_closure` which looks up the
closure's function in the registry and invokes it with the packed args.

## Return Values

All closure functions return `i64`. Return values are the LLVM return
value of the function. For `float` returns, the value is bitcast from
`double` to `i64` before returning. For `bool` returns, the value is
zext from `i1` to `i64`.

## Named Function Wrappers

Named user functions that are used as closure values get a wrapper
generated:

```llvm
define i64 @wrap_<name>(i8* %env, i8* %packed) {
  ; Unpack arguments from packed struct
  ; Call the real function
  ; Return the result as i64
}
```

## Limitations

1. All closures return `i64` in native code (MVP limitation).
2. String captures store the pointer (not a copy) — the original string
   must outlive the closure.
3. No mutable capture — captured values are immutable by-value copies.
4. No inline anonymous functions in call arguments (not yet supported
   in codegen).