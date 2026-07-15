// Phase 0 (Step 10): LLVM backend foundation (textual IR emitter).
//
// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・LLVM IR (text) 縺ｦ榆帥蜴ｻ縺ｦ縺ｪ縺ｿ縺ｪ縺・
// Inkwell / llvm-sys 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・蟋ｩ蜉ｨ蝗ｲ繧縺ｮ縺ｿ縺ｪ縺・繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
// (System LLVM 縺ｯ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・)
// 蝣ｴ蜷隗｣譫怜ｸ・縺ｮ縺ｿ縺ｪ縺・(aggregates) 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
// 壓ｵ蜈ｰ縺ｯ縺ｪ縺ｿ縺ｪ縺・ Phase 1+ 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
//
// Design (docs/llvm_backend.md) 縺ｮ codegen/ 縺ｯ繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・:
//   mod.rs (Context/Module/Builder-like) / types.rs (型マッピング) /
//   fn_builder.rs / structs.rs / calls.rs / generic.rs / interface.rs /
//   async_rt.rs / runtime/ ... (Phase 1+ 縺ｯ頒ｦ)

pub mod types;
pub mod fn_builder;

use crate::Defs;
use crate::Stmt;
use crate::Expr;
use crate::FunctionDef;
use crate::MemoryPlace;
use crate::type_from_str;
use std::collections::HashMap;
use crate::codegen::types::llvm_type_name;

const DEFAULT_TARGET_TRIPLE: &str = "x86_64-pc-windows-msvc";
const LIME_MAIN: &str = "main_lime";

pub fn emit_llvm(stmts: &[Stmt], defs: &Defs, memory: &HashMap<String, MemoryPlace>) -> String {
    let _ = stmts;
    let mut out = String::new();

    out.push_str("; ModuleID = 'lime'\n");
    out.push_str("source_filename = \"lime\"\n");
    out.push_str(&format!("target triple = \"{}\"\n\n", DEFAULT_TARGET_TRIPLE));

    // Runtime declarations
    out.push_str("declare i8* @runtime_alloc(i64, i64)\n");
    out.push_str("declare void @runtime_print(i8*)\n");
    // Printf for print/println builtin lowering (Phase 2)
    out.push_str("declare i32 @printf(i8*, ...)\n\n");

    // Phase 5: String/List runtime function declarations
    out.push_str("declare i64 @strlen(i8*)\n");
    out.push_str("declare i8* @runtime_str_slice(i8*, i64, i64)\n");
    out.push_str("declare i8* @runtime_str_concat(i8*, i8*)\n");
    out.push_str("declare %LimeList @runtime_str_chars(i8*)\n");
    out.push_str("declare %LimeList @runtime_str_bytes(i8*)\n");
    out.push_str("declare %LimeList @runtime_list_add(%LimeList, i64)\n");
    out.push_str("declare %LimeList @runtime_list_set(%LimeList, i64, i64)\n\n");

    // Format strings for print/println builtin lowering (Phase 2)
    out.push_str("@.str.int   = private unnamed_addr constant [5 x i8] c\"%lld\\00\"\n");
    out.push_str("@.str.int_nl = private unnamed_addr constant [6 x i8] c\"%lld\\0A\\00\"\n");
    out.push_str("@.str.float  = private unnamed_addr constant [3 x i8] c\"%g\\00\"\n");
    out.push_str("@.str.float_nl = private unnamed_addr constant [4 x i8] c\"%g\\0A\\00\"\n");
    out.push_str("@.str.str    = private unnamed_addr constant [3 x i8] c\"%s\\00\"\n");
    out.push_str("@.str.str_nl = private unnamed_addr constant [4 x i8] c\"%s\\0A\\00\"\n");
    out.push_str("@.str.true   = private unnamed_addr constant [5 x i8] c\"true\\00\"\n");
    out.push_str("@.str.false  = private unnamed_addr constant [6 x i8] c\"false\\00\"\n\n");

    // Aggregate type declarations
    emit_aggregate_decls(&mut out, defs);

    // Phase 5: collect string literals and emit globals
    let string_literals = collect_string_literals(defs);
    for (s, name) in &string_literals {
        let len = s.len() + 1;
        let escaped = escape_llvm_string(s);
        out.push_str(&format!(
            "@{} = private unnamed_addr constant [{} x i8] c\"{}\\00\"\n",
            name, len, escaped
        ));
    }
    if !string_literals.is_empty() {
        out.push('\n');
    }

    // Phase 6: Generic monomorphization - collect generic calls and create monomorphized copies
    let (mono_name_map, mono_fdefs) = monomorphize_all(defs);

    // Emit monomorphized function definitions first
    for (mangled, mono_fdef) in &mono_fdefs {
        let func_ir = fn_builder::codegen_function(defs, memory, &string_literals, &mono_name_map, &mono_fdefs, mangled, mono_fdef);
        out.push_str(&func_ir);
    }

    // Function definitions using fn_builder (Phase 1) - skip generic templates
    for (name, fdef) in &defs.functions {
        if !fdef.type_params.is_empty() {
            continue;
        }
        let func_ir = fn_builder::codegen_function(defs, memory, &string_literals, &mono_name_map, &mono_fdefs, name, fdef);
        out.push_str(&func_ir);
    }

    // Phase 7: Struct method function definitions
    for (sname, sdef) in &defs.structs {
        for (mname, mdef) in &sdef.methods {
            if !mdef.type_params.is_empty() {
                continue;
            }
            let method_func_name = format!("{}_{}", sname, mname);
            // Prepend self parameter (struct type name, not LLVM IR notation)
            let mut params = vec![(String::from("self"), sname.clone())];
            params.extend(mdef.params.clone());
            let method_fdef = FunctionDef {
                type_params: Vec::new(),
                constraints: Vec::new(),
                params,
                return_type: mdef.return_type.clone(),
                body: mdef.body.clone(),
                is_async: false,
            };
            let func_ir = fn_builder::codegen_function(defs, memory, &string_literals, &mono_name_map, &mono_fdefs, &method_func_name, &method_fdef);
            out.push_str(&func_ir);
        }
    }

    // C runtime main wrapper
    emit_main_wrapper(&mut out, defs);

    out
}

