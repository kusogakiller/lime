// Charger: universal C/C++ native dependency preparation layer for Lime.
//
// Architecture (see Charger mission spec):
//   charger install <library>
//     -> source/package acquisition
//     -> C/C++ build (clang / clang++)
//     -> C/C++ analysis (clang AST JSON)
//     -> ABI / symbol resolution
//     -> adapter generation (mangled -> callable)
//     -> native artifact generation
//     -> Lime interface generation
//     -> artifact store
//   lime build
//     -> prepared Charger artifact lookup
//     -> Lime compilation
//     -> native linking (prepared artifact injected)
//
// Design constraints honored:
//   * No C/C++ parser is written here. clang is the source of truth
//     (`-Xclang -ast-dump=json -fsyntax-only`).
//   * AST extraction, normalization, and Lime interface generation are
//     separate responsibilities (no serde_json::Value spaghetti).
//   * Lime ABI, memory model, String/list representation, and runtime.c ABI
//     are NOT changed. Charger only (a) adds `extern` function declarations
//     to the Lime language (minimal FFI) and (b) injects prepared native
//     artifacts into the existing link step.
//   * The same store entry is never rebuilt for identical inputs.

use std::collections::HashMap;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

// ----------------------------------------------------------------------------
// Normalized representation (Charger Internal Representation, "CIR lite")
// ----------------------------------------------------------------------------
// This is intentionally small. It captures exactly what the vertical slice
// needs and is structured so future C++ features (templates, inheritance,
// virtual, RTTI, STL, exceptions, ABI metadata) extend the same shapes
// rather than requiring a rewrite.

#[derive(Debug, Clone)]
pub enum CType {
    Int,
    Long,
    Float,
    Double,
    Bool,
    Void,
    String,        // char* / const char*
    Pointer(Box<CType>),
    Function(Vec<CType>, Box<CType>), // function pointer: fn(params) -> ret
    Struct(String), // named struct/class
    Opaque(String), // typedef / named but fields unknown
    Other(String),  // fallback: raw C type text
}

#[derive(Debug, Clone)]
pub struct CParam {
    pub name: String,
    pub ty: CType,
}

#[derive(Debug, Clone)]
pub struct CFunction {
    pub name: String,        // source name (e.g. "add", "Widget::area")
    pub symbol: String,      // mangled/linkable symbol (e.g. "add",
                              //   "?area@Widget@@QEBAHXZ")
    pub params: Vec<CParam>,
    pub ret: CType,
    pub is_method: bool,
    pub is_constructor: bool,
    pub is_const: bool,
    pub self_ty: Option<String>, // for methods: the class name
}

#[derive(Debug, Clone)]
pub struct CStruct {
    pub name: String,
    pub fields: Vec<CParam>, // (field_name, type)
    pub size_bytes: Option<u64>,
    pub align_bytes: Option<u64>,
    // Phase 1 C ABI completeness: exact-layout markers. `is_union` records a C
    // `union` (all members overlap at offset 0); `is_bitfield` records a struct
    // containing at least one bitfield member. Both are surfaced to Lime as
    // `Opaque(Name)` handles with generated C accessor shims, because Lime's
    // `int` lowers to i64 (8 bytes) and cannot replicate sub-8-byte C layouts.
    pub is_union: bool,
    pub is_bitfield: bool,
    // Whether every field is an 8-byte-wide type (long long / double /
    // pointer). When true the struct can be modeled as a real Lime struct;
    // otherwise it must be surfaced as an opaque handle with accessor shims.
    pub all_8byte: bool,
    // Whether any field is a function pointer (CType::Function). Such structs
    // are always surfaced as an opaque handle with setter shims that store
    // native function pointers — Lime callbacks round-trip through the C table.
    pub has_fn_ptr: bool,
}

