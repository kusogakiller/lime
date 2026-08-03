# Ownership and Memory Management

Lime has no garbage collector and no borrow checker. Values are either
stack-allocated (immediate) or heap-allocated (runtime-allocated objects).
This document describes the ownership model and lifetime rules.

## Value Semantics

All Lime values are passed by value. When a variable is assigned or passed
to a function, the value is copied (for immediate types) or the reference
is shared (for runtime-allocated objects).

### Immediate Types (stack-allocated)

These types are stored directly in the stack frame and copied on assignment:

- `int` (i64)
- `float` (f64)
- `bool` (i1)
- `Unit` (void)

### Runtime-Allocated Objects (heap-allocated)

These types are allocated on the heap via `runtime_alloc` and accessed
through pointers:

- `str` (i8* pointer to NUL-terminated UTF-8 string)
- `List(T)` (i8* pointer to runtime list structure)
- `%LimeClosure` (i8* pointer to closure struct containing fn_ptr and env_ptr)
- `%Option` (i64 union tag + payload)
- `%Result` (i64 union tag + payload)

## Runtime Allocation

All heap allocations go through `runtime_alloc(size, align)`, which uses
`malloc` internally. There is no explicit deallocation — memory is freed
when the process exits.

### Allocation Rules

1. `runtime_alloc` returns a pointer to zero-initialized memory.
2. The caller is responsible for ensuring the allocation is large enough.
3. Allocations are never explicitly freed (process lifetime).
4. On allocation failure, `runtime_panic` is called (aborts the program).

## Closure Capture

Closures capture variables from their enclosing scope by value (immutable).
Each closure invocation gets its own copy of captured variables.

### Capture Mechanism

Captured values are packed into a heap-allocated i64 array (`env` pointer)
and passed to the closure function alongside the packed arguments.

```
%LimeClosure = type { i8* fn_ptr, i8* env_ptr }
```

- `fn_ptr`: pointer to the closure's function code
- `env_ptr`: pointer to the heap array of captured values

### Capture Types Supported

- `int` — packed as i64
- `float` — packed as i64 (bitcast from double)
- `bool` — packed as i64 (zext from i1)
- `str` — pointer packed as i64 (shared reference, not copied)

### Limitations

- Mutable capture is not supported.
- String captures share the original pointer (no copy-on-write).
- All captured values are packed as i64 in the env array.

## List and String Ownership

Lists and strings are reference-counted at the runtime level through the
`runtime_alloc`/`runtime_str_concat`/`runtime_list_add` APIs. There is
no reference counting in the current implementation — lists and strings
are owned by the variable that holds them and are not shared between
variables.

### String Handling

- Strings are NUL-terminated UTF-8 sequences stored on the heap.
- `runtime_str_concat` allocates a new string and returns a pointer.
- `runtime_str_from_i64` / `runtime_str_from_f64` / `runtime_str_from_bool`
  allocate new strings for conversion.
- `runtime_str_from_option` treats the payload as int64_t; float bits
  show raw values (architectural limitation).

## Lifetime Rules

1. Variables are valid for the scope in which they are declared.
2. Closures capture variables by value at the point of closure creation.
3. Captured values live as long as the closure lives.
4. Function parameters are valid for the duration of the function call.
5. Return values are valid after the function returns.

## No Borrow Checker

Lime does not implement a borrow checker or ownership tracking system.
The runtime does not enforce ownership rules. The compiler's type checker
ensures type safety but does not track ownership or lifetimes.

This means:
- No compile-time borrow checking (no `&` or `mut` semantics).
- No compile-time lifetime annotations.
- No move semantics — all values are copied or shared by pointer.
- No compile-time prevention of use-after-free (not applicable since
  there is no deallocation).