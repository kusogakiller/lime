# OpenSSL integration status (Iteration 10)

## Goal
Add OpenSSL 3.3.2 to the Charger native-execution gate as a high-priority
C-ABI stress test (opaque structs, const correctness, callbacks,
pointer-to-pointer, allocator/free, nested typedefs, macro-heavy headers,
platform conditionals).

## Blocked: environment provisioning, NOT a Charger defect

Source present at `C:/Users/szzxl/Downloads/openssl-3.3.2` (113 pre-generated
`.h`, 64 `crypto/*.c`, `opensslconf.h` present) but the tree is **not
configured** (`configdata.pm` absent; 26 public headers are `.h.in` templates;
`include/openssl/configuration.h` is missing — only `.h.in` exists).

### Evidence (clang AST probe, required "inspect AST first" step)
Every OpenSSL public header transitively does:
  `#include <openssl/opensslconf.h>` -> `#include <openssl/configuration.h>`
`configuration.h` is a `.h.in` template expanded only by `Configure`.
Result: `clang -Xclang -ast-dump=json -fsyntax-only` on `bn.h`/`evp.h`/`aes.h`
all fail with:
  `fatal error: 'openssl/configuration.h' file not found`
So Charger's AST-extraction stage cannot run on ANY OpenSSL header until the
tree is configured. (Confirmed 2026-08-19.)

### Configure cannot run either
`perl Configure ...` aborts:
  `Can't locate Locale::Maketext::Simple.pm in @INC`
The only Perl present is MSYS/Cygwin 5.42.2, which is missing that module
AND `CPAN` itself (cpan install also fails). No Strawberry/complete Perl on
the machine. So Configure cannot generate `configuration.h` / expand `.h.in`.

## Required to unblock (environment, not Charger)
1. A complete Perl (e.g. Strawberry Perl) so `Configure` runs, OR
2. Manually generate `include/openssl/configuration.h` + expand the 26
   `.h.in` templates (fragile, not preferred), then
3. Build OpenSSL with the LLVM 22 clang toolchain to produce a native artifact.

After (1)/(2)+(3), `charger install bench_clang/realworld/corpus/openssl`
becomes feasible; the AST/adapter work (opaque BIGNUM/EVP_PKEY, typedef
chains, callbacks) can then be validated with executable proof per the
engineering rules.

## Rule reminder
Do NOT claim OpenSSL support without executable proof. No library-specific
branches in charger.rs.
