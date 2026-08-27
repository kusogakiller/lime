# Charger — Official Library Support Matrix

This document is the authoritative record of which C libraries, at which exact
versions, on which platform/ABI/toolchain, Charger **officially supports**.

Policy: support is defined per

    Library × Version × Platform × ABI × Toolchain × Build Configuration

and is granted ONLY when all eleven criteria in the project mission are met
(exact source provenance, exact version, install PASS, native artifact PASS,
Lime E2E PASS with expected output, regression-gate persistence, runtime
dependency closure, …). A green adapter build alone is NOT official support.

Status vocabulary: VERIFIED / ENVIRONMENT-BLOCKED / UNVERIFIED /
UNSUPPORTED / UNKNOWN.

Last updated: Iteration 32 (FINAL Charger baseline before returning to Lime
core). Gate: E2E PASS=22 FAIL=0 AND ABI contracts PASS=13 FAIL=0, including
artifact-level native-symbol verification (5,546 lime_* shim symbols checked
against the actual archives via llvm-nm). Store entry selection is now
identity-based (manifest installed_seq stamp), not hash-lexical order.

---

## 1. Officially Supported (VERIFIED)

Platform for every entry below:

    Platform : x86_64-pc-windows-msvc (Win64)
    ABI      : MSVC x64 calling convention
    Compiler : clang 22.1.8 (LLVM 22.1.8, Downloads toolchain)
    Archiver : lib.exe (MSVC COFF .lib) via VS 2022 BuildTools 14.44
    Host env : vcvarsall.bat x64 required for system headers

### zlib — 1.3.1  [VERIFIED]
    Source provenance : corpus src/zlib.tar.gz -> src/zlib-1.3.1/
    Version source    : directory name + runtime zlibVersion() == "1.3.1"
    Build flags       : -O2
    Dependencies      : none
    Charger commit    : current tree (post Iteration-29 fix)
    Smoke             : bench_clang/regression/zlib_smoke/zlib_smoke.lime
    Expected output   : "1.3.1" + CRC32("123456789") == 3421780262
    Gate              : PASS (run_regression.sh row 1)

### SQLite — amalgamation 3.45.1  [VERIFIED]
    Source provenance : corpus sqlite3.c/sqlite3.h/shell.c (amalgamation drop)
    Version source    : SQLITE_VERSION "3.45.1" (sqlite3.h:149)
    Build flags       : -O2 -DSQLITE_ENABLE_UNLOCK_NOTIFY
                        -DSQLITE_ENABLE_SNAPSHOT -DSQLITE_ENABLE_RTREE
    Dependencies      : none
    Smoke             : sqlite_smoke — expects 3045001 (version number) + 1
    Gate              : PASS (row 3)

### curl — 8.10.1  [VERIFIED]
    Source provenance : corpus include/curl + lib/ (curl-8.10.1 tree present)
    Version source    : LIBCURL_VERSION "8.10.1" (include/curl/curlver.h:35);
                        runtime curl_version() == "libcurl/8.10.1"
    Build flags       : -O2 -DBUILDING_LIBCURL ; exclude ldap.c (corpus cfg)
    Dependencies      : none linked external
    Smoke             : curl_smoke — version string, rc, error buffer
    Gate              : PASS (row 5)

### SDL2 — 2.30.9 (Windows TU subset)  [VERIFIED]
    Source provenance : C:/Users/szzxl/Downloads/lime_corpus_src/SDL2-2.30.9-win
                        (Windows-only corpus copy: generic thread TUs removed,
                        mirrors upstream CMake selection; see CABI_CAPABILITY.md
                        Iteration 15 note — this is SUPPORT CONFIGURATION, not a
                        generic Charger capability)
    Build flags       : corpus config only
    Smoke             : sdl2_smoke — headless Init(0), platform string, quit
    Gate              : PASS (row 6)

### FFmpeg libavutil API slice — 60.26.102  [VERIFIED as SLICE ONLY]
    Scope       : version API only (avutil_version / av_version_info)
    Version     : LIBAVUTIL 60.26.102 (libavutil/version.h)
    Build flags : -O2 -I. (+ corpus stub config.h)
    Smoke       : ffmpeg_smoke — expects 3938918 (=0x3C1A66 → 60.26.102) +
                  version info string
    Gate        : PASS (row 7)
    NOT CLAIMED : full FFmpeg support. Only the slice listed here.

