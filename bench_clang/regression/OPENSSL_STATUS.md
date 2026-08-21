# OpenSSL integration status (Iteration 10 / 11)

## Summary (as of 2026-08-21)

| Gate | Status | Evidence |
|------|--------|----------|
| INSTALL | **GREEN** | `charger install` EXIT=0; store `openssl-3.3.2/0.1.0/567ecbd8b952efde`; 1958 funcs / 145 structs |
| LINK | GREEN only in the weak/unresolved-allowed sense | Lime smoke links via lld-link (unresolved syms weakened, become NULL at runtime) |
| NATIVE EXECUTION | **BLOCKED** | `EVP_DigestInit_ex` SEGVs (exit 139) calling a NULL provider dispatch pointer |

**OpenSSL support is NOT declared GREEN.** Only the install/adapter-extraction
gate is proven. SHA-256 digest does NOT complete to exit 0.

## What was resolved (environment provisioning — DONE)

The earlier "environment blocked" state is resolved:
- Strawberry Perl installed (Configure now runs).
- `perl Configure VC-CLANG ...` generated `include/openssl/configuration.h` and
  expanded the `.h.in` templates.
- `charger install` compiles all ~2000 TUs with LLVM 22 clang and archives them.

## Root cause of the native-execution block (NOT a Charger defect)

Decisive test (C.1/C.2): a **pure native C caller** that touches NO Lime code —
`OPENSSL_init_crypto`, `OSSL_PROVIDER_load("default")`, `EVP_sha256`,
`EVP_MD_CTX_new`, `EVP_DigestInit_ex`, `EVP_DigestUpdate`, `EVP_DigestFinal_ex`,
`EVP_MD_CTX_free` — linked against the *same* Charger-produced `.lib` with the
*same* toolchain (clang / lld-link) **reproduces the failure**:

- `link.exe` and `lld-link` both report `OpenSSL_init_crypto` unresolved from the
  archive even though `init.obj` (member `5a08e60dddf7_init.obj`) contains a valid
  `T OpenSSL_init_crypto` (confirmed via `dumpbin /symbols` and `llvm-nm`).
- Linking the loose extracted `init.obj` directly (bypassing the archive) ALSO
  fails to resolve it under `link.exe`.
- `cdb` on the Lime smoke shows the SEGV is `call 0x0` inside `EVP_DigestInit_ex`
  -> a NULL function pointer in the SHA-256 dispatch table
  (`ossl_sha256_functions` / `EVP_sha256` provider fetch path).

Because the failure reproduces **without any Charger ABI/codegen in the path**,
it is an OpenSSL static-artifact / provider-wiring problem, not a Charger bug.

### Contributing factors observed in the generated artifact

- `lib.exe` (LNK4006) reports duplicate definitions across architecture backends
  compiled into one build: `AES_set_encrypt_key` in both `aes_core.obj` and
  `aes_x86core.obj`; `bn_mul_mont` in `bn_ppc.obj` and `bn_sparc.obj`;
  `ChaCha20_ctr32_*` / `OPENSSL_ppccap_P` (PowerPC) referenced from
  `chacha_ppc.obj` on an x86-64 build. This indicates OpenSSL's Configure step
  compiled ALL platform backends; the duplicate-symbol drops/multi-arch selection
  corrupt the provider registration / dispatch wiring that `EVP_DigestInit_ex`
  relies on at runtime.
- The original "GNU ar vs MSVC COFF .lib format" hypothesis was tested and
  REJECTED: switching the archive tool to `lib.exe` (proper MSVC COFF `.lib`)
  did NOT change the runtime SEGV (smoke still exits 139). The loose-`init.obj`
  link.exe failure was a red herring (Lime links with lld-link, not link.exe).

## What was changed in Charger (generic, kept)

`src/charger.rs`: on Windows, the native archive is now built with MSVC `lib.exe`
(proper COFF `.lib` with a linker-readable symbol index) instead of `llvm-ar`
(GNU `ar` format, which `file` reports as "current ar archive" and which some
Windows linkers mishandle). On non-Windows the GNU/ELF `llvm-ar` path is
unchanged. This is a **generic OS/toolchain-format improvement**, gated on
`cfg!(windows)` — there is NO library-specific branch (no `if library == openssl`
etc.). It was verified to keep the 6-library regression gate green and does not
fix the OpenSSL execution block.

## Prohibited (do NOT do)

- No OpenSSL-specific branch in `charger.rs` (architecture source selection,
  provider registration, static provider wiring, duplicate-implementation
  elimination, or build-system configuration).
- Do not chase the provider-dispatch NULL as a Charger ABI/codegen defect — it is
  reproduced by a pure native C caller.

## Unblock conditions (any one)

1. Build OpenSSL such that only the target-arch backend is compiled and the
   default provider + SHA-256 dispatch register correctly (environment/build
   config work, outside Charger), then re-run the native-execution smoke to
   exit 0.
2. Demonstrate via a pure native C caller (no Lime) that `EVP_DigestInit_ex` ->
   `EVP_DigestFinal_ex` completes and prints a 32-byte digest, on the SAME
   `.lib` Charger produces.

Until (1) or (2) holds, OpenSSL stays UNAVAILABLE for native execution.