/// Phase 5: escape a string for LLVM IR constant format.
fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            0 => out.push_str("\\00"),
            b'\\' => out.push_str("\\5C"),
            b'"' => out.push_str("\\22"),
            b'\n' => out.push_str("\\0A"),
            b'\r' => out.push_str("\\0D"),
            b'\t' => out.push_str("\\09"),
            0x20..=0x7E => out.push(b as char),
            _ => out.push_str(&format!("\\{:02X}", b)),
        }
    }
    out
}

/// Phase 5: pre-scan all function bodies for Expr::StringLit, collect unique strings.
fn collect_string_literals(defs: &Defs) -> HashMap<String, String> {
    let mut strings = HashMap::new();
    let mut idx = 0usize;
    for (_, fdef) in &defs.functions {
        collect_strings_from_stmts(&fdef.body, &mut strings, &mut idx);
    }
    // Phase 7: also scan struct method bodies
    for (_, sdef) in &defs.structs {
        for (_, mdef) in &sdef.methods {
            collect_strings_from_stmts(&mdef.body, &mut strings, &mut idx);
        }
    }
    strings
}

fn collect_strings_from_stmts(stmts: &[Stmt], strings: &mut HashMap<String, String>, idx: &mut usize) {
    for s in stmts {
        match s {
            Stmt::Let { value, .. } => collect_strings_from_expr(value, strings, idx),
            Stmt::Return(e) => {
                if let Some(e) = e {
                    collect_strings_from_expr(e, strings, idx);
                }
            }
            Stmt::Expr(e) => collect_strings_from_expr(e, strings, idx),
            Stmt::Assign { value, .. } => collect_strings_from_expr(value, strings, idx),
            Stmt::If { cond, then_branch, else_branch } => {
                collect_strings_from_expr(cond, strings, idx);
                collect_strings_from_stmts(then_branch, strings, idx);
                if let Some(eb) = else_branch {
                    collect_strings_from_stmts(eb, strings, idx);
                }
            }
            Stmt::While { cond, body } => {
                collect_strings_from_expr(cond, strings, idx);
                collect_strings_from_stmts(body, strings, idx);
            }
            Stmt::For { iterable, body, .. } => {
                collect_strings_from_expr(iterable, strings, idx);
                collect_strings_from_stmts(body, strings, idx);
            }
            Stmt::Match { expr, arms } => {
                collect_strings_from_expr(expr, strings, idx);
                for (_, body) in arms {
                    collect_strings_from_stmts(body, strings, idx);
                }
            }
            _ => {}
        }
    }
}