### FFmpeg media-object slice (avcodec/avutil) — avcodec 62.28.102  [VERIFIED as SLICE ONLY]
    Scope       : AVPacket/AVFrame/AVCodecParameters lifecycle
                  (alloc/free/unref take+free via T**)
    Version     : LIBAVCODEC 62.28.102 (media_objects corpus tree;
                  avformat slice tree additionally carries LIBAVFORMAT
                  62.12.102 but is not exercised by the smoke)
    Build flags : -O2 -I. -idirafter libavutil -include config.h (corpus cfg)
    Smoke       : ffmpeg_media_objects_smoke — PKT_OK|FRM_OK|PAR_OK
    Gate        : PASS (row 8)

### libpng — 1.6.50  [VERIFIED as of Iteration 29]
    Source provenance : corpus src/libpng-1.6.50/ (+ deps zlib-1.3.1)
    Version source    : directory name; runtime png_access_version_number()
                        == 10650 (== 1.6.50) asserted by png_smoke
    Build flags       : -O2 ; deps = ["zlib-1.3.1"]
    Dependencies      : zlib 1.3.1 (STATIC .lib closure; no DLL needed —
                        both artifacts built statically by Charger)
    Note              : previously mis-reported ENVIRONMENT-BLOCKED; true root
                        cause was a Charger bug (see §4 Iteration 29 entry).
    Smoke             : png_smoke — expects 10650
    Gate              : PASS (row 2)

### libjpeg-turbo — 3.1.0  [VERIFIED as of Iteration 29]
    Source provenance : corpus src/libjpeg-turbo-3.1.0/
    Version source    : directory name (src/ tree); smoke asserts
                        JPEG_LIB_VERSION 90 + turbo version 568 + create/
                        destroy round-trip ("CREATE_DESTROY_OK")
    Build flags       : -O2
    Dependencies      : none
    Smoke             : jpeg_smoke
    Gate              : PASS (row 4)

---

## 2. Synthetic ABI fixtures (regression-only — NEVER counted as library support)

These exist to keep ABI mechanisms under continuous test. They do not extend
Official Library Coverage.

    libcallbackarg        callback argument (fn-ptr param)          PASS
    libcallbackreturn     typedef callbacks w/ non-void returns     PASS
    anon_flatten          anonymous struct/union flattening         PASS
    libenumedge           enum width + fixed-underlying constants   PASS
    libenumedge_nanchor   same, non-anchor form                     PASS
    libpackedbitfield     packed + bitfield composite               PASS
    libfamprobe           flexible array member                     PASS
    libfreshprobe         pointer-typedef handle (struct pointee)   PASS
    libcolprobe           minimal opaque handle                     PASS
    libvarargedge         shaped variadic edges                     PASS
    libubi                bitfields inside unions                   PASS
    libptrtypedef         NEW Iter29: pointer typedef whose POINTEE is a
                          scalar typedef (`typedef byte_t *bytep;`) — keeps
                          the stack-overflow regression under cover  PASS
    libpackedanon         NEW Iter29: packed outer + NESTED ANONYMOUS struct
                          and nested anonymous BITFIELD struct — full Lime E2E
                          of flattened members                       PASS
    libparamtypes         NEW Iter30: multi-token base-type parameters
                          (`unsigned char` / `signed char` / `unsigned int`)
                          through RAW C functions + struct by-value return and
                          argument wrappers (lime_ret_/lime_val_)      PASS

Gate totals (Iteration 32, clean-store rebuild):
* E2E phase: PASS=22 FAIL=0
* ABI contract phase: PASS=13 FAIL=0
  (contracts=13, functions checked=56, shim references checked=8807,
   required symbols checked=50, artifact-level lime_* symbols checked=5546
   — measured values from the integrated gate)

---

## 2b. ABI Contract Gate (Iteration 31)

`run_regression.sh` now runs TWO phases and fails as a whole if either fails:

    Phase 1  E2E            install -> build -> link -> execute per library
    Phase 2  verify-abi     frozen expected contract vs generated artifacts

Contracts live in `bench_clang/abi_contracts/<store-name>.json`. They are the
Source of Truth for the ABI surface: NEVER regenerate them from Charger
output. Each pins library / exact version / platform / compiler, a critical
function-signature surface (params + return + native symbol), required
symbols, and forbidden historical shapes.