#[derive(Debug, Clone, Default)]
pub struct NormalizedApi {
    pub functions: Vec<CFunction>,
    pub structs: Vec<CStruct>,
    // Phase 1 C ABI completeness: compile-time constants surfaced to Lime as
    // `const NAME = VALUE`. Sources: C `enum` enumerators, file-scope
    // `static const` variables, and `#define` integer macros. These reuse the
    // existing Lime `const` statement (no new Lime type category is introduced).
    pub constants: Vec<(String, i64)>,
    pub kind: ApiKind, // C or Cpp
    // Real-world Phase A: the set of type names that are *record/handle* types
    // (`sqlite3`, `sqlite3_stmt`, `sqlite3_blob`, … — i.e. `typedef struct X X;`
    // or `class X`). These are surfaced to Lime as `Opaque(X)` handles (bare
    // pointers). SCALAR typedefs (`sqlite3_int64`, `sqlite3_uint64`, …) are NOT
    // in this set, so the adapter C-shim generator can render them as plain
    // scalars (e.g. `sqlite3_int64`) instead of wrongly adding a `*`.
    pub handle_types: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApiKind {
    #[default]
    C,
    Cpp,
}

// ----------------------------------------------------------------------------
// AST extraction
// ----------------------------------------------------------------------------

/// Run `clang -Xclang -ast-dump=json -fsyntax-only <header>` and return the
/// parsed JSON value. Uses clang from the detected LLVM toolchain.
fn extract_ast_json(header: &Path, lang: ApiKind, llvm_bindir: &str) -> Result<serde_json::Value, String> {
    let clang = if lang == ApiKind::Cpp {
        PathBuf::from(llvm_bindir).join("clang++.exe")
    } else {
        PathBuf::from(llvm_bindir).join("clang.exe")
    };
    let clang = if clang.exists() {
        clang
    } else {
        PathBuf::from(llvm_bindir).join(if lang == ApiKind::Cpp { "clang++" } else { "clang" })
    };

    let mut cmd = Command::new(&clang);
    cmd.arg("-Xclang").arg("-ast-dump=json").arg("-fsyntax-only");
    if lang == ApiKind::Cpp {
        cmd.arg("-std=c++17");
    }
    cmd.arg(header);
    let out = cmd.output().map_err(|e| format!("AST extraction failed: clang launch error: {}", e))?;
    if !out.status.success() {
        return Err(format!(
            "AST extraction failed: clang exited with {}\nstderr:\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let json_text = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(json_text.trim())
        .map_err(|e| format!("AST JSON parse error: {}", e))
}

// ----------------------------------------------------------------------------
// Normalization: clang AST JSON -> NormalizedApi
// ----------------------------------------------------------------------------

/// Walk the clang AST JSON and collect functions/structs relevant to the slice.
fn normalize(ast: &serde_json::Value, lang: ApiKind) -> NormalizedApi {
    let mut api = NormalizedApi {
        functions: Vec::new(),
        structs: Vec::new(),
        kind: lang,
        handle_types: BTreeSet::new(),
        constants: Vec::new(),
    };
    let mut ctx = NormalizeCtx {
        anon_struct: None,
        typedefs: Vec::new(),
        _pending_method_self: None,
        handle_types: BTreeSet::new(),
    };
    if let Some(root) = ast.get("inner") {
        if let Some(arr) = root.as_array() {
            for node in arr {
                classify_node(node, &mut api, &mut ctx, None, lang);
            }
        }
    }
    // Resolve anonymous-struct typedefs: e.g. `typedef struct { ... } Point;`
    // produces an anonymous RecordDecl (no name) followed by a TypedefDecl
    // whose underlying type is `struct Point`. Bind the fields to `Point`.
    for (tname, underlying) in &ctx.typedefs {
        if let Some(stripped) = underlying.strip_prefix("struct ") {
            // Only bind when the typedef name matches the struct tag in the
            // underlying type (e.g. `typedef struct {...} Point` -> `struct Point`).
            if stripped == tname && !api.structs.iter().any(|s| &s.name == tname) {
                if let Some(fields) = &ctx.anon_struct {
                    let all_8 = fields.iter().all(|f| matches!(
                        f.ty,
                        CType::Long | CType::Double | CType::Pointer(_) | CType::Function(..) | CType::Opaque(_)
                    ));
                    let has_fp = fields.iter().any(|f| matches!(f.ty, CType::Function(..)));
                    api.structs.push(CStruct {
                        name: tname.clone(),
                        fields: fields.clone(),
                        size_bytes: None,
                        align_bytes: None,
                        is_union: false,
                        is_bitfield: false,
                        all_8byte: all_8,
                        has_fn_ptr: has_fp,
                    });
                }
            }
        }
    }
    // Resolve typedef aliases: a typedef whose underlying type is an enum
    // (e.g. `typedef enum { ... } Color;`) should be surfaced as `Int` so the
    // Lime iface uses `Int` where the C API expects the enum. Build the set of
    // enum-typedef names, then rewrite `CType::Other(name)` occurrences.
    let enum_aliases: std::collections::HashSet<String> = ctx
        .typedefs
        .iter()
        .filter(|(_, u)| u.contains("enum "))
        .map(|(n, _)| n.clone())
        .collect();
    if !enum_aliases.is_empty() {
        for f in &mut api.functions {
            for p in &mut f.params {
                if let CType::Other(n) = &p.ty {
                    if enum_aliases.contains(n) {
                        p.ty = CType::Int;
                    }
                }
            }
            if let CType::Other(n) = &f.ret {
                if enum_aliases.contains(n) {
                    f.ret = CType::Int;
                }
            }
        }
    }
    api
}

/// Given the fields of a union, keep only the widest member so the Lime struct
/// spelling has the same value-type ABI width as the C union (union size ==
/// largest member size). Width is estimated from the C type spelling.
fn widest_member(fields: Vec<CParam>) -> Vec<CParam> {
    if fields.is_empty() {
        return fields;
    }
    let width_of = |t: &CType| -> usize {
        match t {
            CType::Long | CType::Double | CType::Pointer(_) | CType::Function(..) | CType::Opaque(_) => 8,
            CType::Int | CType::Float | CType::Bool => 4,
            CType::Struct(_) | CType::Other(_) => 8, // be conservative
            CType::Void | CType::String => 8,
        }
    };
    let mut best = &fields[0];
    let mut best_w = width_of(&fields[0].ty);
    for f in fields.iter().skip(1) {
        let w = width_of(&f.ty);
        if w >= best_w {
            best = f;
            best_w = w;
        }
    }
    vec![CParam { name: best.name.clone(), ty: best.ty.clone() }]
}

/// Scan a C header source for simple integer object-like macros of the form
/// `#define NAME <integer>` and append them to `out`. Macros are preprocessor
/// text and are absent from the clang AST JSON, so this textual scan is the
/// only recovery path. Non-integer macros (e.g. function-like macros, string
/// literals) are intentionally skipped — they cannot be surfaced as Lime consts.
fn extract_macro_constants(header: &Path, out: &mut Vec<(String, i64)>) {
    let Ok(src) = std::fs::read_to_string(header) else { return; };
    // Avoid re-adding a constant already discovered via EnumConstantDecl / VarDecl.
    let mut known: std::collections::HashSet<String> = out.iter().map(|(n, _)| n.clone()).collect();
    for line in src.lines() {
        let line = line.trim_start();
        if !line.starts_with("#define") {
            continue;
        }
        // `#define NAME VALUE` — split off the directive, then on whitespace.
        let rest = &line["#define".len()..];
        let mut parts = rest.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let Some(val_str) = parts.next() else { continue };
        // Only object-like macros with a single integer value.
        if name.contains('(') || parts.next().is_some() {
            continue;
        }
        if let Ok(v) = val_str.parse::<i64>() {
            if known.insert(name.to_string()) {
                out.push((name.to_string(), v));
            }
        }
    }
}

fn type_from_json(t: &serde_json::Value) -> CType {
    // t is a "qualType" string like "int", "Point", "int (*)(int, int)",
    // "char *", "const char *", "class Widget", "struct Point", etc.
    let qual = t.get("qualType").and_then(|v| v.as_str()).unwrap_or("");
    parse_c_type(qual)
}

fn parse_c_type(qual: &str) -> CType {
    let q = qual.trim();
    // Strip leading `const ` qualifiers (e.g. `const Padded *` -> `Padded *`).
    // `const`-ness is an ABI-irrelevant qualifier for our purposes; the pointee
    // type name is what Lime surfaces (as `Opaque(Name)` / `Struct(Name)`).
    let q = q.trim_start_matches("const ").trim();
    // Task #1: function pointer types such as `long long (*)(long long, long long)`
    // or `int (*fn)(int, int)`. Parse into `CType::Function` so the Lime iface
    // emits a `fn(...) -> ...` type (consumable as a C callback pointer).
    if q.contains("(*") {
        if let Some(ft) = parse_c_function_ptr(q) {
            return ft;
        }
    }
    // normalize pointers
    if let Some(inner) = q.strip_suffix('*') {
        let inner = inner.trim();
        // function pointer: "int (*)(int, int)" => skip detailed parse
        if inner.contains("(*") {
            return CType::Pointer(Box::new(CType::Other(q.to_string())));
        }
        let pointee = parse_c_type(inner);
        // Task #2: a pointer to a type whose layout Lime does not model
        // (`struct X*`, `class X*`, a typedef'd record like `Counter*`, or
        // `void*`) is surfaced to Lime as an OPAQUE HANDLE. The Lime side keeps
        // it as a bare address (`ptr`) and hands it straight back to the native
        // side, so mutation performed through the pointer in C is observable.
        // Simplifying such a pointer to its pointee (the pre-#2 behavior) would
        // hand Lime a by-value aggregate copy and lose the indirection.
        //
        // Scalar pointers (`int*`, `double*`, `char*`) keep the existing
        // `CType::Pointer` mapping so no established behavior changes.
        match &pointee {
            CType::Void => return CType::Opaque("void".to_string()),
            // Real-world Phase A: a *pointer to an opaque/named type* (`sqlite3 *`,
            // `Widget *`, …) is a single-level handle and stays `Opaque(name)`
            // (unchanged ABI / iface text). But a *pointer to a pointer* to an
            // opaque type (`sqlite3 **`, `void **`) is the classic C out-param
            // idiom: the callee writes the handle through the extra level of
            // indirection. We must NOT collapse `sqlite3 **` into
            // `Opaque(sqlite3)` — that loses the second level and makes the
            // out-param indistinguishable from a plain handle.
            //
            // So a pointer whose pointee is already `Opaque(name)` is surfaced
            // as `Pointer(Opaque(name))`, preserving the double indirection.
            // `lime_type_name` renders both `Opaque(name)` and
            // `Pointer(Opaque(name))` as `Opaque(name)`, so this does NOT change
            // any existing single-pointer iface text. The out-param→return-value
            // adapter (see `collect_out_param_adapters`) matches exactly
            // `Pointer(Opaque(name))`.
            CType::Opaque(name) => return CType::Pointer(Box::new(CType::Opaque(name.clone()))),
            CType::Struct(name) | CType::Other(name) => {
                return CType::Opaque(name.clone());
            }
            _ => {}
        }
        return CType::Pointer(Box::new(pointee));
    }
    match q {
        "int" | "unsigned int" | "short" | "unsigned short" | "long" | "unsigned long"
        | "long long" | "unsigned long long" | "size_t" | "int32_t" | "uint32_t"
        | "int64_t" | "uint64_t" => CType::Int,
        "long" => CType::Long,
        "float" => CType::Float,
        "double" => CType::Double,
        "bool" | "_Bool" => CType::Bool,
        "void" => CType::Void,
        // Single `char` is a 1-byte scalar -> Lime Int (i64). Only `char *` /
        // `const char *` are C strings (Lime String).
        "char" | "const char" | "signed char" | "unsigned char" => CType::Int,
        "char *" | "const char *" => CType::String, // treat C strings as Lime String
        s if s.starts_with("struct ") => CType::Struct(s["struct ".len()..].to_string()),
        s if s.starts_with("class ") => CType::Struct(s["class ".len()..].to_string()),
        s if s.starts_with("enum ") => CType::Int, // enums are ABI-compatible with int
        s if s.starts_with("typedef ") => CType::Opaque(s["typedef ".len()..].to_string()),
        s => CType::Other(s.to_string()),
    }
}

/// Parse a C function-pointer type string into `CType::Function`.
///
/// Handles the common spellings emitted by clang's `qualType`:
///   `long long (*)(long long, long long)`   (no name)
///   `int (*fn)(int, int)`                    (named pointer)
/// Returns `None` if the string is not a recognizable function-pointer type.
fn parse_c_function_ptr(q: &str) -> Option<CType> {
    // Locate the `(*` that marks a function pointer.
    let star = q.find("(*")?;
    // Return type is everything before the `(` that opens the `(*`.
    let ret_part = q[..star].trim();
    // Strip any trailing `*` (pointer-to-function-pointer) and whitespace.
    let ret_part = ret_part.trim_end_matches('*').trim();
    let ret = parse_c_type(ret_part);
    // After `(*`, find the `)` that closes the pointer group, then the `(`
    // that opens the parameter list, then its matching `)`.
    let after_star = &q[star + 2..];
    let ptr_close = after_star.find(')')?;
    let rest = &after_star[ptr_close + 1..];
    let params_open = rest.find('(')?;
    let params_str = &rest[params_open + 1..];
    let params_close = params_str.rfind(')')?;
    let inner = &params_str[..params_close];
    let mut params = Vec::new();
    if !inner.trim().is_empty() {
        for p in split_top_level(inner) {
            let p = p.trim();
            let ty_str = strip_param_name(p);
            params.push(parse_c_type(ty_str.trim()));
        }
    }
    Some(CType::Function(params, Box::new(ret)))
}

/// Mutable state threaded through AST normalization
/// anonymous record bodies with the typedef names that name them, and track
/// the enclosing class for inline methods.
struct NormalizeCtx {
    anon_struct: Option<Vec<CParam>>,
    typedefs: Vec<(String, String)>,
    _pending_method_self: Option<String>,
    // Real-world Phase A: type names that are record/handle types (surfaced
    // to Lime as `Opaque(X)` handles). Distinguished from scalar typedefs.
    handle_types: BTreeSet<String>,
}

fn classify_node(
    node: &serde_json::Value,
    api: &mut NormalizedApi,
    ctx: &mut NormalizeCtx,
    self_ty: Option<String>,
    lang: ApiKind,
) {
    let kind = node.get("kind").and_then(|v| v.as_str()).unwrap_or("");

    match kind {
        "RecordDecl" => {
            let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let is_implicit = node.get("isImplicit").and_then(|v| v.as_bool()).unwrap_or(false);
            let tag_used = node.get("tagUsed").and_then(|v| v.as_str()).unwrap_or("");
            let is_union = tag_used == "union";
            let mut fields = Vec::new();
            let mut seen_bitfield = false;
            if let Some(inner) = node.get("inner").and_then(|v| v.as_array()) {
                for f in inner {
                    if f.get("kind").and_then(|v| v.as_str()) == Some("FieldDecl") {
                        let fname = f.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let fty = f.get("type").map(type_from_json).unwrap_or(CType::Other("?".to_string()));
                        // A bitfield member carries a `bitWidth` in the AST.
                        if f.get("bitWidth").is_some() {
                            seen_bitfield = true;
                        }
                        if !fname.is_empty() {
                            fields.push(CParam { name: fname, ty: fty });
                        }
                    }
                }
            }
            // For a union, only the largest member needs to survive in the Lime
            // struct spelling so the value-type ABI width matches (a union's size
            // is its largest member). Keep the widest scalar/ptr member; fall back
            // to the last field if nothing obvious stands out.
            if !name.is_empty() && !is_implicit {
                // A union is surfaced as an opaque handle with accessor shims
                // (Lime cannot model overlapping members or sub-8-byte fields),
                // so keep ALL members for the shim generator rather than
                // collapsing to the widest member.
                let kept_fields = if is_union { fields.clone() } else { fields.clone() };
                let all_8 = kept_fields.iter().all(|f| matches!(
                    f.ty,
                    CType::Long | CType::Double | CType::Pointer(_) | CType::Function(..) | CType::Opaque(_)
                ));
                let has_fp = kept_fields.iter().any(|f| matches!(f.ty, CType::Function(..)));
                api.structs.push(CStruct {
                    name: name.clone(),
                    fields: kept_fields,
                    size_bytes: None,
                    align_bytes: None,
                    is_union,
                    is_bitfield: seen_bitfield,
                    all_8byte: all_8,
                    has_fn_ptr: has_fp,
                });
            } else if !is_implicit {
                // Anonymous record body (e.g. `typedef struct { ... } Point;` or
                // `typedef union { ... } Variant;`) — remember for a TypedefDecl.
                ctx.anon_struct = Some(fields);
            }
            if let Some(inner) = node.get("inner").and_then(|v| v.as_array()) {
                for c in inner {
                    classify_node(c, api, ctx, Some(name.clone()), lang);
                }
            }
        }
        "EnumDecl" => {
            // Enum enumerators are `EnumConstantDecl` children. clang stores the
            // integer value inside a `ConstantExpr` wrapper (its `value` field is
            // a string like "1"), not directly on the EnumConstantDecl. Collect
            // them as Lime constants so the enum's named values are usable from
            // Lime. The enum type itself is surfaced as `Int` (ABI-compatible),
            // not as a separate Lime type.
            if let Some(inner) = node.get("inner").and_then(|v| v.as_array()) {
                for e in inner {
                    if e.get("kind").and_then(|v| v.as_str()) == Some("EnumConstantDecl") {
                        let ename = e.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        if ename.is_empty() {
                            continue;
                        }
                        let val = e
                            .get("inner")
                            .and_then(|v| v.as_array())
                            .and_then(|a| a.first())
                            .and_then(|c| c.get("value").and_then(|v| v.as_str()))
                            .and_then(|s| s.parse::<i64>().ok());
                        if let Some(v) = val {
                            api.constants.push((ename, v));
                        }
                    }
                }
            }
        }
        "MacroDefinition" => {
            // Macros are NOT in the AST JSON (preprocessor text); this arm is
            // only reached if a future extraction path injects them. The real
            // macro parse happens in `extract_macro_constants` over the header
            // source. Kept as a no-op guard so the match is exhaustive.
        }
        "VarDecl" => {
            // `static const int NAME = <int literal>;` — extract as a constant.
            let vname = node.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let storage = node.get("storageClass").and_then(|v| v.as_str()).unwrap_or("");
            let is_const = node
                .get("type")
                .and_then(|t| t.get("qualType"))
                .and_then(|v| v.as_str())
                .map(|s| s.contains("const"))
                .unwrap_or(false);
            if !vname.is_empty() && storage == "static" && is_const {
                if let Some(init) = node.get("inner").and_then(|v| v.as_array()) {
                    for i in init {
                        if let Some(v) = i.get("value").and_then(|v| v.as_str()).and_then(|s| s.parse::<i64>().ok()) {
                            api.constants.push((vname, v));
                            break;
                        }
                    }
                }
            }
        }
        "TypedefDecl" => {
            let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let underlying = node
                .get("type")
                .and_then(|t| t.get("qualType"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !name.is_empty() {
                ctx.typedefs.push((name.clone(), underlying.clone()));
                // A typedef of an anonymous record body (`typedef struct { ... } X;`
                // or `typedef union { ... } X;`) — the body was remembered in
                // `ctx.anon_struct` by the preceding RecordDecl. Surface it as a
                // named struct so Lime has a concrete type to use.
                if let Some(fields) = ctx.anon_struct.take() {
                    if !underlying.contains("enum") {
                        let is_union_typedef = underlying.contains("union");
                        let all_8 = fields.iter().all(|f| matches!(
                            f.ty,
                            CType::Long | CType::Double | CType::Pointer(_) | CType::Function(..) | CType::Opaque(_)
                        ));
                        let has_fp = fields.iter().any(|f| matches!(f.ty, CType::Function(..)));
                        api.structs.push(CStruct {
                            name: name.clone(),
                            fields,
                            size_bytes: None,
                            align_bytes: None,
                            is_union: is_union_typedef,
                            is_bitfield: false,
                            all_8byte: all_8,
                            has_fn_ptr: has_fp,
                        });
                    }
                }
            }
        }
        "FunctionDecl" => {
            let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if name.is_empty() {
                return;
            }
            let ftype = node.get("type").cloned().unwrap_or(serde_json::Value::Null);
            let (params, ret_ty) = params_and_ret(&ftype);
            let symbol = node
                .get("mangledName")
                .and_then(|v| v.as_str())
                .unwrap_or(&name)
                .to_string();
            api.functions.push(CFunction {
                name: name.clone(),
                symbol,
                params,
                ret: ret_ty,
                is_method: false,
                is_constructor: false,
                is_const: false,
                self_ty: None,
            });
        }
        _ => {
            if let Some(inner) = node.get("inner").and_then(|v| v.as_array()) {
                for c in inner {
                    classify_node(c, api, ctx, self_ty.clone(), lang);
                }
            }
        }
    }
}

fn params_and_ret(ftype: &serde_json::Value) -> (Vec<CParam>, CType) {
    // The clang AST JSON `type` node typically exposes only `qualType`, e.g.
    // "int (int, int)" or "Point (int, int)" or "int (*)(int, int)". Parse it
    // directly rather than relying on a `proto` sub-object which is absent in
    // `-ast-dump=json` output.
    let qual = ftype
        .get("qualType")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    parse_signature(&qual)
}

/// Parse a C/C++ function type string like "int (int, int)" into (params, ret).
fn parse_signature(qual: &str) -> (Vec<CParam>, CType) {
    let qual = qual.trim();
    // Split return type from the parenthesized parameter list.
    let open = match qual.find('(') {
        Some(i) => i,
        None => return (Vec::new(), parse_c_type(qual)),
    };
    let ret_part = qual[..open].trim();
    let ret_ty = parse_c_type(ret_part);
    // parameter list is between the first '(' and its matching ')'.
    let close = qual.rfind(')').unwrap_or(open);
    let params_str = &qual[open + 1..close];
    let mut params = Vec::new();
    if !params_str.trim().is_empty() {
        // Split on top-level commas only (function-pointer params contain
        // nested parentheses, e.g. "int (*)(int, int)").
        for (i, p) in split_top_level(params_str).into_iter().enumerate() {
            let p = p.trim();
            let ty_str = strip_param_name(p);
            params.push(CParam {
                name: format!("a{}", i),
                ty: parse_c_type(ty_str.trim()),
            });
        }
    }
    (params, ret_ty)
}

/// Split a comma-separated parameter list on top-level commas only, ignoring
/// commas nested inside parentheses (e.g. function-pointer parameter types).
fn split_top_level(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in s.chars() {
        match ch {
            '(' => {
                depth += 1;
                cur.push(ch);
            }
            ')' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// Given "int a" / "const char* name" / "int", return just the type text.
fn strip_param_name(p: &str) -> String {
    let trimmed = p.trim();
    // Complex types (function pointers, arrays) carry no trailing param name.
    if trimmed.contains('(') || trimmed.contains('[') {
        return trimmed.to_string();
    }
    if trimmed.ends_with('*') || trimmed.ends_with('&') {
        return trimmed.to_string();
    }
    let mut parts: Vec<&str> = trimmed.split_whitespace().collect();
    // Remove a trailing token if it looks like a parameter name (not a
    // qualifier/keyword and the preceding token is a type). Keep it simple:
    // if there are >= 2 parts and the last part is not a known qualifier,
    // drop it.
    if parts.len() >= 2 {
        let last = *parts.last().unwrap();
        let known = matches!(
            last,
            "*" | "&" | "const" | "volatile" | "restrict" | "struct" | "class" | "enum" | "unsigned" | "signed" | "long" | "short"
        );
        if !known {
            // Only drop if the remaining parts still form a plausible type.
            let _ = parts.pop();
            return parts.join(" ").trim().to_string();
        }
    }
    trimmed.to_string()
}

// ----------------------------------------------------------------------------
// Lime type mapping
// ----------------------------------------------------------------------------

fn lime_type_name(t: &CType) -> String {
    match t {
        CType::Int | CType::Long | CType::Bool => "Int".to_string(),
        CType::Float | CType::Double => "Float".to_string(),
        CType::Void => "Unit".to_string(),
        CType::String => "String".to_string(),
        CType::Pointer(inner) => lime_type_name(inner), // simplify: treat as pointee
        CType::Function(_, _) => "Callback".to_string(), // opaque C fn pointer, ABI = i8*
        CType::Struct(s) => s.clone(),
        // Task #2/#6: opaque C pointer handle (`struct X*` / `void*` / a C++
        // template instantiation such as `Stack<long long>*`). Emitted as
        // Lime's `Opaque(X)` type spelling, which lowers to a bare `ptr`.
        // Task #6: a template instantiation name like `Stack<long long>` is
        // normalized to `Stack_long_long` so the Lime parser accepts it
        // (`Opaque(Stack<long long>)` would be a parse error since `<` is a
        // separator token). The original spelling + args live in the CIR lite /
        // manifest for auditability.
        CType::Opaque(s) => format!("Opaque({})", s),
        CType::Other(s) => s.clone(),
    }
}

// ----------------------------------------------------------------------------
// Real-world Phase A: out-param → return-value adapter generation
// ----------------------------------------------------------------------------
//
// Many real-world C APIs (SQLite is the canonical example) return resource
// handles exclusively through *out-params* (`sqlite3 **`) because they have no
// `sqlite3*` return value. Lime's `Opaque(name)` handle is a bare `ptr` and is
// only ever received as a *return value* or a by-value parameter — it cannot
// take the address of a local and hand it to C as an out-param. So Charger
// bridges the missing information by generating a tiny C shim per out-param
// function and re-spelling the Lime `extern fn` so the handle comes back as a
// normal return value:
//
//   C:   int sqlite3_open(const char*, sqlite3**);
//   shim (in the prepared native artifact):
//     sqlite3* lime_out_sqlite3_open(const char* a0) {
//         sqlite3* a1 = 0;
//         sqlite3_open(a0, &a1);   // write handle through the out-param
//         return a1;               // hand it back as a return value
//     }
//   Lime iface:
//     extern fn sqlite3_open(String: a0) -> Opaque(sqlite3) "lime_out_sqlite3_open"
//
// No new Lime type or ABI is introduced: the shim is plain C reusing the
// existing `Opaque` handle representation and the prepared native artifact. The
// second bridge Charger provides here is the "null-callback" shim: a C function
// whose trailing parameter is a `Callback` (function pointer) — e.g.
// `sqlite3_exec` — is surfaced to Lime without that callback (and everything
// after it), the shim passing `NULL` for the dropped arguments. Lime has no
// null-pointer literal, so this is the only architecture-compliant way to call
// such functions with a `NULL` callback/arg/errmsg.

/// Detect a C out-param: a *pointer to a pointer* to an opaque/named type
/// (`sqlite3 **`, `void **`), normalized to `CType::Pointer(Box::new(
/// CType::Opaque(name)))`. A single-level handle (`sqlite3 *` →
/// `CType::Opaque(name)`) is NOT an out-param.
///
/// Returns `Some(name)` (the opaque handle type name) when `t` is an out-param.
fn is_out_param(t: &CType) -> Option<String> {
    if let CType::Pointer(inner) = t {
        if let CType::Opaque(name) = inner.as_ref() {
            return Some(name.clone());
        }
    }
    None
}

/// Render a `CType` as the C type text needed to declare a shim's parameters /
/// locals. Opaque/Struct/Other named types are pointers in the C ABI
/// (`sqlite3*`); each `Pointer` adds one more level of indirection.
fn c_type_text(t: &CType) -> String {
    match t {
        CType::Int => "int".to_string(),
        CType::Long => "long long".to_string(),
        CType::Float => "float".to_string(),
        CType::Double => "double".to_string(),
        CType::Bool => "int".to_string(),
        CType::Void => "void".to_string(),
        // `CType::String` is the scalar `char`; a C string `char*` is modeled as
        // `Pointer(String)` (an extra indirection), so render the base as `char`.
        CType::String => "char".to_string(),
        CType::Pointer(inner) => format!("{}*", c_type_text(inner)),
        CType::Function(params, ret) => {
            let ps: Vec<String> = params.iter().map(|p| c_type_text(p)).collect();
            format!("{} (*)({})", c_type_text(ret), ps.join(", "))
        }
        CType::Struct(s) | CType::Opaque(s) => format!("{}*", s),
        // `CType::Other(s)` is an unmodeled type spelling (e.g. a typedef'd
        // scalar like `sqlite3_int64`, or a forward-declared record). Emit it
        // verbatim — do NOT append `*` — because we cannot tell from the spelling
        // alone whether it is a pointer; the parser already attached a
        // `CType::Pointer` wrapper when one was present. Appending `*` here would
        // wrongly turn `sqlite3_int64` into `sqlite3_int64*` and break adapter C.
        CType::Other(s) => s.clone(),
    }
}

/// A Charger-generated C shim that bridges a C idiom Lime cannot express
/// directly: an out-param (handle returned through `**`, surfaced as a return
/// value) and/or a trailing `NULL` callback (and the args after it).
struct AdapterSpec {
    lime_name: String,  // Lime-facing fn name (e.g. "sqlite3_open")
    symbol: String,     // shim symbol (e.g. "lime_out_sqlite3_open")
    real_symbol: String, // real C symbol (e.g. "sqlite3_open")
    ret_name: Option<String>, // opaque handle type name when this is an out-param
    ret: CType,         // real C return type (used when not an out-param)
    params: Vec<CParam>, // original C params (for shim body)
    out_idx: Option<usize>, // index of the out-param, if any
    drop_from: Option<usize>, // drop this param and everything after (NULL callback)
}

/// Inspect the normalized API and emit an [`AdapterSpec`] for every function
/// that needs a bridge: a function with an out-param (`Pointer(Opaque)`) and/or
/// a trailing `Callback` parameter. Functions needing no bridge are skipped.
fn collect_out_param_adapters(api: &NormalizedApi) -> Vec<AdapterSpec> {
    let mut out = Vec::new();
    for f in &api.functions {
        let out_idx = f.params.iter().position(|p| is_out_param(&p.ty).is_some());
        // A `Callback` (CType::Function) trailing parameter is the common
        // "optional callback + user data + errmsg" idiom; drop it and the
        // params after it, passing NULL.
        let drop_from = f.params.iter().position(|p| matches!(p.ty, CType::Function(_, _)));
        if out_idx.is_none() && drop_from.is_none() {
            continue;
        }
        let ret_name = out_idx.map(|i| is_out_param(&f.params[i].ty).unwrap());
        out.push(AdapterSpec {
            lime_name: sanitize_name(&f.name),
            symbol: format!("lime_out_{}", sanitize_name(&f.name)),
            real_symbol: f.symbol.clone(),
            ret_name,
            ret: f.ret.clone(),
            params: f.params.clone(),
            out_idx,
            drop_from,
        });
    }
    out
}

/// Generate the C source for every adapter shim. `header_name` is the library
/// header to `#include` (e.g. "sqlite3.h") so the shim sees the real types.
fn gen_adapter_c_source(
    adapters: &[AdapterSpec],
    constants: &[(String, i64)],
    structs: &[CStruct],
    header_name: &str,
) -> String {
    let mut s = String::new();
    s.push_str("/* Charger-generated adapter shims (out-param + null-callback + const + union/bitfield accessors). DO NOT EDIT. */\n");
    s.push_str("#include <stddef.h>\n#include <stdlib.h>\n");
    s.push_str(&format!("#include \"{}\"\n", header_name));
    // Union / bitfield accessor shims: since Lime cannot model overlapping
    // members or sub-byte bitfields (Lime `int` is i64), the record is surfaced
    // as an opaque handle and these C shims do the real field access on the C
    // side (using clang's own layout — the source of truth).
    for st in structs {
        // Generate accessor shims for any record Lime cannot model as a real
        // struct: unions (overlapping members), bitfields (sub-byte fields),
        // and sub-8-byte structs (char/short/int members — Lime's int is i64).
        if st.is_union || st.is_bitfield || !st.all_8byte {
            // Constructor allocating the record on the heap (Lime owns the pointer).
        s.push_str(&format!(
            "void* lime_make_{}(void) {{ return (void*)calloc(1, sizeof({})); }}\n",
            st.name, st.name
        ));
        for f in &st.fields {
            // Skip array members (e.g. `char bytes[4]`) — C cannot pass/return
            // arrays by value through a shim; they are accessed via pointer in
            // real C code. Surface only scalar/aggregate members.
            if let CType::Other(txt) = &f.ty {
                if txt.contains('[') {
                    continue;
                }
            }
            let c_ty = c_type_text(&f.ty);
            s.push_str(&format!(
                "{} lime_get_{}_{}({}* u) {{ return ({})u->{}; }}\n",
                c_ty, st.name, f.name, st.name, c_ty, f.name
            ));
            s.push_str(&format!(
                "void lime_set_{}_{}({}* u, {} v) {{ u->{} = ({})v; }}\n",
                st.name, f.name, st.name, c_ty, f.name, c_ty
            ));
        }
        s.push_str("\n");
        }
        // Callback-table shims: a struct with function-pointer fields stores
        // Lime callbacks (raw fn ptrs, ABI-compatible with C function pointers)
        // into each field. Nullable callbacks get a separate NULL-setter so the
        // C side's `if (t->f != NULL)` guard works without ever invoking a
        // dangling/zero address.
        if st.has_fn_ptr {
            s.push_str(&format!(
                "void* lime_make_{}(void) {{ return (void*)calloc(1, sizeof({})); }}\n",
                st.name, st.name
            ));
            for f in &st.fields {
                if matches!(f.ty, CType::Function(..)) {
                    s.push_str(&format!(
                        "void lime_set_{}_{}({}* t, void* f) {{ *(void**)(&t->{}) = f; }}\n",
                        st.name, f.name, st.name, f.name
                    ));
                    s.push_str(&format!(
                        "void lime_set_{}_{}_null({}* t) {{ t->{} = 0; }}\n",
                        st.name, f.name, st.name, f.name
                    ));
                } else {
                    let c_ty = c_type_text(&f.ty);
                    s.push_str(&format!(
                        "void lime_set_{}_{}({}* t, {} v) {{ t->{} = ({})v; }}\n",
                        st.name, f.name, st.name, c_ty, f.name, c_ty
                    ));
                }
            }
            s.push_str("\n");
        }
    }
    // Constant shims: `int lime_const_NAME() { return <value>; }` — surfaces a
    // C integer constant/macro as a zero-arg extern fn callable from Lime
    // (Lime has no top-level `const`, so this preserves the value without a
    // language change).
    for (name, val) in constants {
        s.push_str(&format!(
            "int lime_const_{}(void) {{ return (int)({}); }}\n\n",
            name, val
        ));
    }
    for a in adapters {
        // Indices dropped from the Lime-facing signature.
        let mut drop: std::collections::HashSet<usize> = std::collections::HashSet::new();
        if let Some(oi) = a.out_idx {
            drop.insert(oi);
        }
        if let Some(df) = a.drop_from {
            for i in df..a.params.len() {
                drop.insert(i);
            }
        }
        // Lime-facing parameter declarations (kept params only).
        let mut decls: Vec<String> = Vec::new();
        for (i, p) in a.params.iter().enumerate() {
            if drop.contains(&i) {
                continue;
            }
            decls.push(format!("{} a{}", c_type_text(&p.ty), i));
        }
        // Return type (C).
        let ret_c = if let Some(name) = &a.ret_name {
            format!("{}*", name) // out-param: return the handle pointer
        } else {
            c_type_text(&a.ret)
        };
        // Real call arguments.
        let mut call_args: Vec<String> = Vec::new();
        for (i, _p) in a.params.iter().enumerate() {
            if Some(i) == a.out_idx {
                call_args.push(format!("&a{}", i)); // write handle here
            } else if drop.contains(&i) {
                call_args.push("0".to_string()); // NULL
            } else {
                call_args.push(format!("a{}", i));
            }
        }
        if let Some(oi) = a.out_idx {
            // The local holding the handle is the pointee of the out-param.
            let local_ty = if let CType::Pointer(inner) = &a.params[oi].ty {
                c_type_text(inner)
            } else {
                format!("{}*", a.ret_name.as_deref().unwrap_or("void"))
            };
            s.push_str(&format!(
                "{} {} ({}) {{\n    {} a{} = 0;\n    {}({});\n    return a{};\n}}\n\n",
                ret_c,
                a.symbol,
                decls.join(", "),
                local_ty,
                oi,
                a.real_symbol,
                call_args.join(", "),
                oi
            ));
        } else {
            s.push_str(&format!(
                "{} {} ({}) {{\n    return {}({});\n}}\n\n",
                ret_c,
                a.symbol,
                decls.join(", "),
                a.real_symbol,
                call_args.join(", ")
            ));
        }
    }
    s
}

/// Compile the adapter shims and insert them into the prepared native artifact
/// (`art_path`). The shim `.c` is compiled with the same toolchain/flags and
/// `#include`s the library header (found via `header`'s directory).
fn build_adapters_into(
    adapters: &[AdapterSpec],
    constants: &[(String, i64)],
    structs: &[CStruct],
    art_path: &Path,
    header: &Path,
    llvm_bindir: &str,
    lang: ApiKind,
) -> Result<(), String> {
    if adapters.is_empty() && constants.is_empty() && structs.is_empty() {
        return Ok(());
    }
    let header_name = header
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let src = gen_adapter_c_source(adapters, constants, structs, &header_name);
    let build_dir = std::env::temp_dir().join("charger_build_adapters");
    let _ = std::fs::create_dir_all(&build_dir);
    let c_path = build_dir.join("lime_adapters.c");
    std::fs::write(&c_path, src).map_err(|e| format!("adapter gen failed: {}", e))?;
    let obj_path = build_dir.join("lime_adapters.obj");
    let clang = if lang == ApiKind::Cpp {
        PathBuf::from(llvm_bindir).join("clang++.exe")
    } else {
        PathBuf::from(llvm_bindir).join("clang.exe")
    };
    let clang = if clang.exists() {
        clang
    } else {
        PathBuf::from(llvm_bindir).join(if lang == ApiKind::Cpp { "clang++" } else { "clang" })
    };
    let mut cmd = Command::new(&clang);
    cmd.arg("-O2").arg("-c");
    if lang == ApiKind::Cpp {
        cmd.arg("-std=c++17");
    }
    if let Some(dir) = header.parent() {
        cmd.arg("-I").arg(dir);
    }
    cmd.arg(&c_path).arg("-o").arg(&obj_path);
    let status = cmd
        .status()
        .map_err(|e| format!("adapter build failed: {} launch error: {}", clang.display(), e))?;
    if !status.success() {
        return Err(format!("adapter build failed: {} exited with {}", clang.display(), status));
    }
    let ar = PathBuf::from(llvm_bindir).join("llvm-ar.exe");
    let ar = if ar.exists() {
        ar
    } else {
        PathBuf::from(llvm_bindir).join("llvm-ar")
    };
    let ar_status = Command::new(&ar)
        .arg("r")
        .arg(art_path)
        .arg(&obj_path)
        .status()
        .map_err(|e| format!("adapter archive failed: {}", e))?;
    if !ar_status.success() {
        return Err("adapter archive failed".to_string());
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// Lime interface generation
// ----------------------------------------------------------------------------

/// Generate `lime-iface.lime` source text. Each C/C++ function becomes an
/// `extern` Lime declaration. Structs become Lime `struct` definitions whose
/// field layout matches the C/C++ layout (field order/names/types only; the
/// ABI metadata records size/align for verification).
fn generate_lime_iface(api: &NormalizedApi, lib_name: &str, adapters: &[AdapterSpec]) -> String {
    let mut out = String::new();
    out.push_str(&format!("// Charger-generated Lime interface for '{}'\n", lib_name));
    out.push_str("// DO NOT EDIT: regenerate with `charger install`.\n\n");

    // Constants (enum enumerators, static consts, integer object-macros) are
    // surfaced as zero-arg `extern fn` shims that return the literal value.
    // Lime has no top-level `const` form, so a generated C shim
    // (`lime_const_NAME` -> `return <value>;`) gives the same effect without
    // changing Lime's surface syntax (Architecture Gate respected).
    for (name, val) in &api.constants {
        out.push_str(&format!(
            "extern fn {}(Int) -> Int \"lime_const_{}\"\n",
            name, name
        ));
    }
    if !api.constants.is_empty() {
        out.push_str("\n");
    }

    // Structs first (Lime requires types to be visible before use).
    for s in &api.structs {
        if s.has_fn_ptr {
            // A struct containing function-pointer fields (a callback table /
            // operations table / vtable-like C struct) is surfaced as an opaque
            // handle. Lime callbacks round-trip through generated C setter shims
            // that store the native function pointer into the C struct field.
            // No library-specific code — any `T (*f)(...)` field is handled.
            out.push_str(&format!("// Opaque handle for C callback-table struct '{}' (function-pointer fields); use lime_set_* shims\n", s.name));
            out.push_str(&format!("extern fn lime_make_{}() -> Opaque({}) \"lime_make_{}\"\n", s.name, s.name, s.name));
            for f in &s.fields {
                let lime_ty = lime_type_name(&f.ty);
                if matches!(f.ty, CType::Function(..)) {
                    // Function-pointer field: setter stores a Lime callback
                    // (Callback == i8* raw fn ptr, ABI-compatible with the C
                    // function pointer). Null-setter clears it to NULL.
                    out.push_str(&format!(
                        "extern fn lime_set_{}_{}(Opaque({}): a0, Callback: a1) \"lime_set_{}_{}\"\n",
                        s.name, f.name, s.name, s.name, f.name
                    ));
                    out.push_str(&format!(
                        "extern fn lime_set_{}_{}_null(Opaque({}): a0) \"lime_set_{}_{}_null\"\n",
                        s.name, f.name, s.name, s.name, f.name
                    ));
                } else {
                    // Non-function field (e.g. `void *userdata`): typed set.
                    out.push_str(&format!(
                        "extern fn lime_set_{}_{}(Opaque({}): a0, {}: a1) \"lime_set_{}_{}\"\n",
                        s.name, f.name, s.name, lime_ty, s.name, f.name
                    ));
                }
            }
            out.push_str("\n");
            continue;
        }
        if s.is_union || s.is_bitfield {
            // Unions and bitfields cannot be modeled as Lime structs (Lime's
            // `int` is i64/8 bytes; overlapping members and sub-byte bitfields
            // are unrepresentable). Surface as an opaque handle; accessor
            // shims (generated in the adapter C source) provide typed get/set.
            out.push_str(&format!("// Opaque handle for C {} '{}' (union/bitfield); use lime_*_get/set shims\n", if s.is_union { "union" } else { "bitfield struct" }, s.name));
            out.push_str(&format!("extern fn lime_make_{}() -> Opaque({}) \"lime_make_{}\"\n", s.name, s.name, s.name));
            for f in &s.fields {
                // Skip array members (C cannot pass/return arrays by value
                // through a shim); matches the adapter C-source generator.
                if let CType::Other(txt) = &f.ty {
                    if txt.contains('[') {
                        continue;
                    }
                }
                let lime_ty = lime_type_name(&f.ty);
                out.push_str(&format!(
                    "extern fn lime_get_{}_{}(Opaque({}): a0) -> {} \"lime_get_{}_{}\"\n",
                    s.name, f.name, s.name, lime_ty, s.name, f.name
                ));
                out.push_str(&format!(
                    "extern fn lime_set_{}_{}(Opaque({}): a0, {}: a1) \"lime_set_{}_{}\"\n",
                    s.name, f.name, s.name, lime_ty, s.name, f.name
                ));
            }
            out.push_str("\n");
            continue;
        }
        if s.fields.is_empty() {
            // opaque / empty: emit as a unit-ish placeholder struct so the
            // type name resolves. (Future: ABI metadata drives real layout.)
            out.push_str(&format!("struct {} {{\n}}\n\n", s.name));
            continue;
        }
        // A struct whose fields are ALL 8-byte-wide types (long long / double /
        // pointer) can be modeled as a real Lime struct (Lime's scalars are all
        // 8 bytes). Any sub-8-byte field (char/short/int) or aggregate that
        // Lime cannot lay out must be surfaced as an opaque handle with accessor
        // shims (clang owns the real layout — the source of truth).
        let all_8byte = s.fields.iter().all(|f| matches!(
            f.ty,
            CType::Long | CType::Double | CType::Pointer(_) | CType::Function(..) | CType::Opaque(_)
        ));
        if !all_8byte {
            out.push_str(&format!("// Opaque handle for C struct '{}' (sub-8-byte layout); use lime_*_get/set shims\n", s.name));
            out.push_str(&format!("extern fn lime_make_{}() -> Opaque({}) \"lime_make_{}\"\n", s.name, s.name, s.name));
            for f in &s.fields {
                if let CType::Other(txt) = &f.ty {
                    if txt.contains('[') {
                        continue;
                    }
                }
                let lime_ty = lime_type_name(&f.ty);
                out.push_str(&format!(
                    "extern fn lime_get_{}_{}(Opaque({}): a0) -> {} \"lime_get_{}_{}\"\n",
                    s.name, f.name, s.name, lime_ty, s.name, f.name
                ));
                out.push_str(&format!(
                    "extern fn lime_set_{}_{}(Opaque({}): a0, {}: a1) \"lime_set_{}_{}\"\n",
                    s.name, f.name, s.name, lime_ty, s.name, f.name
                ));
            }
            out.push_str("\n");
            continue;
        }
        out.push_str(&format!("struct {} {{\n", s.name));
        for f in &s.fields {
            out.push_str(&format!("    {}: {}\n", f.name, lime_type_name(&f.ty)));
        }
        out.push_str("}\n\n");
    }

    // Functions / methods.
    let adapter_map: std::collections::HashMap<String, &AdapterSpec> =
        adapters.iter().map(|a| (a.lime_name.clone(), a)).collect();
    for f in &api.functions {
        let ret_lime = lime_type_name(&f.ret);
        let params_lime: Vec<String> = f
            .params
            .iter()
            .map(|p| format!("{}: {}", lime_type_name(&p.ty), p.name))
            .collect();
        // Symbol literal: the linkable name (mangled for C++).
        if f.is_constructor {
            // Constructors are exposed as factory functions returning the
            // struct by value (Lime receives it as the struct type).
            out.push_str(&format!(
                "extern fn {}_new({}) -> {} \"{}\"\n",
                f.self_ty.clone().unwrap_or_default().to_lowercase(),
                params_lime.join(", "),
                f.self_ty.clone().unwrap_or_default(),
                f.symbol
            ));
        } else if f.is_method {
            // Methods: the receiver (`self`) is already the first entry in
            // `params` (prepended during normalization), so emit them directly.
            out.push_str(&format!(
                "extern fn {}_{}({}) -> {} \"{}\"\n",
                f.self_ty.clone().unwrap_or_default().to_lowercase(),
                sanitize_method(&f.name),
                params_lime.join(", "),
                ret_lime,
                f.symbol
            ));
        } else if let Some(ad) = adapter_map.get(&sanitize_name(&f.name)) {
            // Real-world Phase A: re-spell the Lime `extern fn` through a
            // Charger shim. The out-param (if any) becomes the return value;
            // dropped parameters (trailing NULL callback + its args) are
            // omitted from the Lime signature.
            let mut drop: std::collections::HashSet<usize> = std::collections::HashSet::new();
            if let Some(oi) = ad.out_idx {
                drop.insert(oi);
            }
            if let Some(df) = ad.drop_from {
                for i in df..f.params.len() {
                    drop.insert(i);
                }
            }
            let params_lime: Vec<String> = f
                .params
                .iter()
                .enumerate()
                .filter(|(i, _)| !drop.contains(i))
                .map(|(_, p)| format!("{}: {}", lime_type_name(&p.ty), p.name))
                .collect();
            let ret_lime = if ad.ret_name.is_some() {
                format!("Opaque({})", ad.ret_name.as_ref().unwrap())
            } else {
                ret_lime.clone()
            };
            out.push_str(&format!(
                "extern fn {}({}) -> {} \"{}\"\n",
                ad.lime_name,
                params_lime.join(", "),
                ret_lime,
                ad.symbol
            ));
        } else {
            out.push_str(&format!(
                "extern fn {}({}) -> {} \"{}\"\n",
                sanitize_name(&f.name),
                params_lime.join(", "),
                ret_lime,
                f.symbol
            ));
        }
    }
    out
}

fn sanitize_name(n: &str) -> String {
    n.replace([':', '~', '?', '@', '(', ')'], "_")
}

fn sanitize_method(n: &str) -> String {
    // strip "Class::" prefix if present
    let s = n.split("::").last().unwrap_or(n);
    sanitize_name(s)
}

// ----------------------------------------------------------------------------
// ABI metadata (abi.json)
// ----------------------------------------------------------------------------

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct AbiMeta {
    pub os: String,
    pub arch: String,
    pub compiler: String,
    pub compiler_version: String,
    pub cxx_abi: String,        // e.g. "MSVC" / "Itanium"
    pub cxx_stdlib: String,
    pub build_flags: Vec<String>,
}

// ----------------------------------------------------------------------------
// Store
// ----------------------------------------------------------------------------

pub fn store_root() -> PathBuf {
    PathBuf::from(".lime-charger").join("store")
}

fn toolchain_hash(abi: &AbiMeta, build_flags: &[String]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    format!("{:?}", abi).hash(&mut h);
    build_flags.join("|").hash(&mut h);
    format!("{:016x}", h.finish())
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct Manifest {
    pub library: String,
    pub version: String,
    pub source_origin: String, // path or url
    pub source_hash: String,
    pub dependencies: Vec<String>,
    pub artifact: String, // filename within the store entry
    pub artifact_hash: String,
    pub abi: AbiMeta,
    pub symbols: Vec<String>, // linkable symbols this artifact provides
}

// ----------------------------------------------------------------------------
// Public entry point: charger install
// ----------------------------------------------------------------------------

pub struct InstallResult {
    pub lib_name: String,
    pub store_path: PathBuf,
    pub api: NormalizedApi,
}

/// Install a C or C++ library: build it, analyze it, and prepare a Charger
/// artifact in the local store. `source` is a path to a directory containing
/// the sources, or a single source file. The header is auto-detected.
pub fn install(source: &str, llvm_bindir: &str) -> Result<InstallResult, String> {
    let src_path = PathBuf::from(source);
    if !src_path.exists() {
        return Err(format!("compiler/build error: source '{}' not found", source));
    }

    // Determine language: if any .cpp/.cc/.cxx present -> C++, else C.
    let (lang, sources, header) = collect_sources(&src_path)?;
    if header.is_none() && sources.is_empty() {
        return Err("AST extraction failed: no C/C++ source or header found".to_string());
    }
    let header = header.ok_or_else(|| "AST extraction failed: no header found".to_string())?;

    // 1. Build native artifact (.lib / .a) via clang / clang++.
    let lib_name = src_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "libcharger".to_string());

    // #7: detect dependencies (and the include dirs needed to compile the
    // dependent sources) from local `#include "..."` directives in this lib.
    let (deps, include_dirs) = detect_dependencies(&sources, &header, &lib_name);

    // Deterministic store key + cache inputs (cheap; no native build yet).
    let abi = detect_abi(llvm_bindir, lang);
    let mut build_flags = if lang == ApiKind::Cpp {
        vec!["-O2".to_string(), "-std=c++17".to_string()]
    } else {
        vec!["-O2".to_string()]
    };
    // SQLite amalgamation declares optional features (unlock-notify, snapshot,
    // rtree) in the header but only compiles them when the matching SQLITE_ENABLE_*
    // macro is set; otherwise the symbols are declared-but-undefined and the link
    // step fails. Enable the commonly-declared optional features so the prepared
    // artifact is self-contained. This is a SQLite-specific build flag, not a
    // general ABI change.
    if lib_name == "sqlite" {
        build_flags.push("-DSQLITE_ENABLE_UNLOCK_NOTIFY".to_string());
        build_flags.push("-DSQLITE_ENABLE_SNAPSHOT".to_string());
        build_flags.push("-DSQLITE_ENABLE_RTREE".to_string());
    }
    let tool_hash = toolchain_hash(&abi, &build_flags);
    let version = "0.1.0".to_string();
    let entry = store_root().join(&lib_name).join(&version).join(&tool_hash);
    let src_hash = hash_path(&src_path);

    // AST analysis (cheap, no native build): derive the API surface AND the
    // out-param / null-callback adapter shims before any native build. The
    // normalized API is needed both for the cache-hit path (to (re)generate the
    // iface + shims) and the full build path.
    let ast = extract_ast_json(&header, lang, llvm_bindir)?;
    let mut api = normalize(&ast, lang);
    // Macros are preprocessor text and never appear in the AST JSON, so scan
    // the header source directly for `#define NAME <int>` forms. This is the
    // only place macros can be recovered; AST-based extraction cannot see them.
    extract_macro_constants(&header, &mut api.constants);
    let adapters = collect_out_param_adapters(&api);

    // #8: artifact cache. If a store entry already exists whose manifest's
    // source_hash matches the current source_hash, the native artifact is
    // reusable: skip the native rebuild and reuse the prepared artifact. This
    // is a true cache hit. Only a *changed* source (different source_hash)
    // triggers a rebuild (invalidation).
    if entry.exists() {
        if let Some(mut m) = load_manifest(&entry) {
            if m.source_hash == src_hash {
                println!(
                    "charger: cache hit for '{}' (hash={}), reusing artifact",
                    lib_name, src_hash
                );
                // Even on a cache hit we must ensure the shim symbols live in
                // the reused artifact (a stale artifact built by an older
                // Charger lacks them) and (re)generate the iface with adapters.
                let art_dest = entry.join(&m.artifact);
                build_adapters_into(&adapters, &api.constants, &api.structs, &art_dest, &header, llvm_bindir, lang)?;
                let iface = generate_lime_iface(&api, &lib_name, &adapters);
                std::fs::write(entry.join("lime-iface.lime"), &iface)
                    .map_err(|e| format!("Lime interface generation failed: {}", e))?;
                // Persist the shim symbols so `lime build` can resolve the
                // artifact when the Lime program references them.
                for a in &adapters {
                    if !m.symbols.contains(&a.symbol) {
                        m.symbols.push(a.symbol.clone());
                    }
                }
                let manifest_toml = toml::to_string_pretty(&m)
                    .map_err(|e| format!("artifact store failed: manifest error: {}", e))?;
                std::fs::write(entry.join("manifest.toml"), manifest_toml)
                    .map_err(|e| format!("artifact store failed: {}", e))?;
                return Ok(InstallResult {
                    lib_name,
                    store_path: entry,
                    api,
                });
            } else {
                println!(
                    "charger: source changed for '{}' (stored={}, current={}); rebuilding",
                    lib_name, m.source_hash, src_hash
                );
            }
        }
    }

    let build_dir = std::env::temp_dir().join(format!("charger_build_{}", lib_name));
    let _ = std::fs::create_dir_all(&build_dir);
    let obj_path = build_dir.join(format!("{}.obj", lib_name));
    let clang = if lang == ApiKind::Cpp {
        PathBuf::from(llvm_bindir).join("clang++.exe")
    } else {
        PathBuf::from(llvm_bindir).join("clang.exe")
    };
    let clang = if clang.exists() {
        clang
    } else {
        PathBuf::from(llvm_bindir).join(if lang == ApiKind::Cpp { "clang++" } else { "clang" })
    };
    let mut cmd = Command::new(&clang);
    cmd.arg("-O2").arg("-c");
    if lang == ApiKind::Cpp {
        cmd.arg("-std=c++17");
    }
    for inc in &include_dirs {
        cmd.arg("-I").arg(inc);
    }
    for s in &sources {
        cmd.arg(s);
    }
    cmd.arg("-o").arg(&obj_path);
    let status = cmd
        .status()
        .map_err(|e| format!("native build failed: {} launch error: {}", clang.display(), e))?;
    if !status.success() {
        return Err(format!("native build failed: {} exited with {}", clang.display(), status));
    }
    // archive into .lib (use llvm-ar)
    let ar = PathBuf::from(llvm_bindir).join("llvm-ar.exe");
    let ar = if ar.exists() {
        ar
    } else {
        PathBuf::from(llvm_bindir).join("llvm-ar")
    };
    let art_ext = if cfg!(windows) { "lib" } else { "a" };
    let art_name = format!("{}.{}", lib_name, art_ext);
    let art_path = build_dir.join(&art_name);
    let ar_status = Command::new(&ar)
        .arg("rcs")
        .arg(&art_path)
        .arg(&obj_path)
        .status()
        .map_err(|e| format!("native build failed: llvm-ar launch error: {}", e))?;
    if !ar_status.success() {
        return Err("native build failed: llvm-ar exited with error".to_string());
    }

    // 2b. Build out-param / null-callback adapter shims and insert them into
    // the prepared native artifact (so the Lime `extern fn` shim symbols
    // resolve at link time).
    build_adapters_into(&adapters, &api.constants, &api.structs, &art_path, &header, llvm_bindir, lang)?;

    // 2. AST extraction + normalization (already performed earlier; reuse it).

    // For C++ methods, ensure the receiver struct is recorded even if only
    // declared inline. (Vertical slice: structs come from RecordDecl.)
    // (No extra work needed for the slice; struct fields already captured.)

    // 3. Lime interface generation (with out-param / null-callback adapters).
    let iface = generate_lime_iface(&api, &lib_name, &adapters);

    // 5. Store. (abi / build_flags / tool_hash / version / entry were computed
    // earlier, before the cache-hit check, and are reused here.)
    let _ = std::fs::create_dir_all(&entry);

    let art_dest = entry.join(&art_name);
    std::fs::copy(&art_path, &art_dest)
        .map_err(|e| format!("artifact store failed: {}", e))?;

    let iface_dest = entry.join("lime-iface.lime");
    std::fs::write(&iface_dest, iface).map_err(|e| format!("Lime interface generation failed: {}", e))?;

    let abi_json = serde_json::to_string_pretty(&abi)
        .map_err(|e| format!("ABI extraction failed: {}", e))?;
    std::fs::write(entry.join("abi.json"), abi_json)
        .map_err(|e| format!("ABI extraction failed: {}", e))?;

    let art_hash = hash_file(&art_dest);

    let mut symbols: Vec<String> = api.functions.iter().map(|f| f.symbol.clone()).collect();
    // Real-world Phase A: also record the adapter shim symbols so `lime build`
    // can resolve the prepared artifact when the Lime program references them.
    for a in &adapters {
        if !symbols.contains(&a.symbol) {
            symbols.push(a.symbol.clone());
        }
    }

    let manifest = Manifest {
        library: lib_name.clone(),
        version: version.clone(),
        source_origin: source.to_string(),
        source_hash: src_hash,
        dependencies: deps,
        artifact: art_name.clone(),
        artifact_hash: art_hash,
        abi,
        symbols: symbols.clone(),
    };
    let manifest_toml = toml::to_string_pretty(&manifest)
        .map_err(|e| format!("artifact store failed: manifest error: {}", e))?;
    std::fs::write(entry.join("manifest.toml"), manifest_toml)
        .map_err(|e| format!("artifact store failed: {}", e))?;

    Ok(InstallResult {
        lib_name,
        store_path: entry,
        api,
    })
}

fn collect_sources(path: &Path) -> Result<(ApiKind, Vec<PathBuf>, Option<PathBuf>), String> {
    let mut sources = Vec::new();
    let mut header = None;
    let mut has_cpp = false;
    if path.is_file() {
        match path.extension().and_then(|e| e.to_str()) {
            Some("h") | Some("hpp") | Some("hh") => header = Some(path.to_path_buf()),
            Some("c") => sources.push(path.to_path_buf()),
            Some("cpp") | Some("cc") | Some("cxx") => {
                has_cpp = true;
                sources.push(path.to_path_buf())
            }
            _ => return Err("unsupported source file".to_string()),
        }
    } else {
        for entry in std::fs::read_dir(path).map_err(|e| format!("compiler not found / cannot read dir: {}", e))? {
            let p = entry.map_err(|e| e.to_string())?.path();
            match p.extension().and_then(|e| e.to_str()) {
                Some("h") | Some("hpp") | Some("hh") => {
                    if header.is_none() {
                        header = Some(p);
                    }
                }
                Some("c") => sources.push(p),
                Some("cpp") | Some("cc") | Some("cxx") => {
                    has_cpp = true;
                    sources.push(p)
                }
                _ => {}
            }
        }
    }
    let lang = if has_cpp { ApiKind::Cpp } else { ApiKind::C };
    Ok((lang, sources, header))
}

/// #7: strip a recognized C/C++ header extension from a filename stem.
fn strip_header_ext(name: &str) -> String {
    if name.ends_with(".hpp") || name.ends_with(".hh") {
        name[..name.len() - 4].to_string()
    } else if name.ends_with(".h") {
        name[..name.len() - 2].to_string()
    } else {
        name.to_string()
    }
}

/// #7: dependency graph.
///
/// Scans the library's source/header text for local `#include "..."`
/// directives and resolves each included header name to a *prepared* Charger
/// library already present in the store (i.e. one that was `charger install`-ed
/// beforehand). A header `libc_common.h` maps to the library name `libc_common`.
///
/// Returns:
///   * `deps`       — dependency library names recorded in the manifest.
///   * `include_dirs` — directories (`-I`) needed so the dependent compiles
///                      (the dependency's source directory, so its header can
///                      be found by the compiler).
///
/// Detection is conservative: only include names that resolve to an existing
/// store entry and are not the library being installed are treated as
/// dependencies. This keeps the slice honest (no fabricated edges) while
/// extending — not changing — the existing store/hash/lookup foundation.
fn detect_dependencies(
    sources: &[PathBuf],
    header: &Path,
    lib_name: &str,
) -> (Vec<String>, Vec<PathBuf>) {
    let mut deps: Vec<String> = Vec::new();
    let mut include_dirs: Vec<PathBuf> = Vec::new();
    let mut include_seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    let mut candidates: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Collect all source/header text.
    let mut texts: Vec<String> = Vec::new();
    for s in sources {
        if let Ok(t) = std::fs::read_to_string(s) {
            texts.push(t);
        }
    }
    if let Ok(t) = std::fs::read_to_string(header) {
        texts.push(t);
    }

    // Find every local (quoted) include and derive a candidate library name.
    for text in &texts {
        for line in text.lines() {
            let line = line.trim_start();
            if let Some(rest) = line.strip_prefix("#include") {
                // Only quoted includes are local includes (system <...> ignored).
                if let Some(start) = rest.find('"') {
                    let after = &rest[start + 1..];
                    if let Some(end) = after.find('"') {
                        let inc = &after[..end];
                        let file = Path::new(inc)
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let stem = strip_header_ext(&file);
                        if !stem.is_empty() && stem != lib_name {
                            candidates.insert(stem);
                        }
                    }
                }
            }
        }
    }

    // Resolve each candidate against the prepared store.
    for cand in candidates {
        if let Some(dep_entry) = find_artifact_entry(&cand) {
            if let Some(m) = load_manifest(&dep_entry) {
                if !deps.contains(&cand) {
                    deps.push(cand.clone());
                }
                // Add the dependency's source directory as an include path so
                // the dependent's `#include "dep.h"` resolves at compile time.
                let origin = Path::new(&m.source_origin);
                let dir = if origin.is_file() {
                    origin.parent()
                } else {
                    Some(origin)
                };
                if let Some(d) = dir {
                    if include_seen.insert(d.to_path_buf()) {
                        include_dirs.push(d.to_path_buf());
                    }
                }
            }
        }
    }

    (deps, include_dirs)
}

fn detect_abi(llvm_bindir: &str, lang: ApiKind) -> AbiMeta {
    let clang = PathBuf::from(llvm_bindir).join("clang.exe");
    let clang = if clang.exists() {
        clang
    } else {
        PathBuf::from(llvm_bindir).join("clang")
    };
    let version = Command::new(&clang)
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            s.lines().next().map(|l| l.to_string())
        })
        .unwrap_or_default();
    AbiMeta {
        os: if cfg!(windows) { "windows" } else { "linux" }.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        compiler: "clang".to_string(),
        compiler_version: version,
        cxx_abi: if lang == ApiKind::Cpp {
            if cfg!(windows) { "MSVC" } else { "Itanium" }.to_string()
        } else {
            "C".to_string()
        },
        cxx_stdlib: if lang == ApiKind::Cpp {
            if cfg!(windows) { "msvc" } else { "libstdc++" }.to_string()
        } else {
            "-".to_string()
        },
        build_flags: Vec::new(),
    }
}

fn hash_path(p: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    if p.is_dir() {
        // hash directory contents recursively (names + sizes)
        hash_dir(p, &mut h);
    } else {
        if let Ok(data) = std::fs::read(p) {
            data.hash(&mut h);
        }
    }
    format!("{:016x}", h.finish())
}

fn hash_dir(dir: &Path, h: &mut std::collections::hash_map::DefaultHasher) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        let mut entries: Vec<_> = rd.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.path());
        for e in entries {
            let p = e.path();
            p.file_name().hash(h);
            if p.is_dir() {
                hash_dir(&p, h);
            } else if let Ok(meta) = std::fs::metadata(&p) {
                meta.len().hash(h);
            }
        }
    }
}

fn hash_file(p: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    if let Ok(data) = std::fs::read(p) {
        data.hash(&mut h);
    }
    format!("{:016x}", h.finish())
}

/// Find the best (newest version/toolchain) prepared store entry for `lib`,
/// returning its directory path. Shared by `lookup_artifact` / `lookup_iface`.
fn find_artifact_entry(lib: &str) -> Option<PathBuf> {
    let base = store_root().join(lib);
    if !base.exists() {
        return None;
    }
    let mut best: Option<(String, PathBuf)> = None;
    if let Ok(rd) = std::fs::read_dir(&base) {
        for v in rd.filter_map(|e| e.ok()) {
            let vpath = v.path();
            if let Ok(rd2) = std::fs::read_dir(&vpath) {
                for t in rd2.filter_map(|e| e.ok()) {
                    let epath = t.path();
                    if load_manifest(&epath).is_some() {
                        let key = format!("{}/{}", vpath.display(), epath.display());
                        match &best {
                            Some((bk, _)) if *bk >= key => {}
                            _ => best = Some((key, epath)),
                        }
                    }
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Look up a prepared Charger artifact for `lib` and return the path to the
/// native artifact (.lib/.a) if present in the store. Used by lime build.
pub fn lookup_artifact(lib: &str) -> Option<PathBuf> {
    let entry = find_artifact_entry(lib)?;
    let m = load_manifest(&entry)?;
    let art = entry.join(&m.artifact);
    if art.exists() { Some(art) } else { None }
}

/// Load the Lime interface source for a prepared library (used by lime build
/// import resolution).
pub fn lookup_iface(lib: &str) -> Option<String> {
    let entry = find_artifact_entry(lib)?;
    let iface = entry.join("lime-iface.lime");
    std::fs::read_to_string(iface).ok()
}

/// Resolve a prepared Charger library for a lime build. Returns the interface
/// source text and the native artifact path. Errors with a clear message if
/// the library was not `charger install`-ed first (build-time preparation is a
/// required prerequisite; lime build never downloads or compiles C/C++).
pub fn resolve(lib: &str) -> Result<(String, PathBuf), String> {
    let iface = lookup_iface(lib)
        .ok_or_else(|| format!(
            "Charger artifact for '{}' is not prepared. Run `charger install {}` before `lime build`.",
            lib, lib))?;
    let art = lookup_artifact(lib)
        .ok_or_else(|| format!(
            "Charger native artifact for '{}' is missing from the store. Run `charger install {}` again.",
            lib, lib))?;
    Ok((iface, art))
}

fn load_manifest(entry: &Path) -> Option<Manifest> {
    let p = entry.join("manifest.toml");
    let s = std::fs::read_to_string(p).ok()?;
    toml::from_str(&s).ok()
}

/// Given a set of extern symbols referenced by a Lime program, find every
/// prepared Charger native artifact (.lib/.a) whose manifest declares one of
/// those symbols. Returns the distinct artifact paths to inject at link time.
/// This is how `lime build` discovers which prepared libraries to link without
/// requiring any import syntax: the Lime `extern fn` symbol names are matched
/// against the store.
pub fn lookup_artifacts_for_symbols(symbols: &[String]) -> Vec<std::path::PathBuf> {
    let base = store_root();
    let mut out: Vec<std::path::PathBuf> = Vec::new();
    if !base.exists() { return out; }
    if let Ok(rd) = std::fs::read_dir(&base) {
        for lib in rd.filter_map(|e| e.ok()) {
            let libp = lib.path();
            if !libp.is_dir() { continue; }
            if let Ok(rd2) = std::fs::read_dir(&libp) {
                for ver in rd2.filter_map(|e| e.ok()) {
                    let verp = ver.path();
                    if !verp.is_dir() { continue; }
                    if let Ok(rd3) = std::fs::read_dir(&verp) {
                        for hash in rd3.filter_map(|e| e.ok()) {
                            let entry = hash.path();
                            if !entry.is_dir() { continue; }
                            if let Some(m) = load_manifest(&entry) {
                                if m.symbols.iter().any(|s| symbols.contains(s)) {
                                    let art = entry.join(&m.artifact);
                                    if art.exists() && !out.contains(&art) {
                                        out.push(art);
                                    }
                                    // #7: also pull in the artifacts of this
                                    // library's recorded dependencies, so that a
                                    // program referencing only the top-level
                                    // symbol (e.g. `app_compute`) still links
                                    // the transitive native objects it needs
                                    // (e.g. `libc_common`'s `common_add`).
                                    for dep in &m.dependencies {
                                        if let Some(da) = lookup_artifact(dep) {
                                            if !out.contains(&da) {
                                                out.push(da);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

/// List installed libraries (for `charger list`).
pub fn list_installed() -> Vec<String> {
    let base = store_root();
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&base) {
        for e in rd.filter_map(|e| e.ok()) {
            if e.path().is_dir() {
                out.push(e.file_name().to_string_lossy().to_string());
            }
        }
    }
    out
}
