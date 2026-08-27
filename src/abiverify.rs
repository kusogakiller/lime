// abiverify: permanent ABI CONTRACT verification for Charger's Official
// Support set (Iteration 31).
//
// Role boundary (mission-critical): this module is NOT an alternative ABI
// implementation. It never parses C, never computes layouts, never touches
// Charger's internal CType representation. It compares a HUMAN-REVIEWED,
// FROZEN expected contract (`bench_clang/abi_contracts/<lib>.json`) against
// Charger's actual generated artifacts:
//
//     expected contract (frozen)  <->  lime-iface.lime + manifest.toml
//
// What it detects:
//   * signature drift      — a generated extern fn's params/return/symbol
//                            deviating from the frozen contract (catches,
//                            e.g., Iteration-27 pointer-typedef struct-by-value
//                            misclassification, Iteration-30 `unsigned char`
//                            mangling, silent retyping of parameters)
//   * dangling shim refs   — an iface-declared symbol missing from the
//                            manifest symbols list (Iteration-30 Bug B class:
//                            iface references a lime_val_/lime_ret_/lime_out_
//                            shim that was never emitted)
//   * platform drift       — contract platform vs manifest abi.triple
//   * forbidden shapes     — contract-listed substrings that must never
//                            appear in a generated interface
//   * missing artifacts    — contract-required symbols absent from manifest
//
// Entry selection: among `.lime-charger/store/<lib>/0.1.0/*/` the entry whose
// manifest.toml has the NEWEST mtime is verified. (NOTE, tracked separately:
// charger's build-time lookup currently selects by lexicographically largest
// hash string, which is time-arbitrary — recorded as a CURRENT ISSUE.)

use std::path::{Path, PathBuf};