Verifier (`lime charger verify-contract <lib>...`, generic, in
src/abiverify.rs) checks, per library:
1. platform vs manifest abi.triple
2. every contract function has >=1 iface decl matching params+ret+symbol
3. EVERY iface-declared symbol exists in manifest.symbols
   (dangling-shim detection — Iteration-30 Bug B class)
4. required symbols present
5. forbidden substrings absent from the iface

Historical regression shapes permanently covered by contracts:
* Iteration 27 — pointer typedef handle must stay Pointer/Opaque
  (`libptrtypedef`: ptd_make returns Int-sized handle, NOT struct-by-value)
* Iteration 29 — scalar-pointer typedef must not self-loop / crash
  (`libptrtypedef`, `libfreshprobe`: fresh_hmake -> Opaque(FreshHandle_))
* Iteration 30 — multi-token base-type params keep their width
  (`libparamtypes`: ptt_uc/sc/ui Int params with DIRECT symbols;
   forbidden substrings Opaque(unsigned)/Opaque(signed))
* Iteration 30 Bug B — no dangling lime_val_/lime_ret_/lime_out_/lime_take_
  references (forbidden-substring guards on libpng/libjpeg/libpackedanon +
  universal dangling check)
* take/free T** lifecycle (`ffmpeg_media_objects`: lime_take_av_*_free)
* out-param adapters (`sqlite`: sqlite3_open -> Opaque(sqlite3) via
  lime_out_sqlite3_open)
* shaped variadic families (`libvarargedge`: vae_sum arities pinned to
  lime_vae_sum_vN symbols)

---

## 3. Known candidates NOT yet officially supported

| Library | Status | Blocker |
|---------|--------|---------|
| OpenSSL 3.3.2 | UNSUPPORTED (documented OPENSSL_STATUS.md) | environment/build config; revisit only on explicit demand |
| Any Linux/macOS/ARM target | UNVERIFIED | zero empirical runs off Win64/MSVC; Win64 PASS does NOT transfer |
| Version ranges (e.g. zlib ≥1.3) | not offered | policy: exact versions only until multiple versions independently pass |

---

## 4. Change log affecting support

### Iteration 29 (this session)
* FIXED Charger bug: SCALAR_TYPEDEFS rebuild published self-loop entries
  (`X -> "X"`); Iteration-27's pointer-alias pass fed such names into
  parse_c_type → infinite recursion → main-thread stack overflow.
  Triggered by pointer typedefs whose POINTEE is itself a scalar typedef —
  exactly libpng (`typedef png_byte * png_bytep;`) and libjpeg-turbo
  (JSAMPROW family). Both move ENVIRONMENT-BLOCKED → **VERIFIED**.
* libpackedanon: full Lime E2E proven (PACKED_ANON_OK); UNVERIFIED → gate
  persistence added (support-category fixture, not a real library).
* Gate 17/2 → **21/0**.

### Iteration 30 (hardening)
* FIXED Charger bug: `strip_param_name` stripped the final token of
  multi-token base-type parameter spellings (`unsigned char` → `unsigned`,
  `signed char` → `signed`, `unsigned int` → `unsigned`, `long double` →
  `long`). clang function-type qualTypes carry NO parameter names (measured:
  `"void (int, unsigned char)"`), so any trailing identifier-pop is purely
  defensive; it now refuses to pop C base-type keywords/qualifiers.
  Consequence eliminated: mangled scalar spellings no longer become
  `Other("unsigned")` CType / `Opaque(unsigned)` Lime params.
* FIXED Charger bug (consistency): `lime_shim_symbol` and the adapter emitter
  now share ONE by-value predicate (`is_byval_record_ty`) and iface
  generation publishes the same KNOWN_RECORDS set as adapter emission —
  an iface can no longer reference a `lime_ret_*`/`lime_val_*` shim that was
  never emitted into the artifact (the dangling-symbol class).
  Side effect: libpackedanon's raw setters (`pka_set_head/x/tail`,
  `pka_bit_set_*`) are now correctly callable via their real symbols.
* New fixture libparamtypes keeps both fixes under permanent gate cover.
* Gate 21/0 → **22/0**. No previously-passing entry regressed.

### Iteration 31 (ABI contract gate + consistency hardening)
* NEW: permanent ABI contract gate — `bench_clang/abi_contracts/*.json`
  (13 frozen contracts) + generic verifier `src/abiverify.rs` exposed as
  `lime charger verify-contract`, wired into `run_regression.sh` Phase 2.
  Gate now fails on ANY ABI-surface drift in the Official Support set.