fn collect_strings_from_expr(e: &Expr, strings: &mut HashMap<String, String>, idx: &mut usize) {
    match e {
        Expr::StringLit(s) => {
            if !strings.contains_key(s) {
                let name = format!(".str.{}", *idx);
                *idx += 1;
                strings.insert(s.clone(), name);
            }
        }
        Expr::BinOp { left, right, .. } => {
            collect_strings_from_expr(left, strings, idx);
            collect_strings_from_expr(right, strings, idx);
        }
        Expr::UnOp { operand, .. } => collect_strings_from_expr(operand, strings, idx),
        Expr::Call { args, .. } => {
            for a in args {
                collect_strings_from_expr(a, strings, idx);
            }
        }
        Expr::MethodCall { object, args, .. } => {
            collect_strings_from_expr(object, strings, idx);
            for a in args {
                collect_strings_from_expr(a, strings, idx);
            }
        }
        Expr::FieldAccess { object, .. } => collect_strings_from_expr(object, strings, idx),
        Expr::Array(items) => {
            for it in items {
                collect_strings_from_expr(it, strings, idx);
            }
        }
        Expr::Range { start, end } => {
            collect_strings_from_expr(start, strings, idx);
            collect_strings_from_expr(end, strings, idx);
        }
        Expr::Await(inner) => collect_strings_from_expr(inner, strings, idx),
        _ => {}
    }
}

// Phase 6: Generic monomorphization.
// Parse "func(i64, str)" into ("func", ["i64", "str"])
fn parse_generic_call_name(func: &str) -> Option<(&str, Vec<&str>)> {
    if let Some(paren_idx) = func.find('(') {
        let base = &func[..paren_idx];
        let inner = &func[paren_idx + 1..func.len() - 1];
        let args: Vec<&str> = if inner.is_empty() {
            Vec::new()
        } else {
            inner.split(", ").collect()
        };
        Some((base, args))
    } else {
        None
    }
}

fn monomorphize_type_str(t: &str, type_params: &[String], type_args: &[&str]) -> String {
    let mut result = t.to_string();
    for (i, tp) in type_params.iter().enumerate() {
        if i < type_args.len() {
            result = result.replace(tp.as_str(), type_args[i]);
        }
    }
    result
}

fn monomorphize_function(fdef: &FunctionDef, type_params: &[String], type_args: &[&str]) -> FunctionDef {
    let params: Vec<(String, String)> = fdef.params.iter()
        .map(|(n, t)| (n.clone(), monomorphize_type_str(t, type_params, type_args)))
        .collect();
    let return_type = fdef.return_type.as_ref()
        .map(|rt| monomorphize_type_str(rt, type_params, type_args));
    FunctionDef {
        type_params: Vec::new(),
        constraints: Vec::new(),
        params,
        return_type,
        body: fdef.body.clone(),
        is_async: fdef.is_async,
    }
}

fn mangled_name(base: &str, type_args: &[&str]) -> String {
    format!("{}.{}", base, type_args.join("."))
}

/// Scan all function bodies for generic calls and monomorphize them.
fn monomorphize_all(defs: &Defs) -> (HashMap<String, String>, HashMap<String, FunctionDef>) {
    let mut mono_name_map: HashMap<String, String> = HashMap::new();
    let mut mono_fdefs: HashMap<String, FunctionDef> = HashMap::new();

    let mut worklist: Vec<String> = defs.functions.keys().cloned().collect();
    let mut done: HashMap<String, bool> = HashMap::new();

    while let Some(name) = worklist.pop() {
        if done.contains_key(&name) { continue; }
        done.insert(name.clone(), true);

        let fdef = match defs.functions.get(&name) {
            Some(f) => f,
            None => continue,
        };

        collect_mono_from_stmts(&fdef.body, defs, &mut mono_name_map, &mut mono_fdefs, &mut worklist);
    }

    (mono_name_map, mono_fdefs)
}

fn collect_mono_from_stmts(
    stmts: &[Stmt],
    defs: &Defs,
    mono_name_map: &mut HashMap<String, String>,
    mono_fdefs: &mut HashMap<String, FunctionDef>,
    worklist: &mut Vec<String>,
) {
    for s in stmts {
        match s {
            Stmt::Let { value, .. } => collect_mono_from_expr(value, defs, mono_name_map, mono_fdefs, worklist),
            Stmt::Return(e) => {
                if let Some(e) = e {
                    collect_mono_from_expr(e, defs, mono_name_map, mono_fdefs, worklist);
                }
            }
            Stmt::Expr(e) => collect_mono_from_expr(e, defs, mono_name_map, mono_fdefs, worklist),
            Stmt::Assign { value, .. } => collect_mono_from_expr(value, defs, mono_name_map, mono_fdefs, worklist),
            Stmt::If { cond, then_branch, else_branch } => {
                collect_mono_from_expr(cond, defs, mono_name_map, mono_fdefs, worklist);
                collect_mono_from_stmts(then_branch, defs, mono_name_map, mono_fdefs, worklist);
                if let Some(eb) = else_branch {
                    collect_mono_from_stmts(eb, defs, mono_name_map, mono_fdefs, worklist);
                }
            }
            Stmt::While { cond, body } => {
                collect_mono_from_expr(cond, defs, mono_name_map, mono_fdefs, worklist);
                collect_mono_from_stmts(body, defs, mono_name_map, mono_fdefs, worklist);
            }
            Stmt::For { iterable, body, .. } => {
                collect_mono_from_expr(iterable, defs, mono_name_map, mono_fdefs, worklist);
                collect_mono_from_stmts(body, defs, mono_name_map, mono_fdefs, worklist);
            }
            Stmt::Match { expr, arms } => {
                collect_mono_from_expr(expr, defs, mono_name_map, mono_fdefs, worklist);
                for (_, body) in arms {
                    collect_mono_from_stmts(body, defs, mono_name_map, mono_fdefs, worklist);
                }
            }
            _ => {}
        }
    }
}