#[derive(serde::Deserialize, Debug)]
pub struct Contract {
    pub library: String,
    #[serde(default)]
    pub version: String,
    pub platform: String,
    #[serde(default)]
    pub compiler: String,
    /// Store directory name (corpus basename). Defaults to `library`.
    #[serde(default)]
    pub store: Option<String>,
    /// Expected function signatures. An entry passes when AT LEAST ONE iface
    /// declaration with the same name matches params+return+symbol exactly
    /// (extra arities — e.g. variadic families — are allowed).
    #[serde(default)]
    pub functions: Vec<ContractFn>,
    /// Symbols that MUST exist in the manifest symbols list.
    #[serde(default)]
    pub required_symbols: Vec<String>,
    /// Substrings that must NEVER appear in the generated iface.
    #[serde(default)]
    pub forbidden_substrings: Vec<String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct ContractFn {
    pub name: String,
    #[serde(default)]
    pub params: Vec<String>,
    /// Absent / empty => expect Unit (no `-> T` clause).
    #[serde(default)]
    pub ret: Option<String>,
    /// Expected native symbol literal. Defaults to `name`.
    #[serde(default)]
    pub symbol: Option<String>,
}

#[derive(serde::Deserialize, Debug, Default)]
struct ManifestSlice {
    #[serde(default)]
    pub symbols: Vec<String>,
    #[serde(default)]
    pub abi: ManifestAbi,
    /// Native archive filename within the entry ("libjpeg.lib", ...).
    #[serde(default)]
    pub artifact: String,
}

#[derive(serde::Deserialize, Debug, Default)]
struct ManifestAbi {
    #[serde(default)]
    triple: String,
}

/// One parsed `extern fn ...` line from lime-iface.lime.
struct IfaceFn {
    name: String,
    params: Vec<String>,
    ret: String, // "Unit" when the declaration has no `-> T`
    symbol: String,
}

fn store_root() -> PathBuf {
    PathBuf::from(".lime-charger").join("store")
}

/// Newest store entry for `store_name`, selected by the manifest's
/// `installed_seq` stamp (Iteration 32). This is the SAME identity-based
/// semantics the production selector now uses (`charger.rs`'s
/// `find_artifact_entry_exact_in`): max(installed_seq), ties on largest path.
/// The previous fs-mtime probe here was an Iteration-31 diagnostic workaround
/// and is retired now that Charger records ordering metadata itself.
fn newest_entry(store_name: &str) -> Result<PathBuf, String> {
    #[derive(serde::Deserialize, Default)]
    struct SeqOnly {
        #[serde(default)]
        installed_seq: u64,
    }
    let base = store_root().join(store_name).join("0.1.0");
    let mut best: Option<(u64, String, PathBuf)> = None;
    let rd = std::fs::read_dir(&base)
        .map_err(|e| format!("store read failed for '{}': {}", base.display(), e))?;
    for h in rd.filter_map(|e| e.ok()) {
        let entry = h.path();
        if !entry.is_dir() {
            continue;
        }
        let mf = entry.join("manifest.toml");
        let seq: u64 = std::fs::read_to_string(&mf)
            .ok()
            .and_then(|t| toml::from_str::<SeqOnly>(&t).ok())
            .map(|s| s.installed_seq)
            .unwrap_or(0);
        if std::fs::metadata(&mf).is_err() {
            continue;
        }
        let key = entry.to_string_lossy().to_string();
        let newer = match &best {
            Some((bs, bk, _)) => seq > *bs || (seq == *bs && key > *bk),
            None => true,
        };
        if newer {
            best = Some((seq, key, entry));
        }
    }
    best.map(|(_, _, p)| p).ok_or_else(|| {
        format!(
            "no installed store entry for '{}' (run `charger install` first)",
            store_name
        )
    })
}

/// Parse one `extern fn NAME(params) [-> RET] "SYMBOL"` declaration.
fn parse_iface_line(line: &str) -> Option<IfaceFn> {
    let rest = line.trim().strip_prefix("extern fn ")?;
    // Quoted symbol literal is always last: locate BOTH quotes precisely.
    let q1 = rest.rfind('"')?;
    let q0 = rest[..q1].rfind('"')?;
    if q0 >= q1 {
        return None;
    }
    let symbol = rest[q0 + 1..q1].to_string();
    // Everything before the OPENING quote (exclusive).
    let head = rest[..q0].trim_end();
    let open = head.find('(')?;
    // Depth-match the parameter-list closing paren (Lime types contain their
    // own parens: `Opaque(X)` — a naive rfind(')') would stop there).
    let mut depth = 0i32;
    let mut close: Option<usize> = None;
    for (i, ch) in head.char_indices().skip(open) {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    let name = head[..open].trim().to_string();
    let params_str = &head[open + 1..close];
    let mut params = Vec::new();
    if !params_str.trim().is_empty() {
        // Top-level comma split (Lime types contain parens: Opaque(X)).
        let mut d = 0i32;
        let mut cur = String::new();
        for ch in params_str.chars() {
            match ch {
                '(' => {
                    d += 1;
                    cur.push(ch);
                }
                ')' => {
                    d -= 1;
                    cur.push(ch);
                }
                ',' if d == 0 => {
                    params.push(cur.trim().to_string());
                    cur.clear();
                }
                _ => cur.push(ch),
            }
        }
        if !cur.trim().is_empty() {
            params.push(cur.trim().to_string());
        }
    }
    // Normalize `T: aN` -> `T`; keep bare types as-is.
    let params: Vec<String> = params
        .into_iter()
        .map(|p| match p.rsplit_once(':') {
            Some((t, arg)) if arg.trim().starts_with('a') => t.trim().to_string(),
            _ => p.trim().to_string(),
        })
        .collect();
    // Optional `-> RET` between the closing paren and the symbol quote.
    let tail = head[close + 1..].trim();
    let ret = match tail.strip_prefix("->") {
        Some(r) => r.trim().to_string(),
        None => "Unit".to_string(),
    };
    Some(IfaceFn {
        name,
        params,
        ret,
        symbol,
    })
}

fn load_iface_fns(entry: &Path) -> Result<Vec<IfaceFn>, String> {
    let text = std::fs::read_to_string(entry.join("lime-iface.lime"))
        .map_err(|e| format!("cannot read lime-iface.lime: {}", e))?;
    Ok(text.lines().filter_map(parse_iface_line).collect())
}

fn load_manifest(entry: &Path) -> Result<ManifestSlice, String> {
    let text = std::fs::read_to_string(entry.join("manifest.toml"))
        .map_err(|e| format!("cannot read manifest.toml: {}", e))?;
    toml::from_str(&text).map_err(|e| format!("manifest parse failed: {}", e))
}

fn load_contract(lib: &str) -> Result<Contract, String> {
    let path = Path::new("bench_clang")
        .join("abi_contracts")
        .join(format!("{}.json", lib));
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("contract not found: {} ({})", path.display(), e))?;
    serde_json::from_str(&text)
        .map_err(|e| format!("contract JSON invalid ({}): {}", path.display(), e))
}

/// Locate `llvm-nm(.exe)` from the same environment variables the rest of
/// the toolchain uses (LIME_LLVM_BIN / LLVM_SYS_221_PREFIX / LIME_LLVM_PREFIX),
/// falling back to PATH. Returns None when unavailable — callers then skip
/// artifact-level symbol verification with a printed note (manifest-level
/// checks remain authoritative).
fn find_llvm_nm() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    for var in ["LIME_LLVM_BIN", "LLVM_SYS_221_PREFIX", "LIME_LLVM_PREFIX"] {
        if let Ok(v) = std::env::var(var) {
            let mut p = PathBuf::from(&v).join("bin");
            p.push("llvm-nm.exe");
            candidates.push(p);
            let mut q = PathBuf::from(&v);
            q.push("llvm-nm.exe");
            candidates.push(q);
        }
    }
    if let Ok(paths) = std::env::var("PATH") {
        for dir in std::env::split_paths(&paths) {
            candidates.push(dir.join("llvm-nm.exe"));
            candidates.push(dir.join("llvm-nm"));
        }
    }
    candidates.into_iter().find(|p| p.exists())
}