* FIXED Charger bug (found BY the new gate): callback-table setter shims
  (`lime_set_<struct>_<field>` / `_null` for function-pointer fields) were
  emitted into artifacts but never registered in manifest symbols —
  dangling for every library exposing a callback table (measured: libjpeg
  jpeg_memory_mgr/error_mgr, SQLite sqlite3_io_methods/vfs, FFmpeg AVClass).
* FIXED Charger bug (found BY the new gate): GLOBAL-variable accessors
  (`lime_get_/lime_set_<global>` + array `_i` + struct-global per-field)
  were emitted but never registered — hundreds of dangling refs on curl
  (Windows GUID globals) and SDL2.
* FIXED Charger bug (found BY the new gate): iface/emitter divergence for
  ARRAY fields inside callback tables and multi-dimensional arrays — iface
  declared whole-array setters that no C shim ever implemented
  (FFmpeg AVCodecParser.codec_ids, AVPanScan.position). Iface now routes
  arrays through emit_field_accessors (the _i shims) exactly like C side.
* KNOWN ISSUE recorded (RESOLVED in Iteration 32 — see change log above):
  store entry selection previously used lexicographic hash ordering.
* KNOWN ISSUE recorded (not fixed, tracked): macro-expanded declarations from
  transitively-included system headers (CRT functions, GUID globals) still
  surface in generated interfaces as declarations. They are excluded from the
  artifact-level physical-symbol guarantee by design; pruning them from
  ifaces is a future hygiene improvement.
* Gate: E2E 22/0 AND ABI 13/0.

### Iteration 32 (store selection hardening — FINAL Charger baseline)
* FIXED Charger bug: store-entry selection used the LEXICOGRAPHICALLY
  LARGEST version/hash path string. A tool-hash is a content digest, so its
  lexical order carries no chronology — a STALE artifact from an old Charger
  binary could silently win lookups in `lime build` / manifest resolution
  (measured during Iteration-31 investigation on libpng).
* NEW selection semantics (identity-based, deterministic): every manifest now
  records `installed_seq` (millis at install write time, bumped on cache-hit
  re-installs). All selection sites (`find_artifact_entry_exact_in`,
  `lookup_artifacts_for_symbols`, contract-gate entry picker) select
  max(installed_seq), ties broken by largest path — a total order.
  Legacy manifests deserialize as seq=0 and always lose to stamped entries.
* Unit tests (production selector exercised directly):
  stamp-wins-over-lexical-max / legacy-loses / deterministic-repeat /
  empty-store / total-ordering comparator — 5/5 PASS.
* NEW artifact-level verification: the ABI gate now runs llvm-nm over each
  archive and HARD-FAILS when any Charger-generated shim (`lime_*` namespace)
  or contract-required symbol is not physically defined. Raw C declarations
  that merely appear in an interface (conditional-feature APIs, transitive
  CRT decls) are explicitly OUT of the physical-symbol guarantee — their
  callability is owned by corpus build configuration and proven by E2E.
* Clean-store rebuild performed: `.lime-charger/store` wiped and all 13
  corpora reinstalled through the corrected selection path before final gate.
* Gate after clean rebuild: E2E 22/0 AND ABI 13/0 (artifact symbols 5,546).

### CURRENT ISSUES recorded (NOT fixed this session — tracked, do not hide)
* Charger: ~~`strip_param_name` type-token loss~~ **FIXED in Iteration 30**
  (see change log; regression fixture libparamtypes).
* Lime: hexadecimal integer literals (0x…) are rejected by the frontend —
  call sites using them produce misleading "unknown function" type errors.
  (Category B: Lime bug, not Charger.)
* Lime: calling a REAL C function that returns a sub-8-byte SIGNED int may
  read implementation-defined upper-RAX bits on Win64 depending on codegen;
  generated accessor shims widen returns to long long and are always safe.
  Smokes should prefer shims for narrow-return functions.

---

## 5. Reproduction

    cmd /c run_regression_with_vcvars.bat        # uses Git Bash internally
    # or, with MSVC env active:
    bash bench_clang/regression/run_regression.sh

Expected tail:
    E2E: PASS=22 FAIL=0
    ABI: PASS=13 FAIL=0
