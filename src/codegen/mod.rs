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

use crate::Defs;
use crate::Stmt;
use crate::Type;
use types::llvm_type_name;

// 蝗ｲ繧險ｱ蜿ｯ target triple (Phase 8 縺ｯ TargetMachine 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・)
const DEFAULT_TARGET_TRIPLE: &str = "x86_64-pc-windows-msvc";

/// Lime 縺ｮ main 縺ｯ C runtime 縺ｮ main 縺ｾ縺ｧ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・
/// (Lime main 縺ｯ void 縺ｾ縺ｧ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｯ、C main 縺ｯ i32 縺ｾ縺ｧ繧｢蜷阪→)
const LIME_MAIN: &str = "main_lime";

/// AST + Defs 縺ｯ蛻・蜈･ LLVM IR (text) 縺ｦ股�ｸｦ縺ｧ繧｢蜷阪→。
pub fn emit_llvm(_stmts: &[Stmt], defs: &Defs) -> String {
    let mut out = String::new();

    out.push_str("; ModuleID = 'lime'\n");
    out.push_str("source_filename = \"lime\"\n");
    out.push_str(&format!("target triple = \"{}\"\n\n", DEFAULT_TARGET_TRIPLE));

    // 蝣ｴ蜷隗｣譫怜ｸ・縺ｮ縺ｿ縺ｪ縺・定義 (aggregates 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・)
    emit_aggregate_decls(&mut out, defs);

    // 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｿ縺ｪ縺・(Phase 0: 縺ｮ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・ ret <zero> 縺ｾ縺ｧ繧｢蜷阪→)
    for (name, fdef) in &defs.functions {
        emit_function(&mut out, name, fdef, defs);
    }

    // C runtime main 縺ｯ Lime main 縺ｦ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・
    emit_main_wrapper(&mut out, defs);

    out
}

/// 蝣ｴ蜷隗｣譫怜ｸ・縺ｮ縺ｪ縺ｿ縺ｪ縺・定義: struct / state / list / option / interface
fn emit_aggregate_decls(out: &mut String, defs: &Defs) {
    // struct / state 縺ｯ %Name = type { i64 } 縺ｾ縺ｧ繧｢蜷阪→(Phase 0 placeholder)
    // (Phase 2/4 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｪ縺ｿ縺ｪ縺・蝣ｴ蜷隗｣譫怜ｸ・縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｿ縺ｪ縺・)
    for sname in defs.structs.keys() {
        out.push_str(&format!("%{} = type {{ i64 }}\n", sname));
    }
    for sname in defs.states.keys() {
        out.push_str(&format!("%{} = type {{ i64 }}\n", sname));
    }
    // 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・ runtime 型
    out.push_str("%LimeList = type { i8*, i64, i64 }\n");
    out.push_str("%LimeOption = type { i1, i8* }\n");
    out.push_str("%LimeIface = type { i8*, i8* }\n");
    out.push('\n');
}

/// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・定義 (Phase 0: 縺ｮ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・ ret <zero>)
fn emit_function(out: &mut String, name: &str, fdef: &crate::FunctionDef, defs: &Defs) {
    // 忣削蜈ｰ縺ｯ Lime main 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・ C main 縺ｾ縺ｧ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・
    // 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・名縺ｯ "main" 縺ｯ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・ main_lime 縺ｾ縺ｧ繧｢蜷阪→
    let llvm_name = if name == "main" {
        LIME_MAIN.to_string()
    } else {
        name.to_string()
    };

    let ret_ty = match &fdef.return_type {
        Some(rt) => llvm_type_name(&type_from_str(rt, defs)),
        None => "void".to_string(),
    };

    // 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・(値型縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・ Phase 0)
    let mut params = Vec::new();
    for (_, ptype) in &fdef.params {
        params.push(llvm_type_name(&type_from_str(ptype, defs)));
    }
    let params_str = params.join(", ");
    let async_note = if fdef.is_async { "; async (lime)" } else { "" };

    out.push_str(&format!(
        "; Function {}(){}\ndefine {} @{} ({}) {{\n",
        name, async_note, ret_ty, llvm_name, params_str
    ));

    // Phase 0: 縺ｮ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｯ ret <zero> 縺ｾ縺ｧ繧｢蜷阪→
    if ret_ty == "void" {
        out.push_str("  ret void\n");
    } else {
        out.push_str(&format!("  ret {} {}\n", ret_ty, zero_value(&ret_ty)));
    }
    out.push_str("}\n\n");
}

/// C runtime main 縺ｯ Lime main 縺ｦ蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・
fn emit_main_wrapper(out: &mut String, defs: &Defs) {
    if !defs.functions.contains_key("main") {
        return;
    }
    out.push_str("define i32 @main() {\n");
    out.push_str(&format!("  call void @{}()\n", LIME_MAIN));
    out.push_str("  ret i32 0\n");
    out.push_str("}\n\n");
}

/// 型名縺ｯ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・ zero literal 縺ｦ股�ｸｦ縺ｧ繧｢蜷阪→
fn zero_value(ty: &str) -> &'static str {
    match ty {
        "i64" => "0",
        "double" => "0.0",
        "i1" => "false",
        "void" => "void",
        _ => "null", // 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・(i8* 蜈ｷ雎｡)
    }
}

/// 縺ｮ縺ｿ縺ｪ縺・縺ｯ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・ Type 縺ｦ股�ｸｦ縺ｧ繧｢蜷阪→(crate::type_from_str 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・)
fn type_from_str(s: &str, defs: &Defs) -> Type {
    crate::type_from_str(s, defs)
}
