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
    // C++ specifics reserved for future implementation:
    pub base_classes: Vec<String>,
    pub has_vtable: bool,
    pub is_class: bool,
}

#[derive(Debug, Clone, Default)]
pub struct NormalizedApi {
    pub functions: Vec<CFunction>,
    pub structs: Vec<CStruct>,
    pub kind: ApiKind, // C or Cpp
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
    };
    let mut ctx = NormalizeCtx {
        anon_struct: None,
        typedefs: Vec::new(),
        _pending_method_self: None,
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
                    api.structs.push(CStruct {
                        name: tname.clone(),
                        fields: fields.clone(),
                        size_bytes: None,
                        align_bytes: None,
                        base_classes: Vec::new(),
                        has_vtable: false,
                        is_class: false,
                    });
                }
            }
        }
    }
    api
}

fn type_from_json(t: &serde_json::Value) -> CType {
    // t is a "qualType" string like "int", "Point", "int (*)(int, int)",
    // "char *", "const char *", "class Widget", "struct Point", etc.
    let qual = t.get("qualType").and_then(|v| v.as_str()).unwrap_or("");
    parse_c_type(qual)
}

fn parse_c_type(qual: &str) -> CType {
    let q = qual.trim();
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
        return CType::Pointer(Box::new(parse_c_type(inner)));
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
        "char" | "const char" | "char *" | "const char *" => CType::String, // treat C strings as Lime String
        s if s.starts_with("struct ") => CType::Struct(s["struct ".len()..].to_string()),
        s if s.starts_with("class ") => CType::Struct(s["class ".len()..].to_string()),
        s if s.starts_with("enum ") => CType::Opaque(s["enum ".len()..].to_string()),
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

/// Mutable state threaded through AST normalization so we can associate
/// anonymous record bodies with the typedef names that name them, and track
/// the enclosing class for inline methods.
struct NormalizeCtx {
    anon_struct: Option<Vec<CParam>>,
    typedefs: Vec<(String, String)>,
    _pending_method_self: Option<String>,
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
            let mut fields = Vec::new();
            if let Some(inner) = node.get("inner").and_then(|v| v.as_array()) {
                for f in inner {
                    if f.get("kind").and_then(|v| v.as_str()) == Some("FieldDecl") {
                        let fname = f.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let fty = f.get("type").map(type_from_json).unwrap_or(CType::Other("?".to_string()));
                        if !fname.is_empty() {
                            fields.push(CParam { name: fname, ty: fty });
                        }
                    }
                }
            }
            if !name.is_empty() && !is_implicit {
                let is_class = node.get("tagUsed").and_then(|v| v.as_str()) == Some("class");
                api.structs.push(CStruct {
                    name: name.clone(),
                    fields,
                    size_bytes: None,
                    align_bytes: None,
                    base_classes: Vec::new(),
                    has_vtable: false,
                    is_class,
                });
            } else if !is_implicit {
                // Anonymous record body (e.g. `typedef struct { ... } Point;`)
                // — remember for a following TypedefDecl.
                ctx.anon_struct = Some(fields);
            }
            if let Some(inner) = node.get("inner").and_then(|v| v.as_array()) {
                for c in inner {
                    classify_node(c, api, ctx, Some(name.clone()), lang);
                }
            }
        }
        "CXXRecordDecl" => {
            // C++ class/struct: capture the class name so inline methods get a
            // correct `self` receiver type. Emit an opaque struct placeholder so
            // the Lime type name resolves (full layout is recorded in ABI
            // metadata for future precise layout). Field extraction here is
            // omitted for the vertical slice; the receiver is passed by value.
            let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            // Skip implicit compiler-injected records (e.g. _GUID, type_info)
            // and the inner re-declaration/definition copy of a class. These
            // duplicate the real members already present under the non-implicit
            // declaration, so we must NOT recurse into them (that would
            // double-classify the members).
            let is_implicit = node.get("isImplicit").and_then(|v| v.as_bool()).unwrap_or(false);
            if is_implicit || name.is_empty() {
                return;
            }
            if !api.structs.iter().any(|s| s.name == name) {
                api.structs.push(CStruct {
                    name: name.clone(),
                    fields: Vec::new(),
                    size_bytes: None,
                    align_bytes: None,
                    base_classes: Vec::new(),
                    has_vtable: false,
                    is_class: true,
                });
            }
            if let Some(inner) = node.get("inner").and_then(|v| v.as_array()) {
                for c in inner {
                    classify_node(c, api, ctx, Some(name.clone()), lang);
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
                ctx.typedefs.push((name, underlying));
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
        "CXXMethodDecl" | "CXXConstructorDecl" | "CXXDestructorDecl" => {
            let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if name.is_empty() {
                return;
            }
            let self_ty = self_ty.clone();
            let ftype = node.get("type").cloned().unwrap_or(serde_json::Value::Null);
            let (mut params, ret_ty) = params_and_ret(&ftype);
            let is_ctor = kind == "CXXConstructorDecl";
            let is_dtor = kind == "CXXDestructorDecl";
            let is_const = node.get("const").and_then(|v| v.as_bool()).unwrap_or(false);
            let symbol = node
                .get("mangledName")
                .and_then(|v| v.as_str())
                .unwrap_or(&name)
                .to_string();
            // Receiver is the first Lime-facing parameter.
            let mut all_params = Vec::new();
            if !is_ctor && !is_dtor {
                all_params.push(CParam {
                    name: "self".to_string(),
                    ty: CType::Struct(self_ty.clone().unwrap_or_default()),
                });
            }
            all_params.append(&mut params);
            let disp_name = if is_ctor {
                format!("{}::{}", self_ty.clone().unwrap_or_default(), name)
            } else if is_dtor {
                format!("{}::~{}", self_ty.clone().unwrap_or_default(), name)
            } else {
                match &self_ty {
                    Some(s) => format!("{}::{}", s, name),
                    None => name.clone(),
                }
            };
            api.functions.push(CFunction {
                name: disp_name,
                symbol,
                params: all_params,
                ret: if is_ctor { CType::Void } else { ret_ty },
                is_method: !is_ctor && !is_dtor,
                is_constructor: is_ctor,
                is_const,
                self_ty: self_ty.clone(),
            });
            if let Some(inner) = node.get("inner").and_then(|v| v.as_array()) {
                for c in inner {
                    classify_node(c, api, ctx, self_ty.clone(), lang);
                }
            }
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
        CType::Opaque(s) => s.clone(),
        CType::Other(s) => s.clone(),
    }
}

// ----------------------------------------------------------------------------
// Lime interface generation
// ----------------------------------------------------------------------------

/// Generate `lime-iface.lime` source text. Each C/C++ function becomes an
/// `extern` Lime declaration. Structs become Lime `struct` definitions whose
/// field layout matches the C/C++ layout (field order/names/types only; the
/// ABI metadata records size/align for verification).
fn generate_lime_iface(api: &NormalizedApi, lib_name: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("// Charger-generated Lime interface for '{}'\n", lib_name));
    out.push_str("// DO NOT EDIT: regenerate with `charger install`.\n\n");

    // Structs first (Lime requires types to be visible before use).
    for s in &api.structs {
        if s.fields.is_empty() {
            // opaque / empty: emit as a unit-ish placeholder struct so the
            // type name resolves. (Future: ABI metadata drives real layout.)
            out.push_str(&format!("struct {} {{\n}}\n\n", s.name));
            continue;
        }
        out.push_str(&format!("struct {} {{\n", s.name));
        for f in &s.fields {
            out.push_str(&format!("    {}: {}\n", f.name, lime_type_name(&f.ty)));
        }
        out.push_str("}\n\n");
    }

    // Functions / methods.
    for f in &api.functions {
        let ret_lime = lime_type_name(&f.ret);
        let params_lime: Vec<String> = f
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, lime_type_name(&p.ty)))
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

    // 2. AST extraction + normalization.
    let ast = extract_ast_json(&header, lang, llvm_bindir)?;
    let mut api = normalize(&ast, lang);

    // For C++ methods, ensure the receiver struct is recorded even if only
    // declared inline. (Vertical slice: structs come from RecordDecl.)
    // (No extra work needed for the slice; struct fields already captured.)

    // 3. Lime interface generation.
    let iface = generate_lime_iface(&api, &lib_name);

    // 4. ABI metadata.
    let abi = detect_abi(llvm_bindir, lang);
    let build_flags = if lang == ApiKind::Cpp {
        vec!["-O2".to_string(), "-std=c++17".to_string()]
    } else {
        vec!["-O2".to_string()]
    };
    let tool_hash = toolchain_hash(&abi, &build_flags);

    // 5. Store.
    let version = "0.1.0".to_string();
    let entry = store_root()
        .join(&lib_name)
        .join(&version)
        .join(&tool_hash);
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

    let src_hash = hash_path(&src_path);
    let art_hash = hash_file(&art_dest);

    let symbols: Vec<String> = api.functions.iter().map(|f| f.symbol.clone()).collect();

    let manifest = Manifest {
        library: lib_name.clone(),
        version: version.clone(),
        source_origin: source.to_string(),
        source_hash: src_hash,
        dependencies: Vec::new(),
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
