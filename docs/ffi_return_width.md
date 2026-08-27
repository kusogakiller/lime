# Lime FFI Boundary — Extern Return Width Policy

Iteration 33 design note (P4). No implementation shipped; this documents the
measured behaviour and the chosen minimal-risk policy.

## Measured behaviour (current HEAD)

A RAW C extern declared as returning Lime `Int` is lowered as a full-width
i64 call result with no knowledge of the C function's true return width:

```c
/* C */
signed char ptd_sc(signed char v) { return v - 1; }
```

```lime
// Lime
extern fn ptd_sc(Int: v) -> Int "ptd_sc"
let r = ptd_sc(0)   // C returns -1 (AL = 0xFF, EAX zero-extended)
print(r)            // prints 4294967295  (= 0x00000000FFFFFFFF)
```

The callee only defines the low 8/16/32 bits of RAX per the C ABI; the upper
bits are undefined-for-the-contract. Lime reading a full i64 therefore
observes garbage in the high bits whenever the C value is negative (or wider
than the declared Lime type expects).

## Options considered

### Option A — raw externs make no narrow-return guarantee (CHOSEN for now)

* Raw `extern fn ... -> Int` means "the native returns something whose low
  64 bits are the value"; anything narrower is the caller's problem.
* The OFFICIAL FFI boundary is a Charger-prepared shim: Charger-generated
  accessor/wrapper shims widen every scalar return to `long long` on the C
  side, which makes the i64 read exact. This is precisely why all Official
  Support smokes pass end-to-end today.
* Pros: zero implementation risk, matches the frozen Charger architecture,
  no cross-ABI design burden.
* Cons: raw externs of narrow-returning C functions need a hand-written or
  Charger shim wrapper.

### Option B — width-aware extern declarations

Extend extern syntax/type system with the true C return type so codegen can
emit truncation/sign-extension at the boundary.

* Pros: general raw FFI.
* Cons: language + parser + checker + codegen changes; must be reasoned for
  every future ABI (SysV/ARM64), not just Win64. Explicitly out of scope for
  the reliability-hardening iteration and would touch the frozen Lime ABI
  design without user direction.

## Policy going forward

1. Prefer Charger-prepared shims for anything returning narrower than 64
   bits (this is already how every Official Support library is exercised).
2. If a raw extern to a known narrow-returning C function is unavoidable,
   wrap it on the C side (one-line shim) instead of relying on undefined
   upper bits.
3. Revisit Option B only if a concrete Official Support target demands raw
   narrow externs; treat it as a designed language feature, not a patch.