/// Extract the set of defined symbol names from a native archive via llvm-nm.
/// Returns None when the tool is unavailable; Err on tool failure.
fn artifact_defined_symbols(
    nm: &Path,
    lib: &Path,
) -> Result<std::collections::HashSet<String>, String> {
    let out = std::process::Command::new(nm)
        .arg(lib)
        .output()
        .map_err(|e| format!("llvm-nm launch failed: {}", e))?;
    if !out.status.success() {
        return Err(format!(
            "llvm-nm exited with {} on {}",
            out.status,
            lib.display()
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut set = std::collections::HashSet::new();
    for line in text.lines() {
        // Typical COFF archive lines: "<value> <type> <name>" or just "<name>".
        // Keep every whitespace-separated token that looks like an identifier;
        // lookups check exact membership plus an optional leading underscore.
        for tok in line.split_whitespace() {
            if tok.len() >= 2
                && tok
                    .chars()
                    .next()
                    .map(|c| c.is_alphabetic() || c == '_')
                    .unwrap_or(false)
            {
                set.insert(tok.to_string());
            }
        }
    }
    Ok(set)
}

/// True when the artifact symbol table contains `sym` (allowing the common
/// COFF leading-underscore decoration).
fn symbols_contain(set: &std::collections::HashSet<String>, sym: &str) -> bool {
    set.contains(sym) || set.contains(&format!("_{}", sym))
}

/// Verify one library's ABI contract against its newest store entry.
/// Returns Err(message) on any contract violation; Ok(report_text) on PASS.
pub fn verify_contract(lib: &str) -> Result<String, String> {
    let contract = load_contract(lib)?;
    let store_name = contract.store.clone().unwrap_or_else(|| lib.to_string());
    let entry = newest_entry(&store_name)?;

    let manifest = load_manifest(&entry)?;
    let iface = load_iface_fns(&entry)?;
    let mut problems: Vec<String> = Vec::new();
    let mut checked_functions = 0usize;

    // --- platform / toolchain identity -------------------------------------
    if !manifest.abi.triple.is_empty()
        && !manifest
            .abi
            .triple
            .to_ascii_lowercase()
            .contains(&contract.platform.to_ascii_lowercase())
        && !contract
            .platform
            .to_ascii_lowercase()
            .contains(&manifest.abi.triple.to_ascii_lowercase())
    {
        problems.push(format!(
            "platform mismatch: contract='{}' manifest.abi.triple='{}'",
            contract.platform, manifest.abi.triple
        ));
    }

    // --- forbidden substrings ------------------------------------------------
    let raw_iface = std::fs::read_to_string(entry.join("lime-iface.lime")).unwrap_or_default();
    for bad in &contract.forbidden_substrings {
        if raw_iface.contains(bad.as_str()) {
            problems.push(format!(
                "forbidden shape present in iface: \"{}\" (regression shape)",
                bad
            ));
        }
    }

    // --- function signatures -------------------------------------------------
    for want in &contract.functions {
        checked_functions += 1;
        let candidates: Vec<&IfaceFn> = iface.iter().filter(|d| d.name == want.name).collect();
        if candidates.is_empty() {
            problems.push(format!(
                "[ABI FAIL] {}\n  Function: {}\n  Reason: declared in contract but NOT present in generated iface",
                contract.library, want.name
            ));
            continue;
        }
        let want_ret = want.ret.clone().unwrap_or_else(|| "Unit".to_string());
        let want_sym = want.symbol.clone().unwrap_or_else(|| want.name.clone());
        let matched = candidates
            .iter()
            .any(|d| d.params == want.params && d.ret == want_ret && d.symbol == want_sym);
        if !matched {
            // Report the closest candidate (same name, first found) for diffing.
            let got = candidates[0];
            problems.push(format!(
                "[ABI FAIL] {}\n\nFunction: {}\n\nExpected:\n  params:\n{}\n  return:\n    {}\n  symbol:\n    {}\n\nActual:\n  params:\n{}\n  return:\n    {}\n  symbol:\n    {}\n",
                contract.library,
                want.name,
                want.params.iter().enumerate().map(|(i, p)| format!("    {}: {}", i, p)).collect::<Vec<_>>().join("\n"),
                want_ret,
                want_sym,
                got.params.iter().enumerate().map(|(i, p)| format!("    {}: {}", i, p)).collect::<Vec<_>>().join("\n"),
                got.ret,
                got.symbol,
            ));
        }
    }

    // --- dangling shim references (iface symbol must exist in manifest) -----
    let mut shim_refs_checked = 0usize;
    for d in &iface {
        shim_refs_checked += 1;
        if !manifest.symbols.contains(&d.symbol) {
            problems.push(format!(
                "[SHIM FAIL] {}\nInterface references:\n  {}\nbut the manifest symbols list does not contain it\n(dangling shim reference — emitted-artifact drift)",
                contract.library, d.symbol
            ));
        }
    }

    // --- required symbols -----------------------------------------------------
    let mut symbols_checked = 0usize;
    for req in &contract.required_symbols {
        symbols_checked += 1;
        if !manifest.symbols.contains(req) {
            problems.push(format!(
                "[SYMBOL FAIL] {}\nRequired symbol missing from manifest: {}",
                contract.library, req
            ));
        }
    }

    // --- artifact-level symbol existence (Iteration 32) ----------------------
    // Manifest-level checks prove the MANIFEST is consistent; this proves the
    // NATIVE ARCHIVE actually defines what both the iface and the manifest
    // promise ("manifest says it exists" vs ".lib contains it"). Uses llvm-nm
    // from the supported LLVM toolchain; when the tool cannot be located the
    // scan is skipped with a printed note (manifest checks remain), but a
    // MISSING symbol in a scanned archive is a hard gate failure.
    let mut artifact_symbols_checked = 0usize;
    let nm = find_llvm_nm();
    let art_path = if manifest.artifact.is_empty() {
        None
    } else {
        Some(entry.join(&manifest.artifact))
    };
    let defined = match (&nm, &art_path) {
        (Some(nm), Some(lib_path)) if lib_path.exists() => {
            match artifact_defined_symbols(nm, lib_path) {
                Ok(set) => Some(set),
                Err(e) => {
                    return Err(format!(
                        "verify-contract '{}' FAILED: artifact symbol scan failed: {}",
                        lib, e
                    ));
                }
            }
        }
        _ => None,
    };
    if let Some(set) = &defined {
        // SCOPE of the physical-symbol guarantee (Iteration 32): Charger
        // vouches for (a) every symbol in ITS OWN generated-shim namespace
        // (`lime_*`: out/take/val/ret adapters, const shims, struct/global
        // accessors, variadic families) and (b) every symbol the frozen
        // contract explicitly requires. Raw C symbols merely DECLARED by a
        // header are NOT part of that guarantee: conditional-feature APIs
        // (e.g. SQLite's SQLITE_ENABLE_COLUMN_METADATA block) and
        // transitively-included CRT declarations legitimately appear in an
        // interface without being part of the built archive — their
        // callability is owned by the corpus build configuration and is
        // proven end-to-end by the E2E phase instead. This keeps the gate
        // strict about Charger's own layer without demanding a full linker-
        // level audit of every third-party declaration.
        for d in &iface {
            if !d.symbol.starts_with("lime_") {
                continue;
            }
            artifact_symbols_checked += 1;
            if !symbols_contain(set, &d.symbol) {
                problems.push(format!(
                    "[ARTIFACT FAIL] {}\nGenerated shim declared by iface/manifest but NOT defined in the native archive: {}\narchive: {}",
                    contract.library,
                    d.symbol,
                    art_path.as_ref().map(|p| p.display().to_string()).unwrap_or_default()
                ));
            }
        }
        for req in &contract.required_symbols {
            if !symbols_contain(set, req) {
                problems.push(format!(
                    "[ARTIFACT FAIL] {}\nRequired symbol NOT defined in the native archive: {}\narchive: {}",
                    contract.library,
                    req,
                    art_path.as_ref().map(|p| p.display().to_string()).unwrap_or_default()
                ));
            }
        }
    } else {
        println!(
            "note: llvm-nm not found — artifact-level symbol scan SKIPPED for '{}' (manifest-level checks still enforced)",
            lib
        );
    }

    // --- reverse direction: unreferenced manifest symbols (WARNING only) ----
    let referenced: std::collections::HashSet<&str> =
        iface.iter().map(|d| d.symbol.as_str()).collect();
    let unreferenced = manifest
        .symbols
        .iter()
        .filter(|s| !referenced.contains(s.as_str()))
        .count();

    if !problems.is_empty() {
        let mut out = String::new();
        out.push_str(&format!(
            "verify-contract '{}' FAILED ({} problem(s))\n",
            lib,
            problems.len()
        ));
        for p in &problems {
            out.push_str(p);
            out.push('\n');
            out.push('\n');
        }
        out.push_str(&format!(
            "checked: functions={} shim_refs={} required_symbols={} artifact_symbols={} | entry={}\n",
            checked_functions,
            shim_refs_checked,
            symbols_checked,
            artifact_symbols_checked,
            entry.display()
        ));
        return Err(out);
    }

    Ok(format!(
        "PASS  {} v{} | functions={} shim_refs={} required_symbols={} artifact_symbols={} unreferenced_manifest_symbols={} (warning-only) | entry={}",
        contract.library,
        contract.version,
        checked_functions,
        shim_refs_checked,
        symbols_checked,
        artifact_symbols_checked,
        unreferenced,
        entry
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    ))
}

/// Verify a fixed list of libraries and print the aggregate report.
/// Returns (passed, failed, total_functions, total_shim_refs, total_symbols).
pub fn verify_all(libs: &[&str]) -> (usize, usize, usize, usize, usize) {
    let (mut pass, mut fail) = (0usize, 0usize);
    let (mut fns, mut refs, mut syms) = (0usize, 0usize, 0usize);
    println!("verify-abi (contract gate)");
    for lib in libs {
        match verify_contract(lib) {
            Ok(line) => {
                pass += 1;
                println!("{}", line);
                // Pull counts back out of the summary line for aggregates.
                for part in line.split('|') {
                    let p = part.trim();
                    if let Some(rest) = p.strip_prefix("functions=") {
                        fns += rest
                            .split_whitespace()
                            .next()
                            .and_then(|n| n.parse::<usize>().ok())
                            .unwrap_or(0);
                    } else if let Some(rest) = p.strip_prefix("shim_refs=") {
                        refs += rest
                            .split_whitespace()
                            .next()
                            .and_then(|n| n.parse::<usize>().ok())
                            .unwrap_or(0);
                    } else if let Some(rest) = p.strip_prefix("required_symbols=") {
                        syms += rest
                            .split_whitespace()
                            .next()
                            .and_then(|n| n.parse::<usize>().ok())
                            .unwrap_or(0);
                    }
                }
            }
            Err(e) => {
                fail += 1;
                println!("FAIL  {}", lib);
                println!("{}", e);
            }
        }
    }
    (pass, fail, fns, refs, syms)
}