fn collect_mono_from_expr(
    e: &Expr,
    defs: &Defs,
    mono_name_map: &mut HashMap<String, String>,
    mono_fdefs: &mut HashMap<String, FunctionDef>,
    worklist: &mut Vec<String>,
) {
    match e {
        Expr::Call { func, args } => {
            // Check if this is a generic call like "max(i64)"
            if let Some((base, type_strs)) = parse_generic_call_name(func) {
                if let Some(fdef) = defs.functions.get(base) {
                    if !fdef.type_params.is_empty() {
                        let call_name = func.clone();
                        if !mono_name_map.contains_key(&call_name) {
                            let mangled = mangled_name(base, &type_strs);
                            let type_params: Vec<&str> = type_strs.iter().map(|s| *s).collect();
                            let mono = monomorphize_function(fdef, &fdef.type_params, &type_params);
                            mono_name_map.insert(call_name.clone(), mangled.clone());
                            mono_fdefs.insert(mangled.clone(), mono);
                            worklist.push(mangled);
                        }
                    }
                }
            }
            for a in args {
                collect_mono_from_expr(a, defs, mono_name_map, mono_fdefs, worklist);
            }
        }
        Expr::BinOp { left, right, .. } => {
            collect_mono_from_expr(left, defs, mono_name_map, mono_fdefs, worklist);
            collect_mono_from_expr(right, defs, mono_name_map, mono_fdefs, worklist);
        }
        Expr::UnOp { operand, .. } => collect_mono_from_expr(operand, defs, mono_name_map, mono_fdefs, worklist),
        Expr::MethodCall { object, args, .. } => {
            collect_mono_from_expr(object, defs, mono_name_map, mono_fdefs, worklist);
            for a in args {
                collect_mono_from_expr(a, defs, mono_name_map, mono_fdefs, worklist);
            }
        }
        Expr::FieldAccess { object, .. } => collect_mono_from_expr(object, defs, mono_name_map, mono_fdefs, worklist),
        Expr::Array(items) => {
            for it in items {
                collect_mono_from_expr(it, defs, mono_name_map, mono_fdefs, worklist);
            }
        }
        Expr::Range { start, end } => {
            collect_mono_from_expr(start, defs, mono_name_map, mono_fdefs, worklist);
            collect_mono_from_expr(end, defs, mono_name_map, mono_fdefs, worklist);
        }
        Expr::Await(inner) => collect_mono_from_expr(inner, defs, mono_name_map, mono_fdefs, worklist),
        _ => {}
    }
}

/// 蝣ｴ蜷隗｣譫怜ｸ・縺ｮ縺ｪ縺ｿ縺ｪ縺・定義: struct / state / list / option / interface
fn emit_aggregate_decls(out: &mut String, defs: &Defs) {
    // struct declarations with real field types (Phase 3)
    for (sname, sdef) in &defs.structs {
        let field_tys: Vec<String> = sdef
            .fields
            .iter()
            .map(|(_, ftype)| llvm_type_name(&type_from_str(ftype, defs)))
            .collect();
        out.push_str(&format!("%{} = type {{ {} }}\n", sname, field_tys.join(", ")));
    }
    // state declarations as tagged unions (Phase 4)
    for sname in defs.states.keys() {
        out.push_str(&format!("%{} = type {{ i32, [4 x i64] }}\n", sname));
    }
    // 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・ runtime 型
    out.push_str("%LimeList = type { i8*, i64, i64 }\n");
    out.push_str("%LimeOption = type { i1, i8* }\n");
    out.push_str("%LimeIface = type { i8*, i8* }\n");
    out.push('\n');
}

fn emit_main_wrapper(out: &mut String, defs: &Defs) {
    if !defs.functions.contains_key("main") {
        return;
    }
    out.push_str("define i32 @main() {\n");
    out.push_str(&format!("  call void @{}()\n", LIME_MAIN));
    out.push_str("  ret i32 0\n");
    out.push_str("}\n\n");
}
