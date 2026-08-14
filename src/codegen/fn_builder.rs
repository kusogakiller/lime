// Phase 1 (Step 10): LLVM backend - function/statement/expression codegen.
//
// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・(Step 1-7) 縺ｦ文本 IR 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
//   Step 1: 蜈ｷ蜊･阪�ｸｦ/蜈ｷ遘ｻ/boolean 縺ｮ Constant
//   Step 2: let -> alloca/store/load (Memory Analysis 縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・蝗ｲ繧)
//   Step 3: 蜈ｷ隹ｿ演算 (+ - * / %) 縺ｮ Builtin 型縺の明縺ｮ縺ｪ縺ｿ縺ｪ縺・
//   Step 4: 斎惻演算 (== != < > <= >=) -> icmp / fcmp
//   Step 5: 輔旓演算 (and or not) -> 蜈ｷ遨ｺ繧ｳ繝｡繝ｳ繝・阜ｼ (short circuit)
//   Step 6: if -> BasicBlock (entry/then/else/merge)
//   Step 7: while -> CFG (cond/body/merge)
//
// 蝣ｴ蜷隗｣譫怜ｸ・縺ｮ縺ｿ縺ｪ縺・(Struct/State/Match/Generic/Interface/Async/List/
// String Runtime 縺ｯ Phase 1 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・駘ｽ蜿ｪ縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・蝗ｲ繧縺ｮ縺ｿ縺ｪ縺・蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｿ縺ｪ縺・縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
// (Phase 0 縺ｮ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・ ret <zero> 縺ｾ縺ｧ繧｢蜷阪→)

use crate::Defs;
use crate::Expr;
use crate::Stmt;
use crate::Type;
use crate::FunctionDef;
use crate::StructDef;
use crate::Pattern;
use crate::MemoryPlace;
use crate::type_from_str;
use crate::ResolvedOperator;
use std::collections::HashMap;
use super::types::{llvm_type_name, is_float, align_of, zero_value_for_type};

struct Cg<'a> {
    out: String,
    defs: &'a Defs,
    memory: &'a HashMap<String, MemoryPlace>,
    string_literals: &'a HashMap<String, String>,
    mono_name_map: &'a HashMap<String, String>,
    mono_fdefs: &'a HashMap<String, FunctionDef>,
    env: HashMap<String, Type>,
    named: HashMap<String, String>,
    current_block: String,
    temp: usize,
    block: usize,
    terminated: bool,
    warnings: Vec<String>,
    fn_ret_ty: String,
    pending_fns: Vec<String>,
    anon_count: usize,
    /// Phase B.1: string-length trackers (ACTIVE). Maps a string variable name
    /// to the alloca (i64*) holding its currently-known length. ONLY populated
    /// once we actually see `var = var.push_byte(...)` on a pending variable.
    string_len_trackers: HashMap<String, String>,
    /// Phase B.1: PENDING trackers. A variable initialized from a string literal
    /// is registered here (len alloca created) but NOT yet active — it only
    /// becomes active when we see `var = var.push_byte(...)`. This keeps ordinary
    /// literal-init variables (e.g. `let s = "hello"; s.length()`) lowering to
    /// strlen as before, preserving existing codegen tests.
    pending_len_trackers: HashMap<String, String>,
    /// Value-range analysis for integer i32-narrowing optimization.
    /// Maps a variable name to the known (min, max) range of its currently
    /// stored integer value. Used to decide when integer arithmetic can be
    /// safely emitted in i32 (then `sext` back to i64) so LLVM vectorizes
    /// tight loops the way Clang -O3 does. `None` (absent) means unknown /
    /// possibly-too-large-to-fit-i32 → do not narrow.
    var_range: HashMap<String, (i128, i128)>,
    /// Active `while` loop induction-variable info (for trip-count-based
    /// accumulation range bounds). `loop_counter` is the loop variable name,
    /// `loop_bound` its constant upper bound, `loop_counter_init` its known
    /// starting value.
    loop_counter: Option<String>,
    loop_bound: Option<i128>,
    loop_counter_init: i128,
    /// True when the current `while` loop is pure integer arithmetic (no
    /// method/function calls in its body). Used to gate the i32-narrowing and
    /// i32-indvar optimizations, which would otherwise break runtime-call
    /// inlining (e.g. `runtime_list_add`) in collection loops like `set_ops`.
    loop_pure_arith: bool,
    /// Maps an i64 SSA value (the `sext i32 -> i64` result of a narrowed op)
    /// back to its i32 source, so chained arithmetic stays in i32 without a
    /// sext/trunc round-trip that would defeat LLVM's auto-vectorizer.
    i32_form: HashMap<String, String>,
}

impl<'a> Cg<'a> {
    fn new(defs: &'a Defs, memory: &'a HashMap<String, MemoryPlace>, string_literals: &'a HashMap<String, String>, mono_name_map: &'a HashMap<String, String>, mono_fdefs: &'a HashMap<String, FunctionDef>) -> Self {
        Cg {
            out: String::new(),
            defs,
            memory,
            string_literals,
            mono_name_map,
            mono_fdefs,
            env: HashMap::new(),
            named: HashMap::new(),
            current_block: String::new(),
            temp: 0,
            block: 0,
            terminated: false,
            warnings: Vec::new(),
            fn_ret_ty: "void".to_string(),
            pending_fns: Vec::new(),
            anon_count: 0,
            string_len_trackers: HashMap::new(),
            pending_len_trackers: HashMap::new(),
            var_range: HashMap::new(),
            loop_counter: None,
            loop_bound: None,
            loop_counter_init: 0,
            loop_pure_arith: false,
            i32_form: HashMap::new(),
        }
    }

    fn fresh_temp(&mut self) -> String {
        let t = self.temp;
        self.temp += 1;
        format!("%t{}", t)
    }

    fn fresh_block(&mut self) -> String {
        let b = self.block;
        self.block += 1;
        format!("L{}", b)
    }

    /// Whether the last emitted line already terminates the current block
    /// (ret / br / switch / unreachable), so we don't append instructions or
    /// duplicate terminators after it.
    fn block_terminated(&self) -> bool {
        let trimmed = self.out.trim_end();
        let last = trimmed
            .rsplit('\n')
            .next()
            .unwrap_or("")
            .trim();
        last.starts_with("ret ")
            || last.starts_with("br ")
            || last.starts_with("switch ")
            || last.starts_with("unreachable")
    }

    /// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ define 縺ｾ縺ｧ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・
    fn codegen_function(&mut self, name: &str, fdef: &FunctionDef) -> String {
        // 忣削蜈ｰ縺ｯ Lime main 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・ main_lime 縺ｾ縺ｧ繧｢蜷阪→
        let llvm_name = if name == "main" {
            "main_lime".to_string()
        } else {
            name.to_string()
        };

        let ret_ty = match &fdef.return_type {
            Some(rt) => llvm_type_name(&type_from_str(rt, self.defs)),
            None => "void".to_string(),
        };
        self.fn_ret_ty = ret_ty.clone();
        // Function params with names (%p0, %p1, ...) so alloca/store can reference them
        let mut params = Vec::new();
        let mut param_idx = 0;
        for (_, ptype) in &fdef.params {
            params.push(format!(
                "{} %p{}",
                llvm_type_name(&type_from_str(ptype, self.defs)),
                param_idx
            ));
            param_idx += 1;
        }
        let params_str = params.join(", ");
        let async_note = if fdef.is_async { "; async (lime)" } else { "" };

        let mut head = String::new();
        head.push_str(&format!(
            "\n; Function {}(){}\ndefine {} @{} ({}) local_unnamed_addr #0 {{\n",
            name, async_note, ret_ty, llvm_name, params_str
        ));

        // 蝗ｲ繧險ｱ蜿ｯ: 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・蝗ｲ繧縺ｮ縺ｿ縺ｪ縺・縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
        if !body_supported(&fdef.body) {
            // Phase 1 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・ stub (Phase 0 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・)
            let mut s = String::new();
            if ret_ty == "void" {
                s.push_str("  ret void\n");
            } else {
                s.push_str(&format!("  ret {} {}\n", ret_ty, zero_value_for_type(&type_from_str(
                    fdef.return_type.as_deref().unwrap_or(""), self.defs))));
            }
            return format!("{}{}}}\n", head, s);
        }

        // 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ param 縺ｯ alloca + store
        let mut param_allocs = String::new();
        let mut pidx = 0;
        for (pname, ptype) in &fdef.params {
            let ty = type_from_str(ptype, self.defs);
            let llty = llvm_type_name(&ty);
            let ptr = self.fresh_temp();
            let val = format!("%p{}", pidx);
            pidx += 1;
            param_allocs.push_str(&format!(
                "  {} = alloca {}, align {}\n",
                ptr, llty, align_of(&ty)
            ));
            param_allocs.push_str(&format!(
                "  store {} {}, {}* {}, align {}\n",
                llty, val, llty, ptr, align_of(&ty)
            ));
            self.env.insert(pname.clone(), ty);
            self.named.insert(pname.clone(), ptr);
        }

        // Phase 7: struct method field extraction
        // For struct methods, the first param is 'self' of struct type.
        // Extract each field into a named local variable so the body can
        // reference fields directly (e.g. 'x' instead of 'self.x').
        let mut field_extracts = String::new();
        if let Some((first_pname, first_ptype)) = fdef.params.first() {
            if first_pname == "self" {
                if let Type::Struct(ref sname) = type_from_str(first_ptype, self.defs) {
                    if let Some(sdef) = self.defs.structs.get(sname) {
                        let self_llty = llvm_type_name(&Type::Struct(sname.clone()));
                        // self was alloca'd and stored; load it back
                        let self_ptr = self.named.get("self").cloned().unwrap();
                        let self_loaded = self.fresh_temp();
                        field_extracts.push_str(&format!(
                            "  {} = load {}, {}* {}, align {}\n",
                            self_loaded, self_llty, self_llty, self_ptr,
                            align_of(&Type::Struct(sname.clone()))
                        ));
                        for (i, (fname, ftype)) in sdef.fields.iter().enumerate() {
                            let field_ty = type_from_str(ftype, self.defs);
                            let ll_field_ty = llvm_type_name(&field_ty);
                            let ftmp = self.fresh_temp();
                            field_extracts.push_str(&format!(
                                "  {} = extractvalue {} {}, {}\n",
                                ftmp, self_llty, self_loaded, i
                            ));
                            let fptr = self.fresh_temp();
                            field_extracts.push_str(&format!(
                                "  {} = alloca {}, align {}\n",
                                fptr, ll_field_ty, align_of(&field_ty)
                            ));
                            field_extracts.push_str(&format!(
                                "  store {} {}, {}* {}, align {}\n",
                                ll_field_ty, ftmp, ll_field_ty, fptr, align_of(&field_ty)
                            ));
                            self.env.insert(fname.clone(), field_ty);
                            self.named.insert(fname.clone(), fptr);
                        }
                    }
                }
            }
        }

        // entry block
        let entry = self.fresh_block();
        let mut body_ir = String::new();
        std::mem::swap(&mut self.out, &mut body_ir);
        self.out.push_str(&format!("{}:\n", entry));
        self.current_block = entry;
        self.out.push_str(&param_allocs);
        self.out.push_str(&field_extracts);
        if let Err(e) = self.codegen_stmts(&fdef.body) {
            self.warnings.push(format!("{}: {}", name, e));
        }
        if !self.block_terminated() {
            if ret_ty == "void" {
                self.out.push_str("  ret void\n");
            } else {
                self.out.push_str(&format!("  ret {} {}\n", ret_ty, zero_value_for_type(&type_from_str(
                    fdef.return_type.as_deref().unwrap_or(""), self.defs))));
            }
        }
        self.out.push_str("}\n");
        let mut result = String::new();
        for pf in &self.pending_fns {
            result.push_str(pf);
        }
        result.push_str(&format!("{}{}", head, self.out));
        result
    }

    /// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｿ縺ｪ縺・
    fn codegen_stmts(&mut self, stmts: &[Stmt]) -> Result<(), String> {
        for s in stmts {
            self.codegen_stmt(s)?;
        }
        Ok(())
    }

    fn codegen_stmt(&mut self, s: &Stmt) -> Result<(), String> {
        match s {
            Stmt::Let { name, value, place, type_hint, .. } => {
                let (v, mut ty) = self.codegen_expr(value)?;
                // Phase 4 fix: when a type hint (e.g. `List(int)`) is present and
                // the initializer is an empty list literal, the element type from
                // the hint must win over the inferred `List(Unknown)`. This lets
                // `let List(int): xs = []` codegen `xs.get()` as an Int list.
                if let (Some(th), Expr::Array(items)) = (type_hint.as_ref(), value) {
                    if items.is_empty() {
                        let hint_ty = type_from_str(th, &self.defs);
                        if hint_ty != Type::Unknown {
                            ty = hint_ty;
                        }
                    }
                }
                let llty = llvm_type_name(&ty);
                let align = align_of(&ty);
                let is_heap = matches!(place, Some(MemoryPlace::Heap))
                    || self.memory.get(name) == Some(&MemoryPlace::Heap);
                let ptr = if is_heap {
                    // Phase 1: Heap 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE runtime_alloc 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
                    // (Runtime 縺ｯ Phase 3 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｪ縺ｿ縺ｪ縺・蝣ｴ蜷隗｣譫怜ｸ・縺ｮ縺ｿ縺ｪ縺・)
                    let size = match ty {
                        Type::Float => 8i64,
                        _ => 8i64,
                    };
                    let raw = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = call i8* @runtime_alloc(i64 {}, i64 {})\n",
                        raw, size, align
                    ));
                    let ptr = raw;
                    ptr
                } else {
                    let ptr = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = alloca {}, align {}\n",
                        ptr, llty, align
                    ));
                    ptr
                };
                self.out.push_str(&format!(
                    "  store {} {}, {}* {}, align {}\n",
                    llty, self.bare_value(&v), llty, ptr, align
                ));
                let is_string = matches!(ty, Type::String);
                let is_literal_init = matches!(value, Expr::StringLit(_));
                self.env.insert(name.clone(), ty);
                self.named.insert(name.clone(), ptr);
                // i32-narrowing range tracking: record the known integer range
                // of the initialized value (if statically known to fit i32).
                if let Some(r) = self.int_range(value) {
                    self.var_range.insert(name.clone(), r);
                } else {
                    self.var_range.remove(name);
                }
                // Phase B.1: register a PENDING string-length tracker for
                // variables initialized from a string literal. It only becomes
                // ACTIVE (used for len-tracked push_byte / length queries) once we
                // actually see `name = name.push_byte(...)` in the Assign arm.
                // This keeps ordinary literal-init variables (e.g.
                // `let s = "hello"; s.length()`) lowering to strlen as before.
                if is_string && is_literal_init {
                    let len_ptr = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = alloca i64, align 8\n",
                        len_ptr
                    ));
                    self.out.push_str(&format!(
                        "  store i64 0, i64* {}\n",
                        len_ptr
                    ));
                    self.pending_len_trackers.insert(name.clone(), len_ptr);
                }
                Ok(())
            }
            Stmt::Return { explicit_type: _, value } => {
                self.terminated = true;
                match value {
                    Some(expr) => {
                        let (v, ty) = self.codegen_expr(expr)?;
                        let llty = llvm_type_name(&ty);
                        self.out.push_str(&format!("  ret {} {}\n", llty, self.bare_value(&v)));
                        Ok(())
                    }
                    None => {
                        // A bare `return` still has to satisfy the function's
                        // (possibly inferred) return type: emit the default
                        // value instead of `ret void` when the type is non-void.
                        if self.fn_ret_ty == "void" {
                            self.out.push_str("  ret void\n");
                        } else {
                            let zero = if self.fn_ret_ty.starts_with('%') {
                                "zeroinitializer".to_string()
                            } else {
                                match self.fn_ret_ty.as_str() {
                                    "double" => "0.0".to_string(),
                                    "i1" => "false".to_string(),
                                    "i8*" => "null".to_string(),
                                    _ => "0".to_string(),
                                }
                            };
                            self.out.push_str(&format!("  ret {} {}\n", self.fn_ret_ty, zero));
                        }
                        Ok(())
                    }
                }
            },
            Stmt::Expr(e) => {
                self.codegen_expr(e)?;
                Ok(())
            }
            Stmt::Assign { name, value } => {
                // Phase B.1: length-tracked string variable.
                // Promote a PENDING tracker to ACTIVE on `name = name.push_byte(...)`.
                let is_self_push = matches!(
                    value,
                    Expr::MethodCall { object: box_expr, method, .. }
                    if matches!(box_expr.as_ref(), Expr::Ident(n) if n == name)
                        && method == "push_byte"
                );
                let is_concat_lit = matches!(
                    value,
                    Expr::BinOp { op, left, right, .. }
                    if op == "+"
                        && matches!(left.as_ref(), Expr::Ident(n) if n == name)
                        && matches!(right.as_ref(), Expr::StringLit(_))
                );
                if is_concat_lit {
                    if let Some(len_ptr) = self.pending_len_trackers.get(name).cloned()
                        .or_else(|| self.string_len_trackers.get(name).cloned()) {
                        // Promote pending -> active if needed.
                        self.pending_len_trackers.remove(name);
                        self.string_len_trackers.insert(name.clone(), len_ptr.clone());
                        // Emit the concat normally: text = runtime_str_concat(text, literal)
                        let (v, _ty) = self.codegen_expr(value)?;
                        let ptr = self.named.get(name).cloned()
                            .ok_or_else(|| format!("undefined variable '{}'", name))?;
                        self.out.push_str(&format!(
                            "  store i8* {}, i8** {}\n",
                            self.bare_value(&v), ptr
                        ));
                        // text__len = text__len + literal.len()
                        if let Expr::BinOp { right, .. } = value {
                            if let Expr::StringLit(lit) = right.as_ref() {
                                let cur_len = self.fresh_temp();
                                self.out.push_str(&format!(
                                    "  {} = load i64, i64* {}\n",
                                    cur_len, len_ptr
                                ));
                                let new_len = self.fresh_temp();
                                self.out.push_str(&format!(
                                    "  {} = add i64 {}, {}\n",
                                    new_len, cur_len, lit.len()
                                ));
                                self.out.push_str(&format!(
                                    "  store i64 {}, i64* {}\n",
                                    self.bare_value(&new_len), len_ptr
                                ));
                            }
                        }
                        return Ok(());
                    }
                }
                if is_self_push {
                    if let Some(len_ptr) = self.pending_len_trackers.get(name).cloned() {
                        // Promote pending -> active.
                        self.pending_len_trackers.remove(name);
                        self.string_len_trackers.insert(name.clone(), len_ptr.clone());
                        // Emit: cur = runtime_str_push_byte_len(cur, *cur__len, ch)
                        // then cur__len = cur__len + 1.
                        if let Expr::MethodCall { object, args, .. } = value {
                            let (obj_v, _) = self.codegen_expr(object)?;
                            let (arg_v, _) = self.codegen_expr(&args[0])?;
                            let cur_ptr = self
                                .named
                                .get(name)
                                .cloned()
                                .ok_or_else(|| format!("undefined variable '{}'", name))?;
                            let cur_val = self.fresh_temp();
                            self.out.push_str(&format!(
                                "  {} = load i8*, i8** {}\n",
                                cur_val, cur_ptr
                            ));
                            let loaded_len = self.fresh_temp();
                            self.out.push_str(&format!(
                                "  {} = load i64, i64* {}\n",
                                loaded_len, len_ptr
                            ));
                            let new_cur = self.fresh_temp();
                            self.out.push_str(&format!(
                                "  {} = call i8* @runtime_str_push_byte_len(i8* {}, i64 {}, {})\n",
                                new_cur,
                                self.bare_value(&obj_v),
                                self.bare_value(&loaded_len),
                                self.fmt_call_arg(&arg_v, &Type::Int)
                            ));
                            self.out.push_str(&format!(
                                "  store i8* {}, i8** {}\n",
                                self.bare_value(&new_cur), cur_ptr
                            ));
                            let inc = self.fresh_temp();
                            self.out.push_str(&format!(
                                "  {} = add i64 {}, 1\n",
                                inc, self.bare_value(&loaded_len)
                            ));
                            self.out.push_str(&format!(
                                "  store i64 {}, i64* {}\n",
                                self.bare_value(&inc), len_ptr
                            ));
                            return Ok(());
                        }
                    }
                }
                // Phase B.1: if `name` has an ACTIVE tracker, handle reset / invalidate.
                if let Some(len_ptr) = self.string_len_trackers.get(name).cloned() {
                    // Detect `name = <string literal>` (reset).
                    if matches!(value, Expr::StringLit(_)) {
                        let (v, _ty) = self.codegen_expr(value)?;
                        let ptr = self
                            .named
                            .get(name)
                            .cloned()
                            .ok_or_else(|| format!("undefined variable '{}'", name))?;
                        let llty = "i8*";
                        self.out.push_str(&format!(
                            "  store {} {}, {}* {}, align 8\n",
                            llty, self.bare_value(&v), llty, ptr
                        ));
                        // reset length to 0
                        self.out.push_str(&format!("  store i64 0, i64* {}\n", len_ptr));
                        return Ok(());
                    }
                    // Any other assignment (concat, etc.) invalidates tracking.
                    self.string_len_trackers.remove(name);
                } else if self.pending_len_trackers.contains_key(name) {
                    // A pending tracker that was NOT promoted is invalidated on
                    // any assignment EXCEPT a literal reset (`name = ""`), which
                    // keeps it pending (the next push_byte still promotes it).
                    if !matches!(value, Expr::StringLit(_)) {
                        self.pending_len_trackers.remove(name);
                    }
                }
                let (v, _ty) = self.codegen_expr(value)?;
                let ptr = self
                    .named
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("undefined variable '{}'", name))?;
                let ty = self
                    .env
                    .get(name)
                    .cloned()
                    .unwrap_or(Type::Int);
                let llty = llvm_type_name(&ty);
                self.out.push_str(&format!(
                    "  store {} {}, {}* {}, align {}\n",
                    llty, self.bare_value(&v), llty, ptr, align_of(&ty)
                ));
                // i32-narrowing range tracking (see Stmt::Let for rationale).
                if let Some(r) = self.int_range(value) {
                    self.var_range.insert(name.clone(), r);
                } else {
                    self.var_range.remove(name);
                }
                Ok(())
            }
            Stmt::If { cond, then_branch, else_branch } => {
                let (c, _ct) = self.codegen_expr(cond)?;
                let then_b = self.fresh_block();
                let else_b = self.fresh_block();
                let merge_b = self.fresh_block();
                match else_branch {
                    Some(_) => {
                        self.out
                            .push_str(&format!("  br i1 {}, label %{}, label %{}\n", self.bare_value(&c), then_b, else_b));
                    }
                    None => {
                        self.out
                            .push_str(&format!("  br i1 {}, label %{}, label %{}\n", self.bare_value(&c), then_b, merge_b));
                    }
                }
                self.out.push_str(&format!("{}:\n", then_b));
                self.current_block = then_b;
                self.codegen_stmts(then_branch)?;
                self.out.push_str(&format!("  br label %{}\n", merge_b));
                if let Some(eb) = else_branch {
                    self.out.push_str(&format!("{}:\n", else_b));
                    self.current_block = else_b;
                    self.codegen_stmts(eb)?;
                    self.out.push_str(&format!("  br label %{}\n", merge_b));
                }
                self.out.push_str(&format!("{}:\n", merge_b));
                self.current_block = merge_b;
                Ok(())
            }
            Stmt::While { cond, body } => {
                let cond_b = self.fresh_block();
                let body_b = self.fresh_block();
                let merge_b = self.fresh_block();
                // i32-narrowing: detect a simple integer induction variable
                // `counter < bound` (or <=) with a constant i32-fit bound, so
                // the loop body's arithmetic can be narrowed to i32 and
                // vectorized like Clang -O3.
                let saved_counter = self.loop_counter.clone();
                let saved_bound = self.loop_bound;
                let saved_init = self.loop_counter_init;
                let saved_pure = self.loop_pure_arith;
                let mut counter_cmp_i32: Option<(String, String)> = None; // (i32_instr, bound_literal)
                if let Expr::BinOp { op, left, right, .. } = cond {
                    let (counter, bound) = match op.as_str() {
                        "<" => (left, right),
                        "<=" => (left, right),
                        ">" => (right, left),
                        ">=" => (right, left),
                        _ => (left, right),
                    };
                    if let (Expr::Ident(cn), Expr::IntLit(b)) = (counter.as_ref(), bound.as_ref()) {
                        if *b <= i32::MAX as i64 && *b >= i32::MIN as i64 {
                            self.loop_counter = Some(cn.clone());
                            self.loop_bound = Some(*b as i128);
                            self.loop_counter_init = self.var_range.get(cn).map(|r| r.0).unwrap_or(0);
                            let cmp_instr = match op.as_str() {
                                "<" => "icmp slt",
                                "<=" => "icmp sle",
                                ">" => "icmp sgt",
                                ">=" => "icmp sge",
                                _ => "icmp slt",
                            };
                            // Only emit the i32 induction-var compare and enable
                            // i32 narrowing when the loop body is pure integer
                            // arithmetic (no method/function calls). Collection
                            // loops (e.g. set_ops) keep i64 so that runtime
                            // helpers like `runtime_list_add` stay inlineable.
                            let pure = !body.iter().any(Self::stmt_has_call);
                            self.loop_pure_arith = pure;
                            if pure {
                                counter_cmp_i32 = Some((cmp_instr.to_string(), b.to_string()));
                            }
                        }
                    }
                }
                self.out.push_str(&format!("  br label %{}\n", cond_b));
                self.out.push_str(&format!("{}:\n", cond_b));
                self.current_block = cond_b.clone();
                let c: String = if let Some((cmp_instr, bound_lit)) = &counter_cmp_i32 {
                    // Emit the loop condition in i32 so the induction variable
                    // matches the loop body's i32 arithmetic (enables LLVM
                    // auto-vectorization, mirroring Clang -O3).
                    let (cv, _ct) = self.codegen_expr(
                        if let Expr::BinOp { left, .. } = cond { left } else { cond },
                    )?;
                    let ci = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = trunc i64 {} to i32\n",
                        ci,
                        self.bare_value(&cv)
                    ));
                    let cc = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = {} i32 {}, {}\n",
                        cc, cmp_instr, ci, bound_lit
                    ));
                    cc
                } else {
                    let (c, _ct) = self.codegen_expr(cond)?;
                    c
                };
                self.out
                    .push_str(&format!(
                        "  br i1 {}, label %{}, label %{}\n",
                        self.bare_value(&c),
                        body_b,
                        merge_b
                    ));
                self.out.push_str(&format!("{}:\n", body_b));
                self.current_block = body_b;
                self.codegen_stmts(body)?;
                self.out.push_str(&format!("  br label %{}\n", cond_b));
                self.out.push_str(&format!("{}:\n", merge_b));
                self.current_block = merge_b;
                // Restore loop state (so nested/sequential loops don't leak).
                self.loop_counter = saved_counter;
                self.loop_bound = saved_bound;
                self.loop_counter_init = saved_init;
                self.loop_pure_arith = saved_pure;
                Ok(())
            }
            Stmt::Match { expr, arms } => self.codegen_match(expr, arms),
            _ => Err("Phase 1: unsupported statement in codegen".to_string()),
        }
    }

    /// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｪ縺ｿ縺ｪ縺・(SSA value, Type) 縺ｦ股�ｸｦ縺ｧ繧｢蜷阪→
    fn codegen_expr(&mut self, e: &Expr) -> Result<(String, Type), String> {
        match e {
            Expr::IntLit(i) => Ok((format!("i64 {}", i), Type::Int)),
            Expr::FloatLit(f) => {
                // LLVM 縺ｯ double 縺ｯ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・ format 縺ｯ蝗ｲ繧縺ｮ縺ｿ縺ｪ縺・
                let s = if f.fract() == 0.0 {
                    format!("{}.0", f)
                } else {
                    format!("{}", f)
                };
                Ok((format!("double {}", s), Type::Float))
            }
            Expr::BoolLit(b) => Ok((format!("i1 {}", if *b { "true" } else { "false" }), Type::Bool)),
            Expr::Ident(n) => {
                // Check if it's a local variable first
                if let Some(ptr) = self.named.get(n).cloned() {
                    let ty = self
                        .env
                        .get(n)
                        .cloned()
                        .ok_or_else(|| format!("undefined variable '{}'", n))?;
                    let llty = llvm_type_name(&ty);
                    let tmp = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = load {}, {}* {}, align {}\n",
                        tmp, llty, llty, ptr, align_of(&ty)
                    ));
                    // If the variable holds a function value (closure), it's already loaded
                    // as %LimeClosure* — return it directly with the Fn type.
                    if matches!(ty, Type::Fn(_, _)) {
                        return Ok((tmp, ty));
                    }
                    return Ok((tmp, ty));
                }
                // Phase B-2.2: named function reference -> generate wrapper + wrap in closure
                if let Some(fdef) = self.defs.functions.get(n) {
                    let param_types: Vec<Type> = fdef.params.iter()
                        .map(|(_, t)| type_from_str(t, self.defs))
                        .collect();
                    let ret = match &fdef.return_type {
                        Some(rt) => type_from_str(rt, self.defs),
                        None => Type::Unit,
                    };
                    let llvm_name = if n == "main" { "main_lime" } else { n };

                    // Generate a wrapper: define i64 @wrap_<name>(i8* %env, i8* %packed) { ... }
                    let wrapper_name = format!("wrap_{}", n);
                    let ll_ret = llvm_type_name(&ret);
                    let mut wrapper_ir = String::new();
                    wrapper_ir.push_str(&format!(
                        "\n; Closure wrapper for {}\ndefine {} @{}(i8* %env, i8* %packed) {{\n",
                        n, ll_ret, wrapper_name
                    ));
                    wrapper_ir.push_str("L0:\n");

                    // Unpack each argument from the packed struct
                    let mut unpacked_args = Vec::new();
                    for (i, (_, ptype_str)) in fdef.params.iter().enumerate() {
                        let pty = type_from_str(ptype_str, self.defs);
                        let llpty = llvm_type_name(&pty);
                        let offset = (i as i64) * 8;
                        let raw_ptr = format!("%uptr_{}", i);
                        wrapper_ir.push_str(&format!(
                            "  {} = getelementptr i8, i8* %packed, i64 {}\n",
                            raw_ptr, offset
                        ));
                        let ptr_i64 = format!("%uptr_i64_{}", i);
                        wrapper_ir.push_str(&format!(
                            "  {} = bitcast i8* {} to i64*\n",
                            ptr_i64, raw_ptr
                        ));
                        let loaded_i64 = format!("%uloaded_{}", i);
                        wrapper_ir.push_str(&format!(
                            "  {} = load i64, i64* {}, align 8\n",
                            loaded_i64, ptr_i64
                        ));
                        // Convert i64 to the actual parameter type
                        let converted = format!("%uconv_{}", i);
                        match pty {
                            Type::Int | Type::Long => {
                                wrapper_ir.push_str(&format!(
                                    "  {} = add i64 {}, 0\n",
                                    converted, loaded_i64
                                ));
                            }
                            Type::Float => {
                                wrapper_ir.push_str(&format!(
                                    "  {} = bitcast i64 {} to double\n",
                                    converted, loaded_i64
                                ));
                            }
                            Type::Bool => {
                                wrapper_ir.push_str(&format!(
                                    "  {} = trunc i64 {} to i1\n",
                                    converted, loaded_i64
                                ));
                            }
                            _ => {
                                wrapper_ir.push_str(&format!(
                                    "  {} = inttoptr i64 {} to {}\n",
                                    converted, loaded_i64, llpty
                                ));
                            }
                        }
                        unpacked_args.push(format!("{} {}", llpty, converted));
                    }

                    // Call the real function
                    let args_str = unpacked_args.join(", ");
                    if ll_ret == "void" {
                        wrapper_ir.push_str(&format!(
                            "  call void @{}({})\n",
                            llvm_name, args_str
                        ));
                        wrapper_ir.push_str("  ret void\n");
                    } else {
                        wrapper_ir.push_str(&format!(
                            "  %cret = call {} @{}({})\n",
                            ll_ret, llvm_name, args_str
                        ));
                        wrapper_ir.push_str(&format!("  ret {} %cret\n", ll_ret));
                    }
                    wrapper_ir.push_str("}\n");
                    self.pending_fns.push(wrapper_ir);

                    // Create closure wrapping the wrapper function
                    let wrapper_ty = format!("{} (i8*, i8*)", ll_ret);
                    let fn_ptr_cast = self.fresh_temp();
                    let closure = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = bitcast {}* @{} to i8*\n",
                        fn_ptr_cast, wrapper_ty, wrapper_name
                    ));
                    self.out.push_str(&format!(
                        "  {} = call %LimeClosure* @runtime_make_fn_ref(i8* {})\n",
                        closure, fn_ptr_cast
                    ));
                    return Ok((closure, Type::Fn(param_types, Box::new(ret))));
                }
                Err(format!("undefined variable '{}'", n))
            }
            Expr::BinOp { left, op, right, resolved_operator } => self.codegen_binop(left, op, right, resolved_operator),
            Expr::UnOp { op, operand } => {
                if op == "not" {
                    let (v, _t) = self.codegen_expr(operand)?;
                    let tmp = self.fresh_temp();
                    self.out.push_str(&format!("  {} = xor i1 {}, true\n", tmp, self.bare_value(&v)));
                    Ok((tmp, Type::Bool))
                } else if op == "-" {
                    let (v, t) = self.codegen_expr(operand)?;
                    let tmp = self.fresh_temp();
                    match t {
                        Type::Float => self.out.push_str(&format!(
                            "  {} = fsub double 0.0, {}\n",
                            tmp,
                            self.bare_value(&v)
                        )),
                        Type::Int | Type::Long => self.out.push_str(&format!(
                            "  {} = sub i64 0, {}\n",
                            tmp,
                            self.bare_value(&v)
                        )),
                        _ => return Err(format!("Phase 1: unary '-' not supported for {:?}", t)),
                    }
                    Ok((tmp, t))
                } else {
                    Err(format!("Phase 1: unsupported unary operator '{}'", op))
                }
            }
            Expr::FieldAccess { object, field } => self.codegen_field_access(object, field),
            Expr::Call { func, args } => self.codegen_call(func, args),
            Expr::StringLit(s) => self.codegen_string_lit(s),
            Expr::MethodCall { object, method, args } => self.codegen_method_call(object, method, args),
            Expr::Array(items) => self.codegen_array_lit(items),
            Expr::Await(inner) => self.codegen_expr(inner),
            Expr::FnDef { params, body } => self.codegen_anon_fn(params, body),
            _ => Err("Phase 5: unsupported expression in codegen".to_string()),
        }
    }

    /// Collect free variables used in a statement list.
    /// Free variables are names that appear in `Expr::Ident` but are not
    /// defined as local parameters or by `Stmt::Let` within the body.
    /// `param_names` are the closure's own parameters (excluded from results).
    fn collect_free_vars(body: &[Stmt], param_names: &[String]) -> Vec<String> {
        let mut used = Vec::new();
        let mut defined = Vec::new();
        for p in param_names {
            defined.push(p.clone());
        }
        Self::collect_vars_stmts(body, &mut used, &mut defined);
        used.sort();
        used.dedup();
        used
    }

    fn collect_vars_stmts(stmts: &[Stmt], used: &mut Vec<String>, defined: &mut Vec<String>) {
        for s in stmts {
            match s {
                Stmt::Let { name, value, .. } => {
                    Self::collect_vars_expr(value, used, defined);
                    defined.push(name.clone());
                }
                Stmt::Assign { name, value, .. } => {
                    Self::collect_vars_expr(value, used, defined);
                    if !defined.contains(name) && !used.contains(name) {
                        used.push(name.clone());
                    }
                }
                Stmt::Return { value: Some(e), .. } => {
                    Self::collect_vars_expr(e, used, defined);
                }
                Stmt::If { cond, then_branch, else_branch } => {
                    Self::collect_vars_expr(cond, used, defined);
                    Self::collect_vars_stmts(then_branch, used, defined);
                    if let Some(eb) = else_branch {
                        Self::collect_vars_stmts(eb, used, defined);
                    }
                }
                Stmt::While { cond, body } => {
                    Self::collect_vars_expr(cond, used, defined);
                    Self::collect_vars_stmts(body, used, defined);
                }
                Stmt::Expr(e) => {
                    Self::collect_vars_expr(e, used, defined);
                }
                Stmt::Defer { body } => {
                    Self::collect_vars_stmts(body, used, defined);
                }
                _ => {}
            }
        }
    }

    fn collect_vars_expr(e: &Expr, used: &mut Vec<String>, defined: &mut Vec<String>) {
        match e {
            Expr::Ident(n) => {
                if !defined.contains(n) && !used.contains(n) {
                    used.push(n.clone());
                }
            }
            Expr::Call { args, .. } => {
                for a in args {
                    Self::collect_vars_expr(a, used, defined);
                }
            }
            Expr::BinOp { left, right, .. } => {
                Self::collect_vars_expr(left, used, defined);
                Self::collect_vars_expr(right, used, defined);
            }
            Expr::UnOp { operand, .. } => {
                Self::collect_vars_expr(operand, used, defined);
            }
            Expr::Array(items) => {
                for item in items {
                    Self::collect_vars_expr(item, used, defined);
                }
            }
            Expr::FieldAccess { object, .. } => {
                Self::collect_vars_expr(object, used, defined);
            }
            Expr::Index { target, index } => {
                Self::collect_vars_expr(target, used, defined);
                Self::collect_vars_expr(index, used, defined);
            }
            Expr::FnDef { params, body } => {
                // Nested fn: its params shadow outer names
                let mut inner_defined = defined.clone();
                for (n, _) in params {
                    inner_defined.push(n.clone());
                }
                Self::collect_vars_stmts(body, used, &mut inner_defined);
            }
            Expr::Slice { target, start, end } => {
                Self::collect_vars_expr(target, used, defined);
                if let Some(s) = start { Self::collect_vars_expr(s, used, defined); }
                if let Some(e) = end { Self::collect_vars_expr(e, used, defined); }
            }
            Expr::Tuple(elems) => {
                for elem in elems {
                    Self::collect_vars_expr(elem, used, defined);
                }
            }
            Expr::Range { start, end, .. } => {
                Self::collect_vars_expr(start, used, defined);
                Self::collect_vars_expr(end, used, defined);
            }
            Expr::Await(inner) => {
                Self::collect_vars_expr(inner, used, defined);
            }
            Expr::MethodCall { object, args, .. } => {
                Self::collect_vars_expr(object, used, defined);
                for a in args {
                    Self::collect_vars_expr(a, used, defined);
                }
            }
            _ => {} // literals, no sub-expressions
        }
    }

    /// Codegen for anonymous function definitions (fn (int: a, int: b) { ... }).
    /// Generates a standalone LLVM function: `define i64 @anon_N(i8* %env, i8* %packed_args)`
    /// that unpacks arguments from the packed struct, executes the body, and returns.
    /// If the closure captures free variables, they are packed into a heap i64 array
    /// and passed as the env_ptr.
    fn codegen_anon_fn(
        &mut self,
        params: &[(String, String)],
        body: &[Stmt],
    ) -> Result<(String, Type), String> {
        let name = format!("anon_{}", self.anon_count);
        self.anon_count += 1;

        // Parse parameter types
        let param_types: Vec<Type> = params.iter()
            .map(|(_, t)| type_from_str(t, self.defs))
            .collect();

        // Return type: hardcoded to i64 (MVP limitation).
        let ret_type_str = "i64".to_string();

        // Detect free variables in the body that exist in the parent scope
        let param_names: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
        let all_free = Self::collect_free_vars(body, &param_names);
        let captures: Vec<(String, Type)> = all_free.iter()
            .filter_map(|n| {
                self.env.get(n).cloned().map(|t| (n.clone(), t))
            })
            .collect();

        // Build function head: define i64 @anon_N(i8* %env, i8* %packed_args) {
        let mut ir = String::new();
        ir.push_str(&format!(
            "\n; Anonymous function ({} captures)\ndefine {} @{}(i8* %env, i8* %packed_args) {{\n",
            captures.len(), ret_type_str, name
        ));

        // Entry block
        ir.push_str("L0:\n");

        // Unpack captured variables from the env array
        let mut child = Cg::new(self.defs, self.memory, self.string_literals, self.mono_name_map, self.mono_fdefs);
        let mut named: HashMap<String, String> = HashMap::new();
        let mut env_types: HashMap<String, Type> = HashMap::new();

        for (i, (cname, ctype)) in captures.iter().enumerate() {
            // GEP into env to get i64* for this capture
            let raw_ptr = format!("%env_unpack_{}", i);
            let offset = (i as i64) * 8;
            ir.push_str(&format!(
                "  {} = getelementptr i8, i8* %env, i64 {}\n",
                raw_ptr, offset
            ));
            let ptr_i64 = format!("%env_ptr_{}", i);
            ir.push_str(&format!(
                "  {} = bitcast i8* {} to i64*\n",
                ptr_i64, raw_ptr
            ));
            let loaded_i64 = format!("%env_loaded_{}", i);
            ir.push_str(&format!(
                "  {} = load i64, i64* {}, align 8\n",
                loaded_i64, ptr_i64
            ));
            // Convert from i64 to actual type
            let converted = format!("%env_val_{}", i);
            match ctype {
                Type::Float => {
                    ir.push_str(&format!(
                        "  {} = bitcast i64 {} to double\n",
                        converted, loaded_i64
                    ));
                }
                Type::Bool => {
                    ir.push_str(&format!(
                        "  {} = trunc i64 {} to i1\n",
                        converted, loaded_i64
                    ));
                }
                _ => {
                    ir.push_str(&format!(
                        "  {} = add i64 {}, 0\n",
                        converted, loaded_i64
                    ));
                }
            }
            // alloca + store for the body to reference
            let alloca_ptr = format!("%al_cap_{}", i);
            let llty_real = llvm_type_name(ctype);
            ir.push_str(&format!(
                "  {} = alloca {}, align 8\n",
                alloca_ptr, llty_real
            ));
            ir.push_str(&format!(
                "  store {} {}, {}* {}, align 8\n",
                llty_real, converted, llty_real, alloca_ptr
            ));
            named.insert(cname.clone(), alloca_ptr);
            env_types.insert(cname.clone(), ctype.clone());
        }

        // Unpack arguments from the packed struct
        for (i, (pname, ptype_str)) in params.iter().enumerate() {
            let pty = type_from_str(ptype_str, self.defs);
            let llty = llvm_type_name(&pty);

            // GEP into packed_args to get i64* for this arg
            let raw_ptr = format!("%unpack_{}", i);
            let offset = (i as i64) * 8;
            ir.push_str(&format!(
                "  {} = getelementptr i8, i8* %packed_args, i64 {}\n",
                raw_ptr, offset
            ));
            let ptr_i64 = format!("%ptr_{}", i);
            ir.push_str(&format!(
                "  {} = bitcast i8* {} to {}*\n",
                ptr_i64, raw_ptr, llty
            ));
            let loaded = format!("%arg_{}", i);
            ir.push_str(&format!(
                "  {} = load {}, {}* {}, align 8\n",
                loaded, llty, llty, ptr_i64
            ));

            // alloca + store for the body to reference
            let alloca_ptr = format!("%al_{}", i);
            ir.push_str(&format!(
                "  {} = alloca {}, align 8\n",
                alloca_ptr, llty
            ));
            ir.push_str(&format!(
                "  store {} {}, {}* {}, align 8\n",
                llty, loaded, llty, alloca_ptr
            ));
            named.insert(pname.clone(), alloca_ptr);
            env_types.insert(pname.clone(), pty);
        }

        // Codegen the body using a child Cg, injecting unpacked args + captures
        child.out.clear();
        child.named = named;
        child.env = env_types;
        child.current_block = "L0".to_string();
        child.block = 1;
        child.temp = 0;

        // Codegen statements
        for s in body {
            if let Err(e) = child.codegen_stmt(s) {
                child.warnings.push(format!("{}: {}", name, e));
            }
        }
        // Add default return if not terminated
        if !child.block_terminated() {
            child.out.push_str(&format!("  ret {} 0\n", ret_type_str));
        }
        ir.push_str(&child.out);
        ir.push_str("}\n");

        self.pending_fns.push(ir);

        // Create the closure, packing captures into env if needed
        let fn_ptr_cast = self.fresh_temp();
        let closure = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = bitcast i64 (i8*, i8*)* @{} to i8*\n",
            fn_ptr_cast, name
        ));

        if captures.is_empty() {
            // No captures: use runtime_make_fn_ref (env_ptr = NULL)
            self.out.push_str(&format!(
                "  {} = call %LimeClosure* @runtime_make_fn_ref(i8* {})\n",
                closure, fn_ptr_cast
            ));
        } else {
            // Pack captures into a heap i64 array
            let env_size = (captures.len() * 8) as i64;
            let env_raw = self.fresh_temp();
            self.out.push_str(&format!(
                "  {} = call i8* @runtime_alloc(i64 {}, i64 8)\n",
                env_raw, env_size
            ));
            // Collect alloca pointers first to avoid borrow conflict
            let capture_ptrs: Vec<(String, Type, String)> = captures.iter().map(|(cname, ctype)| {
                let ptr = self.named.get(cname)
                    .cloned()
                    .unwrap_or_else(|| format!("MISSING_{}", cname));
                (cname.clone(), ctype.clone(), ptr)
            }).collect();
            for (i, (_cname, ctype, ptr)) in capture_ptrs.iter().enumerate() {
                let loaded = self.fresh_temp();
                let llty = llvm_type_name(ctype);
                self.out.push_str(&format!(
                    "  {} = load {}, {}* {}, align {}\n",
                    loaded, llty, llty, ptr, align_of(ctype)
                ));
                // Pack as i64
                let packed_val = self.fresh_temp();
                match ctype {
                    Type::Float => {
                        self.out.push_str(&format!(
                            "  {} = bitcast double {} to i64\n",
                            packed_val, loaded
                        ));
                    }
                    Type::Bool => {
                        self.out.push_str(&format!(
                            "  {} = zext i1 {} to i64\n",
                            packed_val, loaded
                        ));
                    }
                    _ => {
                        self.out.push_str(&format!(
                            "  {} = add i64 {}, 0\n",
                            packed_val, loaded
                        ));
                    }
                }
                // Store into env array slot
                let slot_ptr_raw = self.fresh_temp();
                let offset = (i as i64) * 8;
                self.out.push_str(&format!(
                    "  {} = getelementptr i8, i8* {}, i64 {}\n",
                    slot_ptr_raw, env_raw, offset
                ));
                let slot_ptr = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = bitcast i8* {} to i64*\n",
                    slot_ptr, slot_ptr_raw
                ));
                self.out.push_str(&format!(
                    "  store i64 {}, i64* {}, align 8\n",
                    packed_val, slot_ptr
                ));
            }
            // Create closure with env_ptr
            self.out.push_str(&format!(
                "  {} = call %LimeClosure* @runtime_make_closure(i8* {}, i8* {})\n",
                closure, fn_ptr_cast, env_raw
            ));
        }

        Ok((closure, Type::Fn(
            param_types,
            Box::new(Type::Int), // MVP: assume i64 return
        )))
    }

    /// Statically estimate the integer value range of an expression. Returns
    /// `Some((min, max))` only when the range is known and fits within i32
    /// bounds (so the value is safe to materialize in i32). Returns `None` when
    /// the range is unknown or could exceed i32. This is a *conservative*
    /// over-approximation used purely to decide i32-narrowing safety; it never
    /// affects observable semantics.
    fn int_range(&self, e: &Expr) -> Option<(i128, i128)> {
        const I32MIN: i128 = i32::MIN as i128;
        const I32MAX: i128 = i32::MAX as i128;
        fn sat_add(a: i128, b: i128) -> i128 {
            a.checked_add(b).unwrap_or(if (a > 0) == (b > 0) { i32::MAX as i128 + 1 } else { i32::MIN as i128 - 1 })
        }
        fn sat_mul(a: i128, b: i128) -> i128 {
            a.checked_mul(b).unwrap_or(if (a > 0) == (b > 0) { i32::MAX as i128 + 1 } else { i32::MIN as i128 - 1 })
        }
        let r = match e {
            Expr::IntLit(i) => (*i as i128, *i as i128),
            Expr::Ident(n) => {
                if let Some(c) = &self.loop_counter {
                    if c == n {
                        let lo = self.loop_counter_init;
                        let hi = self.loop_bound.map(|b| b - 1).unwrap_or(i32::MAX as i128);
                        return if lo >= I32MIN && hi <= I32MAX { Some((lo, hi)) } else { None };
                    }
                }
                match self.var_range.get(n) {
                    Some(r) => *r,
                    None => return None,
                }
            }
            Expr::BinOp { op, left, right, .. } => {
                match op.as_str() {
                    "+" | "-" | "*" => {
                        let (ll, lh) = self.int_range(left)?;
                        let (rl, rh) = self.int_range(right)?;
                        let (lo, hi) = match op.as_str() {
                            "+" => (sat_add(ll, rl), sat_add(lh, rh)),
                            "-" => (sat_add(ll, -rh), sat_add(lh, -rl)),
                            "*" => (sat_mul(ll, rl), sat_mul(lh, rh)),
                            _ => unreachable!(),
                        };
                        (lo, hi)
                    }
                    "%" => {
                        if let Expr::IntLit(c) = right.as_ref() {
                            if *c >= 1 {
                                let m = *c - 1;
                                (-(m as i128), m as i128)
                            } else {
                                return None;
                            }
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
            _ => return None,
        };
        if r.0 >= I32MIN && r.1 <= I32MAX { Some(r) } else { None }
    }

    /// Whether an expression contains a method/function call (which lowers to a
    /// runtime call we must not disturb with i32 narrowing / i32-induction-var
    /// changes — doing so can break `always_inline` inlining of helpers like
    /// `runtime_list_add`).
    fn expr_has_call(e: &Expr) -> bool {
        match e {
            Expr::MethodCall { .. } => true,
            Expr::Call { .. } => true,
            Expr::BinOp { left, right, .. } => {
                Self::expr_has_call(left) || Self::expr_has_call(right)
            }
            Expr::UnOp { operand, .. } => Self::expr_has_call(operand),
            Expr::FieldAccess { object, .. } => Self::expr_has_call(object),
            Expr::Index { target, index } => {
                Self::expr_has_call(target) || Self::expr_has_call(index)
            }
            Expr::Slice { target, start, end } => {
                Self::expr_has_call(target)
                    || start.as_ref().map_or(false, |s| Self::expr_has_call(s))
                    || end.as_ref().map_or(false, |s| Self::expr_has_call(s))
            }
            Expr::Range { start, end } => {
                Self::expr_has_call(start) || Self::expr_has_call(end)
            }
            Expr::Tuple(es) => es.iter().any(Self::expr_has_call),
            Expr::Array(es) => es.iter().any(Self::expr_has_call),
            _ => false,
        }
    }

    /// Whether a statement (or any nested statement) contains a call.
    fn stmt_has_call(s: &Stmt) -> bool {
        match s {
            Stmt::Let { value, .. } => Self::expr_has_call(value),
            Stmt::Assign { value, .. } => Self::expr_has_call(value),
            Stmt::If { cond, then_branch, else_branch } => {
                Self::expr_has_call(cond)
                    || then_branch.iter().any(Self::stmt_has_call)
                    || else_branch
                        .as_ref()
                        .map_or(false, |b| b.iter().any(Self::stmt_has_call))
            }
            Stmt::For { var: _, iterable, body } => {
                Self::expr_has_call(iterable) || body.iter().any(Self::stmt_has_call)
            }
            Stmt::While { cond, body } => {
                Self::expr_has_call(cond) || body.iter().any(Self::stmt_has_call)
            }
            Stmt::Match { expr, arms } => {
                Self::expr_has_call(expr)
                    || arms.iter().any(|(_p, bs)| bs.iter().any(Self::stmt_has_call))
            }
            Stmt::Return { value, .. } => value.as_ref().map_or(false, Self::expr_has_call),
            Stmt::Expr(e) => Self::expr_has_call(e),
            _ => false,
        }
    }

    /// Return the i32 SSA value for an operand of a narrowed operation. If the
    /// operand already has a known i32 form (it was the result of a previous
    /// narrowed op, recorded in `i32_form`), reuse it directly; otherwise emit
    /// `trunc i64 -> i32`. `v` is the bare (already `%`-prefixed or literal)
    /// value string from `codegen_expr`.
    fn narrow_operand_i32(&mut self, v: &str) -> String {
        let bare = self.bare_value(v);
        if let Some(i32v) = self.i32_form.get(bare) {
            return i32v.clone();
        }
        // Emit `trunc i64 -> i32` (works for both SSA values and literals;
        // LLVM folds the literal case).
        let t = self.fresh_temp();
        self.out.push_str(&format!("  {} = trunc i64 {} to i32\n", t, bare));
        t
    }

    fn codegen_binop(
        &mut self,
        left: &Expr,
        op: &str,
        right: &Expr,
        resolved_operator: &Option<ResolvedOperator>,
    ) -> Result<(String, Type), String> {
        let (lv, lt) = self.codegen_expr(left)?;
        let (rv, rt) = self.codegen_expr(right)?;

        // i32-narrowing optimization: for integer `+ - * %` whose operands and
        // result are statically proven to fit in i32, emit the operation in
        // i32 (trunc operands -> i32 op -> sext back to i64). This lets LLVM
        // vectorize tight loops the way Clang -O3 does, without changing
        // observable semantics (the value provably fits i32 at runtime).
        if lt == Type::Int && rt == Type::Int && matches!(op, "+" | "-" | "*" | "%") && self.loop_pure_arith {
            if let Some(_wr) = self.int_range(&Expr::BinOp {
                op: op.to_string(),
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
                resolved_operator: resolved_operator.clone(),
            }) {
                let lr = self.int_range(left);
                let rr = self.int_range(right);
                if let (Some(lr), Some(rr)) = (lr, rr) {
                    const I32MIN: i128 = i32::MIN as i128;
                    const I32MAX: i128 = i32::MAX as i128;
                    if lr.0 >= I32MIN && lr.1 <= I32MAX && rr.0 >= I32MIN && rr.1 <= I32MAX {
                        let i32_instr = match op {
                            "+" => "add",
                            "-" => "sub",
                            "*" => "mul",
                            "%" => "urem",
                            _ => unreachable!(),
                        };
                        // Reuse the i32 form of an operand if it was itself a
                        // narrowed result (avoids a sext/trunc round-trip that
                        // would break the i32 SSA chain and disable vectorization).
                        let la = self.narrow_operand_i32(&lv);
                        let ra = self.narrow_operand_i32(&rv);
                        let t32 = self.fresh_temp();
                        let flags = if op == "%" { "" } else { " nuw nsw" };
                        self.out.push_str(&format!(
                            "  {} = {}{} i32 {}, {}\n",
                            t32, i32_instr, flags, la, ra
                        ));
                        let tmp = self.fresh_temp();
                        self.out.push_str(&format!(
                            "  {} = sext i32 {} to i64\n",
                            tmp, t32
                        ));
                        self.i32_form.insert(tmp.clone(), t32.clone());
                        return Ok((tmp, Type::Int));
                    }
                }
            }
        }

        // Phase 7: Operator interface lowering (resolved_operator -> direct LLVM call)
        if let Some(ResolvedOperator::MethodCall { method, op: mop }) = resolved_operator {
            let sname = match &lt {
                Type::Struct(s) => s.clone(),
                _ => return Err("Operator interface requires struct type".to_string()),
            };
            let method_func = format!("{}_{}", sname, method);
            let ll_arg = |v: &str, t: &Type| -> String {
                if v.starts_with('%') {
                    format!("{} {}", llvm_type_name(t), v)
                } else {
                    v.to_string()
                }
            };
            let lhs_arg = ll_arg(&lv, &lt);
            let rhs_arg = ll_arg(&rv, &rt);
            match method.as_str() {
                "add" => {
                    let tmp = self.fresh_temp();
                    let ret_llty = llvm_type_name(&lt);
                    self.out.push_str(&format!(
                        "  {} = call {} @{}({}, {})\n",
                        tmp, ret_llty, method_func, lhs_arg, rhs_arg
                    ));
                    let ret_ty = lt.clone();
                    return Ok((tmp, ret_ty));
                }
                "equal" => {
                    let tmp = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = call i1 @{}({}, {})\n",
                        tmp, method_func, lhs_arg, rhs_arg
                    ));
                    if mop == "!=" {
                        let neg = self.fresh_temp();
                        self.out.push_str(&format!("  {} = xor i1 {}, true\n", neg, tmp));
                        return Ok((neg, Type::Bool));
                    }
                    return Ok((tmp, Type::Bool));
                }
                "compare" => {
                    let tmp = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = call i64 @{}({}, {})\n",
                        tmp, method_func, lhs_arg, rhs_arg
                    ));
                    let cmp = self.fresh_temp();
                    let (cmp_instr, cmp_val) = match mop.as_str() {
                        "<" => ("icmp slt", 0i64),
                        ">" => ("icmp sgt", 0i64),
                        "<=" => ("icmp sle", 0i64),
                        ">=" => ("icmp sge", 0i64),
                        _ => return Err(format!("Unknown comparison op for compare(): {}", mop)),
                    };
                    self.out.push_str(&format!(
                        "  {} = {} i64 {}, {}\n",
                        cmp, cmp_instr, tmp, cmp_val
                    ));
                    return Ok((cmp, Type::Bool));
                }
                _ => return Err(format!("Unknown operator interface method: {}", method)),
            }
        }

        // Phase 5: string concatenation
        if lt == Type::String && rt == Type::String && op == "+" {
            let tmp = self.fresh_temp();
            self.out.push_str(&format!(
                "  {} = call i8* @runtime_str_concat(i8* {}, i8* {})\n",
                tmp, lv, rv
            ));
            return Ok((tmp, Type::String));
        }

        let float = is_float(&lt);
        let tmp = self.fresh_temp();
        match op {
            "+" | "-" | "*" | "/" | "%" => {
                let (instr, llty) = if float {
                    let i = match op {
                        "+" => "fadd",
                        "-" => "fsub",
                        "*" => "fmul",
                        "/" => "fdiv",
                        "%" => "frem",
                        _ => unreachable!(),
                    };
                    (i, "double")
                } else {
                    let i = match op {
                        "+" => "add",
                        "-" => "sub",
                        "*" => "mul",
                        "/" => "sdiv",
                        "%" => "srem",
                        _ => unreachable!(),
                    };
                    (i, "i64")
                };
                let flags = if float || op == "/" || op == "%" {
                    ""
                } else {
                    " nuw nsw"
                };
                self.out.push_str(&format!(
                    "  {} = {}{} {} {}, {}\n",
                    tmp, instr, flags, llty, self.bare_value(&lv), self.bare_value(&rv)
                ));
                let ty = if float { Type::Float } else { Type::Int };
                Ok((tmp, ty))
            }
            "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                // String equality must go through runtime_str_equals (pointer
                // comparison via icmp is wrong). Route == / != here.
                if lt == Type::String && (op == "==" || op == "!=") {
                    let tmp = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = call i32 @runtime_str_equals(i8* {}, i8* {})\n",
                        tmp,
                        self.bare_value(&lv),
                        self.bare_value(&rv)
                    ));
                    if op == "!=" {
                        let neg = self.fresh_temp();
                        self.out.push_str(&format!("  {} = icmp eq i32 {}, 0\n", neg, tmp));
                        return Ok((neg, Type::Bool));
                    }
                    let iseq = self.fresh_temp();
                    self.out.push_str(&format!("  {} = icmp ne i32 {}, 0\n", iseq, tmp));
                    return Ok((iseq, Type::Bool));
                }
                let (instr, llty) = if float {
                    let i = match op {
                        "==" => "fcmp oeq",
                        "!=" => "fcmp one",
                        "<" => "fcmp olt",
                        ">" => "fcmp ogt",
                        "<=" => "fcmp ole",
                        ">=" => "fcmp oge",
                        _ => unreachable!(),
                    };
                    (i, "double")
                } else {
                    let i = match op {
                        "==" => "icmp eq",
                        "!=" => "icmp ne",
                        "<" => "icmp slt",
                        ">" => "icmp sgt",
                        "<=" => "icmp sle",
                        ">=" => "icmp sge",
                        _ => unreachable!(),
                    };
                    (i, "i64")
                };
                self.out.push_str(&format!(
                    "  {} = {} {} {}, {}\n",
                    tmp, instr, llty, self.bare_value(&lv), self.bare_value(&rv)
                ));
                Ok((tmp, Type::Bool))
            }
            "and" => {
                let prev = self.current_block.clone();
                let rhs_b = self.fresh_block();
                let end_b = self.fresh_block();
                self.out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", self.bare_value(&lv), rhs_b, end_b));
                self.out.push_str(&format!("{}:\n", rhs_b));
                self.current_block = rhs_b.clone();
                self.out.push_str(&format!("  br label %{}\n", end_b));
                self.out.push_str(&format!("{}:\n", end_b));
                self.current_block = end_b;
                let res = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = phi i1 [ false, %{} ], [ {}, %{} ]\n",
                    res, prev, self.bare_value(&rv), rhs_b
                ));
                Ok((res, Type::Bool))
            }
            "or" => {
                let prev = self.current_block.clone();
                let rhs_b = self.fresh_block();
                let end_b = self.fresh_block();
                self.out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", self.bare_value(&lv), end_b, rhs_b));
                self.out.push_str(&format!("{}:\n", rhs_b));
                self.current_block = rhs_b.clone();
                self.out.push_str(&format!("  br label %{}\n", end_b));
                self.out.push_str(&format!("{}:\n", end_b));
                self.current_block = end_b;
                let res = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = phi i1 [ true, %{} ], [ {}, %{} ]\n",
                    res, prev, self.bare_value(&rv), rhs_b
                ));
                Ok((res, Type::Bool))
            }
            _ => Err(format!("Phase 1: unsupported binary operator '{}'", op)),
        }
    }

    /// Phase 2+3: function call codegen (builtin / struct ctor / user functions)
    fn codegen_call(&mut self, func: &str, args: &[Expr]) -> Result<(String, Type), String> {
        // Builtin print/println
        if func == "print" || func == "println" {
            let add_nl = func == "println";
            for arg in args {
                self.codegen_print(arg, add_nl)?;
            }
            return Ok((String::new(), Type::Unit));
        }
        // Builtin panic: print the message to stderr and abort the program.
        if func == "panic" {
            if args.len() != 1 {
                return Err("panic() takes exactly 1 argument".to_string());
            }
            let (v, t) = self.codegen_expr(&args[0])?;
            if t != Type::String {
                return Err(format!(
                    "panic() argument must be a string, got {:?}",
                    t
                ));
            }
            self.out.push_str(&format!(
                "  call void @runtime_panic({})\n",
                self.fmt_call_arg(&v, &Type::String)
            ));
            return Ok((String::new(), Type::Unit));
        }
        // Builtin str(): converts a value to its string form. For String
        // arguments this is an identity; for an `unknown`-typed value (e.g. an
        // `Error(e)` payload whose concrete type was never pinned, which is a
        // string at runtime) it is a pass-through. Int/float/bool require a
        // runtime conversion helper.
        if func == "str" {
            if args.len() != 1 {
                return Err("str() takes exactly 1 argument".to_string());
            }
            let (v, t) = self.codegen_expr(&args[0])?;
            match t {
                Type::String | Type::Unknown => {
                    return Ok((v, Type::String));
                }
                Type::Int => {
                    let tmp = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = call i8* @runtime_str_from_i64({})\n",
                        tmp, self.fmt_call_arg(&v, &Type::Int)
                    ));
                    return Ok((tmp, Type::String));
                }
                Type::Float => {
                    let tmp = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = call i8* @runtime_str_from_f64({})\n",
                        tmp, self.fmt_call_arg(&v, &Type::Float)
                    ));
                    return Ok((tmp, Type::String));
                }
                Type::Bool => {
                    let tmp = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = call i8* @runtime_str_from_bool({})\n",
                        tmp, self.fmt_call_arg(&v, &Type::Bool)
                    ));
                    return Ok((tmp, Type::String));
                }
                Type::Option(_) => {
                    // Option(T) is %Option = { i32, [4 x i64] }.
                    // Tag (field 0): 0=Some, 1=None. Payload (field 1, [0]): i64.
                    let tmp_tag = self.fresh_temp();
                    let tmp_payload = self.fresh_temp();
                    let tmp_result = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = extractvalue %Option {}, 0\n",
                        tmp_tag, v
                    ));
                    self.out.push_str(&format!(
                        "  {} = extractvalue %Option {}, 1, 0\n",
                        tmp_payload, v
                    ));
                    self.out.push_str(&format!(
                        "  {} = call i8* @runtime_str_from_option(i64 {}, i32 {})\n",
                        tmp_result, tmp_payload, tmp_tag
                    ));
                    return Ok((tmp_result, Type::String));
                }
                Type::State(ref state_name) => {
                    let base = crate::struct_base(state_name);
                    let llvm_ty = format!("%{}", base);
                    if base == "Option" {
                        // Option(T) is %Option = { i32, [4 x i64] }.
                        // Tag (field 0): 0=Some, 1=None. Payload (field 1, [0]): i64.
                        let tmp_tag = self.fresh_temp();
                        let tmp_payload = self.fresh_temp();
                        let tmp_result = self.fresh_temp();
                        self.out.push_str(&format!(
                            "  {} = extractvalue {} {}, 0\n",
                            tmp_tag, llvm_ty, v
                        ));
                        self.out.push_str(&format!(
                            "  {} = extractvalue {} {}, 1, 0\n",
                            tmp_payload, llvm_ty, v
                        ));
                        self.out.push_str(&format!(
                            "  {} = call i8* @runtime_str_from_option(i64 {}, i32 {})\n",
                            tmp_result, tmp_payload, tmp_tag
                        ));
                        return Ok((tmp_result, Type::String));
                    }
                    if base == "Result" {
                        // Result(T, E) is %Result = { i32, [4 x i64] }.
                        // Tag (field 0): 0=Success, 1=Error. Payload (field 1, [0]): i64.
                        let tmp_tag = self.fresh_temp();
                        let tmp_payload = self.fresh_temp();
                        let tmp_result = self.fresh_temp();
                        self.out.push_str(&format!(
                            "  {} = extractvalue {} {}, 0\n",
                            tmp_tag, llvm_ty, v
                        ));
                        self.out.push_str(&format!(
                            "  {} = extractvalue {} {}, 1, 0\n",
                            tmp_payload, llvm_ty, v
                        ));
                        self.out.push_str(&format!(
                            "  {} = call i8* @runtime_str_from_result(i64 {}, i32 {})\n",
                            tmp_result, tmp_payload, tmp_tag
                        ));
                        return Ok((tmp_result, Type::String));
                    }
                    return Err(format!(
                        "str() cannot convert {:?} to a string in codegen",
                        t
                    ));
                }
                Type::Json => {
                    let tmp = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = call i8* @runtime_json_stringify(i8* {})\n",
                        tmp, self.fmt_call_arg(&v, &Type::Json)
                    ));
                    return Ok((tmp, Type::String));
                }
                _ => return Err(format!(
                    "str() cannot convert {:?} to a string in codegen",
                    t
                )),
            }
        }
        // Phase 12 Step 1: stdlib runtime builtins (string/math/time/fs/io)
        if let Some(result) = self.codegen_runtime_builtin(func, args)? {
            return Ok(result);
        }
        // Phase 3: struct constructor
        if let Some(sdef) = self.defs.structs.get(func) {
            return self.codegen_struct_ctor(func, sdef, args);
        }
        // Phase 12 Step 1: package-qualified struct constructor called by its
        // bare name (e.g. `Instant(f)` resolves to `time.Instant`).
        if let Some(resolved) = self.defs.resolve_type(func) {
            if let Some(sdef) = self.defs.structs.get(&resolved) {
                return self.codegen_struct_ctor(&resolved, sdef, args);
            }
        }
        // Phase 4: state variant constructor (Success/Error or user state variant)
        if let Some(state_name) = self.defs.state_variants.get(func) {
            return self.codegen_state_ctor(state_name, func, args);
        }
        // Phase B-2.2: function value call (variable holds a closure or fn ref)
        if let Some(ty) = self.env.get(func).cloned() {
            if let Type::Fn(param_types, ret) = ty {
                return self.codegen_closure_call(func, args, &param_types, &ret);
            }
        }
        // Phase 6: monomorphized generic function call
        if let Some(mangled) = self.mono_name_map.get(func) {
            let mono_fdef = self.mono_fdefs.get(mangled).ok_or_else(|| {
                format!("monomorphized function '{}' not found", mangled)
            })?;
            let llvm_name = mangled;
            let mut call_args = Vec::new();
            for (arg, (_, ptype)) in args.iter().zip(&mono_fdef.params) {
                let (v, _) = self.codegen_expr(arg)?;
                let t = type_from_str(ptype, self.defs);
                call_args.push(self.fmt_call_arg(&v, &t));
            }
            let call_args_str = call_args.join(", ");
            let ret_type = match &mono_fdef.return_type {
                Some(rt) => {
                    let t = type_from_str(rt, self.defs);
                    (llvm_type_name(&t), t)
                }
                None => ("void".to_string(), Type::Unit),
            };
            if ret_type.0 == "void" {
                self.out.push_str(&format!(
                    "  call void @{}({})\n",
                    llvm_name, call_args_str
                ));
                return Ok((String::new(), Type::Unit));
            } else {
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call {} @{}({})\n",
                    tmp, ret_type.0, llvm_name, call_args_str
                ));
                return Ok((tmp, ret_type.1));
            }
        }
        // User function call
        let fdef = self
            .defs
            .functions
            .get(func)
            // Phase 12 Step 1: resolve a bare user-function name to its
            // package-qualified key (e.g. a package function calling another).
            .or_else(|| {
                self.defs
                    .resolve_function(func)
                    .and_then(|resolved| self.defs.functions.get(&resolved))
            })
            .ok_or_else(|| format!("undefined function '{}'", func))?;
        let llvm_name = if func == "main" { "main_lime" } else { func };
        let mut call_args = Vec::new();
        for (arg, (_, ptype)) in args.iter().zip(&fdef.params) {
            let (v, _) = self.codegen_expr(arg)?;
            let t = type_from_str(ptype, self.defs);
            call_args.push(self.fmt_call_arg(&v, &t));
        }
        let call_args_str = call_args.join(", ");
        let ret_type = match &fdef.return_type {
            Some(rt) => {
                let t = type_from_str(rt, self.defs);
                (llvm_type_name(&t), t)
            }
            None => ("void".to_string(), Type::Unit),
        };
        if ret_type.0 == "void" {
            self.out.push_str(&format!(
                "  call void @{}({})\n",
                llvm_name, call_args_str
            ));
            Ok((String::new(), Type::Unit))
        } else {
            let tmp = self.fresh_temp();
            self.out.push_str(&format!(
                "  {} = call {} @{}({})\n",
                tmp, ret_type.0, llvm_name, call_args_str
            ));
            Ok((tmp, ret_type.1))
        }
    }

    /// Phase B-2.2: call through a closure / function value.
    /// The closure stores a function pointer with signature (i8* %env, i8* %packed) -> Ret.
    /// This method loads the closure, packs arguments, and calls through the function pointer.
    fn codegen_closure_call(
        &mut self,
        func: &str,
        args: &[Expr],
        _param_types: &[Type],
        ret: &Type,
    ) -> Result<(String, Type), String> {
        // Load the %LimeClosure* from the variable's alloca
        let closure_ptr = self.named.get(func).cloned().ok_or_else(|| {
            format!("undefined variable '{}'", func)
        })?;
        let closure_loaded = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = load %LimeClosure*, %LimeClosure** {}, align 8\n",
            closure_loaded, closure_ptr
        ));

        // Extract fn_ptr (field 0) and env_ptr (field 1)
        let fn_ptr_gep = self.fresh_temp();
        let fn_ptr_tmp = self.fresh_temp();
        let env_ptr_gep = self.fresh_temp();
        let env_ptr_tmp = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = getelementptr inbounds %LimeClosure, %LimeClosure* {}, i32 0, i32 0\n",
            fn_ptr_gep, closure_loaded
        ));
        self.out.push_str(&format!(
            "  {} = load i8*, i8** {}\n",
            fn_ptr_tmp, fn_ptr_gep
        ));
        self.out.push_str(&format!(
            "  {} = getelementptr inbounds %LimeClosure, %LimeClosure* {}, i32 0, i32 1\n",
            env_ptr_gep, closure_loaded
        ));
        self.out.push_str(&format!(
            "  {} = load i8*, i8** {}\n",
            env_ptr_tmp, env_ptr_gep
        ));

        // Evaluate arguments and pack into heap-allocated i64 array
        let mut arg_vals: Vec<(String, Type)> = Vec::new();
        for a in args {
            arg_vals.push(self.codegen_expr(a)?);
        }

        let packed_ptr = if arg_vals.is_empty() {
            "i8* null".to_string()
        } else {
            let struct_size = arg_vals.len() as i64 * 8;
            let raw_alloc = self.fresh_temp();
            self.out.push_str(&format!(
                "  {} = call i8* @runtime_alloc(i64 {}, i64 8)\n",
                raw_alloc, struct_size
            ));
            for (i, (v, t)) in arg_vals.iter().enumerate() {
                let bare = self.bare_value(v).to_string();
                let offset = (i as i64) * 8;
                let elem_ptr = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = getelementptr i8, i8* {}, i64 {}\n",
                    elem_ptr, raw_alloc, offset
                ));
                let ptr_i64 = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = bitcast i8* {} to i64*\n",
                    ptr_i64, elem_ptr
                ));
                match t {
                    Type::Int | Type::Long => {
                        self.out.push_str(&format!(
                            "  store i64 {}, i64* {}, align 8\n",
                            bare, ptr_i64
                        ));
                    }
                    Type::Float => {
                        let raw = self.fresh_temp();
                        self.out.push_str(&format!(
                            "  {} = bitcast double {} to i64\n",
                            raw, bare
                        ));
                        self.out.push_str(&format!(
                            "  store i64 {}, i64* {}, align 8\n",
                            raw, ptr_i64
                        ));
                    }
                    Type::Bool => {
                        let ext = self.fresh_temp();
                        self.out.push_str(&format!(
                            "  {} = zext i1 {} to i64\n",
                            ext, bare
                        ));
                        self.out.push_str(&format!(
                            "  store i64 {}, i64* {}, align 8\n",
                            ext, ptr_i64
                        ));
                    }
                    _ => {
                        let bc = self.fresh_temp();
                        self.out.push_str(&format!(
                            "  {} = ptrtoint i8* {} to i64\n",
                            bc, bare
                        ));
                        self.out.push_str(&format!(
                            "  store i64 {}, i64* {}, align 8\n",
                            bc, ptr_i64
                        ));
                    }
                }
            }
            raw_alloc
        };

        // Cast fn_ptr to the wrapper type and call: Ret (i8*, i8*)
        let ll_ret = llvm_type_name(ret);
        let wrapper_ty = format!("{} (i8*, i8*)", ll_ret);
        let wrapper_cast = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = bitcast i8* {} to {}*\n",
            wrapper_cast, fn_ptr_tmp, wrapper_ty
        ));
        let tmp = self.fresh_temp();
        if ll_ret == "void" {
            self.out.push_str(&format!(
                "  call {} (i8*, i8*) {}(i8* {}, i8* {})\n",
                ll_ret, wrapper_cast, env_ptr_tmp, packed_ptr
            ));
            Ok((String::new(), Type::Unit))
        } else {
            self.out.push_str(&format!(
                "  {} = call {} (i8*, i8*) {}(i8* {}, i8* {})\n",
                tmp, ll_ret, wrapper_cast, env_ptr_tmp, packed_ptr
            ));
            Ok((tmp, ret.clone()))
        }
    }

    /// Phase 12 Step 1: stdlib runtime builtin lowering.
    ///
    /// Lowers the bare runtime builtins that the stdlib packages wrap
    /// (`string`/`math`/`time`/`fs`/`io`) to calls into the C runtime helpers
    /// declared in src/codegen/mod.rs. Returns `Ok(None)` when `func` is not a
    /// builtin handled here so the caller can continue to user-function lookup.
    fn codegen_runtime_builtin(&mut self, func: &str, args: &[Expr]) -> Result<Option<(String, Type)>, String> {
        // Evaluate an argument and format it for a call of the given type.
        fn arg(cg: &mut Cg, e: &Expr, want: &Type) -> Result<String, String> {
            let (v, t) = cg.codegen_expr(e)?;
            if &t != want {
                return Err(format!(
                    "builtin argument type mismatch: expected {:?}, got {:?}",
                    want, t
                ));
            }
            Ok(cg.fmt_call_arg(&v, want))
        }
        // Evaluate an argument without type checking (for polymorphic builtins).
        fn any_arg(cg: &mut Cg, e: &Expr) -> Result<String, String> {
            let (v, _t) = cg.codegen_expr(e)?;
            Ok(v)
        }
        // N string arguments -> i8* formatted list.
        fn str_args(cg: &mut Cg, exprs: &[Expr], n: usize) -> Result<Vec<String>, String> {
            if exprs.len() != n {
                return Err(format!("builtin expects {} string argument(s), got {}", n, exprs.len()));
            }
            exprs.iter().map(|e| arg(cg, e, &Type::String)).collect()
        }
        fn str_arg(cg: &mut Cg, e: &Expr) -> Result<String, String> {
            str_args(cg, std::slice::from_ref(e), 1).map(|v| v.into_iter().next().unwrap())
        }
        // N float arguments -> double formatted list.
        fn f64_args(cg: &mut Cg, exprs: &[Expr], n: usize) -> Result<Vec<String>, String> {
            if exprs.len() != n {
                return Err(format!("builtin expects {} float argument(s), got {}", n, exprs.len()));
            }
            exprs.iter().map(|e| arg(cg, e, &Type::Float)).collect()
        }
        fn f64_arg(cg: &mut Cg, e: &Expr) -> Result<String, String> {
            f64_args(cg, std::slice::from_ref(e), 1).map(|v| v.into_iter().next().unwrap())
        }
        fn i64_arg(cg: &mut Cg, e: &Expr) -> Result<String, String> {
            arg(cg, e, &Type::Int)
        }
        // Codegen a list expression, store it in a stack slot, and return
        // the pointer. This is needed for runtime functions that accept
        // %LimeList* (e.g. join).
        fn list_arg(cg: &mut Cg, e: &Expr) -> Result<String, String> {
            let (val, _ty) = cg.codegen_expr(e)?;
            let slot = cg.fresh_temp();
            cg.out.push_str(&format!(
                "  {} = alloca %LimeList, align 8\n",
                slot
            ));
            cg.out.push_str(&format!(
                "  store %LimeList {}, ptr {}, align 8\n",
                val, slot
            ));
            Ok(slot)
        }

        // Emit a call returning an i8* string.
        fn call_str(cg: &mut Cg, helper: &str, call_args: &[String]) -> (String, Type) {
            let tmp = cg.fresh_temp();
            cg.out.push_str(&format!(
                "  {} = call i8* @{}({})\n",
                tmp, helper, call_args.join(", ")
            ));
            (tmp, Type::String)
        }
        // Emit a call returning an int (0/1) and convert it to an i1 bool.
        fn call_bool(cg: &mut Cg, helper: &str, call_args: &[String]) -> (String, Type) {
            let tmp = cg.fresh_temp();
            let flag = cg.fresh_temp();
            cg.out.push_str(&format!(
                "  {} = call i32 @{}({})\n",
                tmp, helper, call_args.join(", ")
            ));
            cg.out.push_str(&format!("  {} = icmp ne i32 {}, 0\n", flag, tmp));
            (flag, Type::Bool)
        }
        // Emit a call returning a double.
        fn call_f64(cg: &mut Cg, helper: &str, call_args: &[String]) -> (String, Type) {
            let tmp = cg.fresh_temp();
            cg.out.push_str(&format!(
                "  {} = call double @{}({})\n",
                tmp, helper, call_args.join(", ")
            ));
            (tmp, Type::Float)
        }
        // Emit a call returning an i64.
        fn call_i64(cg: &mut Cg, helper: &str, call_args: &[String]) -> (String, Type) {
            let tmp = cg.fresh_temp();
            cg.out.push_str(&format!(
                "  {} = call i64 @{}({})\n",
                tmp, helper, call_args.join(", ")
            ));
            (tmp, Type::Int)
        }
        // Emit a call returning an i32, sign-extend to i64 to match Type::Int.
        fn call_i32(cg: &mut Cg, helper: &str, call_args: &[String]) -> (String, Type) {
            let tmp = cg.fresh_temp();
            let result = cg.fresh_temp();
            cg.out.push_str(&format!(
                "  {} = call i32 @{}({})\n",
                tmp, helper, call_args.join(", ")
            ));
            cg.out.push_str(&format!(
                "  {} = sext i32 {} to i64\n",
                result, tmp
            ));

            (result, Type::Int)
        }
        // Emit a call returning a %LimeList via sret (strings boxed as i64
        // element slots, matching `split`/`fs_list_dir`).
        fn call_list(cg: &mut Cg, helper: &str, call_args: &[String]) -> (String, Type) {
            let slot = cg.fresh_temp();
            let tmp = cg.fresh_temp();
            cg.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", slot));
            cg.out.push_str(&format!(
                "  call void @{}(ptr sret(%LimeList) {}, {})\n",
                helper, slot, call_args.join(", ")
            ));
            cg.out.push_str(&format!("  {} = load %LimeList, ptr {}, align 8\n", tmp, slot));
            (tmp, Type::List(Box::new(Type::String)))
        }

        match func {
            // ---- string builtins ----
            "len" => {
                if args.len() != 1 {
                    return Err("len() takes exactly 1 argument".to_string());
                }
                let (v, t) = self.codegen_expr(&args[0])?;
                match t {
                    Type::String => {
                        let s = self.fmt_call_arg(&v, &Type::String);
                        let tmp = self.fresh_temp();
                        self.out.push_str(&format!("  {} = call i64 @strlen({})\n", tmp, s));
                        Ok(Some((tmp, Type::Int)))
                    }
                    Type::List(_) => {
                        let tmp = self.fresh_temp();
                        self.out.push_str(&format!(
                            "  {} = extractvalue %LimeList {}, 1\n",
                            tmp,
                            self.bare_value(&v)
                        ));
                        Ok(Some((tmp, Type::Int)))
                    }
                    other => Err(format!("builtin len() not supported for {:?}", other)),
                }
            }
            "byte_len" => {
                let s = str_arg(self, &args[0])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = call i64 @strlen({})\n", tmp, s));
                Ok(Some((tmp, Type::Int)))
            }
            "contains" => {
                let a = str_args(self, args, 2)?;
                let (v, t) = call_bool(self, "runtime_str_contains", &a);
                Ok(Some((v, t)))
            }
            "starts_with" => {
                let a = str_args(self, args, 2)?;
                let (v, t) = call_bool(self, "runtime_str_starts_with", &a);
                Ok(Some((v, t)))
            }
            "ends_with" => {
                let a = str_args(self, args, 2)?;
                let (v, t) = call_bool(self, "runtime_str_ends_with", &a);
                Ok(Some((v, t)))
            }
            "trim" => {
                let s = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_str_trim", &[s]);
                Ok(Some((v, t)))
            }
            "to_upper" => {
                let s = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_str_to_upper", &[s]);
                Ok(Some((v, t)))
            }
            "to_lower" => {
                let s = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_str_to_lower", &[s]);
                Ok(Some((v, t)))
            }
            "replace" => {
                let a = str_args(self, args, 3)?;
                let (v, t) = call_str(self, "runtime_str_replace", &a);
                Ok(Some((v, t)))
            }
            "repeat" => {
                let s = str_arg(self, &args[0])?;
                let n = i64_arg(self, &args[1])?;
                let (v, t) = call_str(self, "runtime_str_repeat", &[s, n]);
                Ok(Some((v, t)))
            }
            "slice" => {
                let s = str_arg(self, &args[0])?;
                let start = i64_arg(self, &args[1])?;
                let end = i64_arg(self, &args[2])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_str_slice({}, {}, {})\n",
                    tmp, s, start, end
                ));
                Ok(Some((tmp, Type::String)))
            }
            "split" => {
                let a = str_args(self, args, 2)?;
                let (v, t) = call_list(self, "runtime_str_split", &a);
                Ok(Some((v, t)))
            }
            "is_empty" => {
                let s = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_str_is_empty", &[s]);
                Ok(Some((v, t)))
            }
            "find" => {
                let a = str_args(self, args, 2)?;
                let (v, t) = call_i64(self, "runtime_str_find", &a);
                Ok(Some((v, t)))
            }
            "count" => {
                let a = str_args(self, args, 2)?;
                let (v, t) = call_i64(self, "runtime_str_count", &a);
                Ok(Some((v, t)))
            }
            "trim_start" => {
                let s = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_str_trim_start", &[s]);
                Ok(Some((v, t)))
            }
            "trim_end" => {
                let s = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_str_trim_end", &[s]);
                Ok(Some((v, t)))
            }
            "join" => {
                let sep = str_arg(self, &args[0])?;
                let list = list_arg(self, &args[1])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_str_join(ptr {}, {})\n",
                    tmp, list, sep
                ));
                Ok(Some((tmp, Type::String)))
            }
            "to_int" => {
                let s = str_arg(self, &args[0])?;
                let (v, t) = call_i64(self, "runtime_str_to_int", &[s]);
                Ok(Some((v, t)))
            }
            "to_float" => {
                let s = str_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_str_to_float", &[s]);
                Ok(Some((v, t)))
            }
            "equals" => {
                let a = str_args(self, args, 2)?;
                let (v, t) = call_bool(self, "runtime_str_equals", &a);
                Ok(Some((v, t)))
            }
            "compare" => {
                let a = str_args(self, args, 2)?;
                let (v, t) = call_i32(self, "runtime_str_compare", &a);
                Ok(Some((v, t)))
            }
            // ---- math builtins ----
            "abs" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_abs", &[x]);
                Ok(Some((v, t)))
            }
            "sqrt" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_sqrt", &[x]);
                Ok(Some((v, t)))
            }
            "min" => {
                let a = f64_args(self, args, 2)?;
                let (v, t) = call_f64(self, "runtime_math_min", &a);
                Ok(Some((v, t)))
            }
            "max" => {
                let a = f64_args(self, args, 2)?;
                let (v, t) = call_f64(self, "runtime_math_max", &a);
                Ok(Some((v, t)))
            }
            "clamp" => {
                let a = f64_args(self, args, 3)?;
                let (v, t) = call_f64(self, "runtime_math_clamp", &a);
                Ok(Some((v, t)))
            }
            "pow" => {
                let a = f64_args(self, args, 2)?;
                let (v, t) = call_f64(self, "runtime_math_pow", &a);
                Ok(Some((v, t)))
            }
            "floor" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_floor", &[x]);
                Ok(Some((v, t)))
            }
            "ceil" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_ceil", &[x]);
                Ok(Some((v, t)))
            }
            "round" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_round", &[x]);
                Ok(Some((v, t)))
            }
            "trunc" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_trunc", &[x]);
                Ok(Some((v, t)))
            }
            "exp" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_exp", &[x]);
                Ok(Some((v, t)))
            }
            "log" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_log", &[x]);
                Ok(Some((v, t)))
            }
            "log10" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_log10", &[x]);
                Ok(Some((v, t)))
            }
            "sin" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_sin", &[x]);
                Ok(Some((v, t)))
            }
            "cos" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_cos", &[x]);
                Ok(Some((v, t)))
            }
            "tan" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_tan", &[x]);
                Ok(Some((v, t)))
            }
            "asin" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_asin", &[x]);
                Ok(Some((v, t)))
            }
            "acos" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_acos", &[x]);
                Ok(Some((v, t)))
            }
            "atan" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_f64(self, "runtime_math_atan", &[x]);
                Ok(Some((v, t)))
            }
            "math_pi" => {
                let (v, t) = call_f64(self, "runtime_math_pi", &[]);
                Ok(Some((v, t)))
            }
            "math_e" => {
                let (v, t) = call_f64(self, "runtime_math_e", &[]);
                Ok(Some((v, t)))
            }
            // ---- time builtins ----
            "time_now" => {
                let (v, t) = call_f64(self, "runtime_time_now", &[]);
                Ok(Some((v, t)))
            }
            "time_sleep" => {
                let x = f64_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_time_sleep", &[x]);
                Ok(Some((v, t)))
            }
            // ---- stdio builtin ----
            "input" => {
                if args.len() > 1 {
                    return Err("input() takes at most 1 argument".to_string());
                }
                let prompt = match args.first() {
                    Some(p) => str_arg(self, p)?,
                    None => "i8* null".to_string(),
                };
                let (v, t) = call_str(self, "runtime_input", &[prompt]);
                Ok(Some((v, t)))
            }
            "eprint" | "eprintln" => {
                for arg in args {
                    let (v, _) = self.codegen_expr(arg)?;
                    let rt = if func == "eprint" { "runtime_eprint" } else { "runtime_eprintln" };
                    self.out.push_str(&format!(
                        "  call void @{}({})\n",
                        rt,
                        self.fmt_call_arg(&v, &Type::String)
                    ));
                }
                Ok(Some((String::new(), Type::Unit)))
            }
            "io_read_line" => {
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = call i8* @runtime_read_line()\n", tmp));
                Ok(Some((tmp, Type::String)))
            }
            "io_read_all" => {
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = call i8* @runtime_read_all()\n", tmp));
                Ok(Some((tmp, Type::String)))
            }
            "io_write_stdout" => {
                let p = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_write_stdout", &[p]);
                Ok(Some((v, t)))
            }
            "io_write_stderr" => {
                let p = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_write_stderr", &[p]);
                Ok(Some((v, t)))
            }
            // ---- filesystem builtins ----
            "read_file" => {
                let p = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_read_file", &[p]);
                Ok(Some((v, t)))
            }
            "write_file" => {
                let a = str_args(self, args, 2)?;
                let (v, t) = call_bool(self, "runtime_write_file", &a);
                Ok(Some((v, t)))
            }
            "append_file" => {
                let a = str_args(self, args, 2)?;
                let (v, t) = call_bool(self, "runtime_append_file", &a);
                Ok(Some((v, t)))
            }
            "file_exists" | "fs_exists" => {
                let p = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_file_exists", &[p]);
                Ok(Some((v, t)))
            }
            "remove_file" => {
                let p = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_remove_file", &[p]);
                Ok(Some((v, t)))
            }
            "fs_create_dir" => {
                let p = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_fs_create_dir", &[p]);
                Ok(Some((v, t)))
            }
            "fs_size" => {
                let p = str_arg(self, &args[0])?;
                let (v, t) = call_i64(self, "runtime_fs_size", &[p]);
                Ok(Some((v, t)))
            }
            "fs_metadata" => {
                if args.len() != 1 {
                    return Err("fs_metadata() takes exactly 1 argument".to_string());
                }
                let p = str_arg(self, &args[0])?;
                let size_slot = self.fresh_temp();
                let isdir_slot = self.fresh_temp();
                let isfile_slot = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca i64, align 8\n", size_slot));
                self.out.push_str(&format!("  {} = alloca i8, align 1\n", isdir_slot));
                self.out.push_str(&format!("  {} = alloca i8, align 1\n", isfile_slot));
                self.out.push_str(&format!(
                    "  call void @runtime_fs_metadata({}, ptr {}, ptr {}, ptr {})\n",
                    p, size_slot, isdir_slot, isfile_slot
                ));
                let size = self.fresh_temp();
                self.out.push_str(&format!("  {} = load i64, ptr {}, align 8\n", size, size_slot));
                let isdir_raw = self.fresh_temp();
                let isdir = self.fresh_temp();
                self.out.push_str(&format!("  {} = load i8, ptr {}, align 1\n", isdir_raw, isdir_slot));
                self.out.push_str(&format!("  {} = icmp ne i8 {}, 0\n", isdir, isdir_raw));
                let isfile_raw = self.fresh_temp();
                let isfile = self.fresh_temp();
                self.out.push_str(&format!("  {} = load i8, ptr {}, align 1\n", isfile_raw, isfile_slot));
                self.out.push_str(&format!("  {} = icmp ne i8 {}, 0\n", isfile, isfile_raw));
                let t1 = self.fresh_temp();
                let t2 = self.fresh_temp();
                let t3 = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = insertvalue %fs.FileMetadata undef, i64 {}, 0\n",
                    t1, size
                ));
                self.out.push_str(&format!(
                    "  {} = insertvalue %fs.FileMetadata {}, i1 {}, 1\n",
                    t2, t1, isdir
                ));
                self.out.push_str(&format!(
                    "  {} = insertvalue %fs.FileMetadata {}, i1 {}, 2\n",
                    t3, t2, isfile
                ));
                Ok(Some((t3, Type::Struct("fs.FileMetadata".to_string()))))
            }
            "fs_list_dir" => {
                let p = str_arg(self, &args[0])?;
                let (v, t) = call_list(self, "runtime_fs_list_dir", &[p]);
                Ok(Some((v, t)))
            }
            "fs_copy" | "fs_rename" => {
                let a = str_args(self, args, 2)?;
                let rt = if func == "fs_copy" { "runtime_fs_copy" } else { "runtime_fs_rename" };
                let (v, t) = call_bool(self, rt, &a);
                Ok(Some((v, t)))
            }
            "fs_is_file" => {
                let p = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_fs_is_file", &[p]);
                Ok(Some((v, t)))
            }
            "fs_is_dir" => {
                let p = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_fs_is_dir", &[p]);
                Ok(Some((v, t)))
            }
            "fs_remove_dir" => {
                let p = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_fs_remove_dir", &[p]);
                Ok(Some((v, t)))
            }
            "fs_read_lines" => {
                let p = str_arg(self, &args[0])?;
                let (v, t) = call_list(self, "runtime_fs_read_lines", &[p]);
                Ok(Some((v, t)))
            }
            "fs_write_lines" => {
                if args.len() != 2 {
                    return Err("fs_write_lines() takes 2 arguments".to_string());
                }
                let p = str_arg(self, &args[0])?;
                let (list_v, _) = self.codegen_expr(&args[1])?;
                let (v, t) = call_bool(self, "runtime_fs_write_lines", &[p, self.bare_value(&list_v).to_string()]);
                Ok(Some((v, t)))
            }
            // ---- list builtins (Phase C-1.2) ----
            "list_insert" => {
                if args.len() != 3 {
                    return Err("list_insert() takes 3 arguments (list, index, elem)".to_string());
                }
                let (list_v, list_t) = self.codegen_expr(&args[0])?;
                let (idx_v, _) = self.codegen_expr(&args[1])?;
                let (elem_v, elem_t) = self.codegen_expr(&args[2])?;
                let converted = self.convert_to_i64(&elem_v, &elem_t)?;
                let slot = self.fresh_temp();
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", slot));
                self.out.push_str(&format!("  store %LimeList {}, ptr {}, align 8\n", self.bare_value(&list_v), slot));
                self.out.push_str(&format!(
                    "  call void @runtime_list_insert(ptr sret(%LimeList) {}, ptr {}, i64 {}, i64 {})\n",
                    slot, slot, self.bare_value(&idx_v), converted
                ));
                self.out.push_str(&format!("  {} = load %LimeList, ptr {}, align 8\n", tmp, slot));
                Ok(Some((tmp, list_t)))
            }
            "list_set" => {
                if args.len() != 3 {
                    return Err("list_set() takes 3 arguments (list, index, elem)".to_string());
                }
                let (list_v, list_t) = self.codegen_expr(&args[0])?;
                let (idx_v, _) = self.codegen_expr(&args[1])?;
                let (elem_v, elem_t) = self.codegen_expr(&args[2])?;
                let converted = self.convert_to_i64(&elem_v, &elem_t)?;
                let tmp = self.fresh_temp();
                let slot = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", slot));
                self.out.push_str(&format!("  store %LimeList {}, ptr {}, align 8\n", self.bare_value(&list_v), slot));
                self.out.push_str(&format!(
                    "  call void @runtime_list_set(ptr sret(%LimeList) {}, ptr {}, i64 {}, i64 {})\n",
                    slot, slot, self.bare_value(&idx_v), converted
                ));
                self.out.push_str(&format!("  {} = load %LimeList, ptr {}, align 8\n", tmp, slot));
                Ok(Some((tmp, list_t)))
            }
            "list_get" => {
                if args.len() != 2 {
                    return Err("list_get() takes 2 arguments (list, index)".to_string());
                }
                let (list_v, _) = self.codegen_expr(&args[0])?;
                let (idx_v, _) = self.codegen_expr(&args[1])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i64 @runtime_list_get(%LimeList {}, i64 {})\n",
                    tmp, self.bare_value(&list_v), self.bare_value(&idx_v)
                ));
                Ok(Some((tmp, Type::Int)))
            }
            "list_clear" | "list_sort" | "list_clone" => {
                if args.len() != 1 {
                    return Err(format!("{}() takes 1 argument", func));
                }
                let (list_v, list_t) = self.codegen_expr(&args[0])?;
                let rt = match func {
                    "list_clear" => "runtime_list_clear",
                    "list_sort" => "runtime_list_sort",
                    "list_clone" => "runtime_list_clone",
                    _ => unreachable!(),
                };
                let slot = self.fresh_temp();
                let tmp = self.fresh_temp();
                if func == "list_clone" {
                    // void runtime_list_clone(LimeList* dest, LimeList* src)
                    self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", slot));
                    self.out.push_str(&format!(
                        "  call void @{}(ptr {}, ptr {})\n",
                        rt, slot, self.bare_value(&list_v)
                    ));
                    self.out.push_str(&format!("  {} = load %LimeList, ptr {}, align 8\n", tmp, slot));
                } else {
                    // void runtime_list_clear/sort(LimeList* list)
                    self.out.push_str(&format!(
                        "  call void @{}(ptr {})\n",
                        rt, self.bare_value(&list_v)
                    ));
                    self.out.push_str(&format!("  {} = load %LimeList, ptr {}, align 8\n", tmp, self.bare_value(&list_v)));
                }
                Ok(Some((tmp, list_t)))
            }
            "list_empty" => {
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", tmp));
                self.out.push_str(&format!("  call void @runtime_list_empty(ptr {})\n", tmp));
                Ok(Some((tmp, Type::List(Box::new(Type::Unknown)))))
            }
            // ---- map builtins (Phase C-1.2) ----
            "map_len" | "map_size" => {
                let (map_v, _) = self.codegen_expr(&args[0])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i64 @runtime_map_len({})\n",
                    tmp, self.bare_value(&map_v)
                ));
                Ok(Some((tmp, Type::Int)))
            }
            "map_is_empty" => {
                let (map_v, _) = self.codegen_expr(&args[0])?;
                let (v, t) = call_bool(self, "runtime_map_is_empty", &[self.bare_value(&map_v).to_string()]);
                Ok(Some((v, t)))
            }
            "map_insert" => {
                if args.len() != 3 {
                    return Err("map_insert() takes 3 arguments (map, key, val)".to_string());
                }
                let (map_v, map_t) = self.codegen_expr(&args[0])?;
                let (key_v, key_t) = self.codegen_expr(&args[1])?;
                let (val_v, val_t) = self.codegen_expr(&args[2])?;
                let key = self.convert_to_i64(&key_v, &key_t)?;
                let val = self.convert_to_i64(&val_v, &val_t)?;
                let slot = self.fresh_temp();
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeMap, align 8\n", slot));
                self.out.push_str(&format!("  store %LimeMap {}, ptr {}, align 8\n", self.bare_value(&map_v), slot));
                self.out.push_str(&format!(
                    "  call void @runtime_map_insert(ptr sret(%LimeMap) {}, ptr {}, i64 {}, i64 {})\n",
                    slot, slot, key, val
                ));
                self.out.push_str(&format!("  {} = load %LimeMap, ptr {}, align 8\n", tmp, slot));
                Ok(Some((tmp, map_t)))
            }
            "map_get" => {
                let (map_v, _) = self.codegen_expr(&args[0])?;
                let (key_v, key_t) = self.codegen_expr(&args[1])?;
                let key = self.convert_to_i64(&key_v, &key_t)?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i64 @runtime_map_get(ptr {}, i64 {})\n",
                    tmp, self.bare_value(&map_v), key
                ));
                Ok(Some((tmp, Type::Int)))
            }
            "map_remove" => {
                let (map_v, map_t) = self.codegen_expr(&args[0])?;
                let (key_v, key_t) = self.codegen_expr(&args[1])?;
                let key = self.convert_to_i64(&key_v, &key_t)?;
                let slot = self.fresh_temp();
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeMap, align 8\n", slot));
                self.out.push_str(&format!("  store %LimeMap {}, ptr {}, align 8\n", self.bare_value(&map_v), slot));
                self.out.push_str(&format!(
                    "  call void @runtime_map_remove(ptr sret(%LimeMap) {}, ptr {}, i64 {})\n",
                    slot, slot, key
                ));
                self.out.push_str(&format!("  {} = load %LimeMap, ptr {}, align 8\n", tmp, slot));
                Ok(Some((tmp, map_t)))
            }
            "map_contains_key" => {
                let (map_v, _) = self.codegen_expr(&args[0])?;
                let (key_v, key_t) = self.codegen_expr(&args[1])?;
                let key = self.convert_to_i64(&key_v, &key_t)?;
                let (v, t) = call_bool(self, "runtime_map_contains_key", &[self.bare_value(&map_v).to_string(), key]);
                Ok(Some((v, t)))
            }
            "map_clear" | "map_clone" => {
                let (map_v, map_t) = self.codegen_expr(&args[0])?;
                let rt = if func == "map_clear" { "runtime_map_clear" } else { "runtime_map_clone" };
                let slot = self.fresh_temp();
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeMap, align 8\n", slot));
                self.out.push_str(&format!("  store %LimeMap {}, ptr {}, align 8\n", self.bare_value(&map_v), slot));
                self.out.push_str(&format!(
                    "  call void @{}(ptr sret(%LimeMap) {}, ptr {})\n",
                    rt, slot, slot
                ));
                self.out.push_str(&format!("  {} = load %LimeMap, ptr {}, align 8\n", tmp, slot));
                Ok(Some((tmp, map_t)))
            }
            // ---- set builtins (Phase C-1.2) ----
            "set_len" | "set_size" => {
                let (set_v, _) = self.codegen_expr(&args[0])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = call i64 @runtime_set_len({})\n", tmp, self.bare_value(&set_v)));
                Ok(Some((tmp, Type::Int)))
            }
            "set_is_empty" => {
                let (set_v, _) = self.codegen_expr(&args[0])?;
                let (v, t) = call_bool(self, "runtime_set_is_empty", &[self.bare_value(&set_v).to_string()]);
                Ok(Some((v, t)))
            }
            "set_add" | "set_remove" => {
                let (set_v, set_t) = self.codegen_expr(&args[0])?;
                let (elem_v, elem_t) = self.codegen_expr(&args[1])?;
                let elem = self.convert_to_i64(&elem_v, &elem_t)?;
                let rt = if func == "set_add" { "runtime_set_add" } else { "runtime_set_remove" };
                let slot = self.fresh_temp();
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeSet, align 8\n", slot));
                self.out.push_str(&format!("  store %LimeSet {}, ptr {}, align 8\n", self.bare_value(&set_v), slot));
                self.out.push_str(&format!(
                    "  call void @{}(ptr sret(%LimeSet) {}, ptr {}, i64 {})\n",
                    rt, slot, slot, elem
                ));
                self.out.push_str(&format!("  {} = load %LimeSet, ptr {}, align 8\n", tmp, slot));
                Ok(Some((tmp, set_t)))
            }
            "set_contains" => {
                let (set_v, _) = self.codegen_expr(&args[0])?;
                let (elem_v, elem_t) = self.codegen_expr(&args[1])?;
                let elem = self.convert_to_i64(&elem_v, &elem_t)?;
                let (v, t) = call_bool(self, "runtime_set_contains", &[self.bare_value(&set_v).to_string(), elem]);
                Ok(Some((v, t)))
            }
            "set_clear" | "set_clone" => {
                let (set_v, set_t) = self.codegen_expr(&args[0])?;
                let rt = if func == "set_clear" { "runtime_set_clear" } else { "runtime_set_clone" };
                let slot = self.fresh_temp();
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeSet, align 8\n", slot));
                self.out.push_str(&format!("  store %LimeSet {}, ptr {}, align 8\n", self.bare_value(&set_v), slot));
                self.out.push_str(&format!(
                    "  call void @{}(ptr sret(%LimeSet) {}, ptr {})\n",
                    rt, slot, slot
                ));
                self.out.push_str(&format!("  {} = load %LimeSet, ptr {}, align 8\n", tmp, slot));
                Ok(Some((tmp, set_t)))
            }
            // ---- queue builtins (Phase C-1.2) ----
            "queue_push" => {
                let (list_v, list_t) = self.codegen_expr(&args[0])?;
                let (elem_v, elem_t) = self.codegen_expr(&args[1])?;
                let elem = self.convert_to_i64(&elem_v, &elem_t)?;
                let slot = self.fresh_temp();
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", slot));
                self.out.push_str(&format!("  store %LimeList {}, ptr {}, align 8\n", self.bare_value(&list_v), slot));
                self.out.push_str(&format!(
                    "  call void @runtime_queue_push(ptr sret(%LimeList) {}, ptr {}, i64 {})\n",
                    slot, slot, elem
                ));
                self.out.push_str(&format!("  {} = load %LimeList, ptr {}, align 8\n", tmp, slot));
                Ok(Some((tmp, list_t)))
            }
            "queue_pop" | "queue_front" | "queue_back" => {
                let (list_v, _) = self.codegen_expr(&args[0])?;
                let rt = match func {
                    "queue_pop" => "runtime_queue_pop",
                    "queue_front" => "runtime_queue_front",
                    "queue_back" => "runtime_queue_back",
                    _ => unreachable!(),
                };
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i64 @{}({})\n",
                    tmp, rt, self.bare_value(&list_v)
                ));
                Ok(Some((tmp, Type::Int)))
            }
            "queue_len" | "queue_size" => {
                let (list_v, _) = self.codegen_expr(&args[0])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i64 @runtime_queue_len({})\n",
                    tmp, self.bare_value(&list_v)
                ));
                Ok(Some((tmp, Type::Int)))
            }
            "queue_is_empty" => {
                let (list_v, _) = self.codegen_expr(&args[0])?;
                let (v, t) = call_bool(self, "runtime_queue_is_empty", &[self.bare_value(&list_v).to_string()]);
                Ok(Some((v, t)))
            }
            "queue_clear" => {
                let (list_v, list_t) = self.codegen_expr(&args[0])?;
                let slot = self.fresh_temp();
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", slot));
                self.out.push_str(&format!("  store %LimeList {}, ptr {}, align 8\n", self.bare_value(&list_v), slot));
                self.out.push_str(&format!(
                    "  call void @runtime_queue_clear(ptr sret(%LimeList) {}, ptr {})\n",
                    slot, slot
                ));
                self.out.push_str(&format!("  {} = load %LimeList, ptr {}, align 8\n", tmp, slot));
                Ok(Some((tmp, list_t)))
            }
            "queue_empty" => {
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", tmp));
                self.out.push_str(&format!("  call void @runtime_list_empty(ptr {})\n", tmp));
                Ok(Some((tmp, Type::List(Box::new(Type::Unknown)))))
            }
            // ---- stack builtins (Phase C-1.2) ----
            "stack_push" => {
                let (list_v, list_t) = self.codegen_expr(&args[0])?;
                let (elem_v, elem_t) = self.codegen_expr(&args[1])?;
                let elem = self.convert_to_i64(&elem_v, &elem_t)?;
                let slot = self.fresh_temp();
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", slot));
                self.out.push_str(&format!("  store %LimeList {}, ptr {}, align 8\n", self.bare_value(&list_v), slot));
                self.out.push_str(&format!(
                    "  call void @runtime_stack_push(ptr sret(%LimeList) {}, ptr {}, i64 {})\n",
                    slot, slot, elem
                ));
                self.out.push_str(&format!("  {} = load %LimeList, ptr {}, align 8\n", tmp, slot));
                Ok(Some((tmp, list_t)))
            }
            "stack_pop" | "stack_peek" => {
                let (list_v, _) = self.codegen_expr(&args[0])?;
                let rt = if func == "stack_pop" { "runtime_stack_pop" } else { "runtime_stack_peek" };
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = call i64 @{}({})\n", tmp, rt, self.bare_value(&list_v)));
                Ok(Some((tmp, Type::Int)))
            }
            "stack_len" | "stack_size" => {
                let (list_v, _) = self.codegen_expr(&args[0])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = call i64 @runtime_stack_len({})\n", tmp, self.bare_value(&list_v)));
                Ok(Some((tmp, Type::Int)))
            }
            "stack_is_empty" => {
                let (list_v, _) = self.codegen_expr(&args[0])?;
                let (v, t) = call_bool(self, "runtime_stack_is_empty", &[self.bare_value(&list_v).to_string()]);
                Ok(Some((v, t)))
            }
            "stack_clear" => {
                let (list_v, list_t) = self.codegen_expr(&args[0])?;
                let slot = self.fresh_temp();
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", slot));
                self.out.push_str(&format!("  store %LimeList {}, ptr {}, align 8\n", self.bare_value(&list_v), slot));
                self.out.push_str(&format!(
                    "  call void @runtime_stack_clear(ptr sret(%LimeList) {}, ptr {})\n",
                    slot, slot
                ));
                self.out.push_str(&format!("  {} = load %LimeList, ptr {}, align 8\n", tmp, slot));
                Ok(Some((tmp, list_t)))
            }
            "stack_empty" => {
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", tmp));
                self.out.push_str(&format!("  call void @runtime_list_empty(ptr {})\n", tmp));
                Ok(Some((tmp, Type::List(Box::new(Type::Unknown)))))
            }
            // ---- JSON builtins ----
            "json_parse" => {
                let a = str_arg(self, &args[0])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = call i8* @runtime_json_parse(i8* {})\n", tmp, a));
                Ok(Some((tmp, Type::Json)))
            }
            "json_stringify" => {
                let a = arg(self, &args[0], &Type::Json)?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = call i8* @runtime_json_stringify(i8* {})\n", tmp, a));
                Ok(Some((tmp, Type::String)))
            }
            "json_get" => {
                let j = arg(self, &args[0], &Type::Json)?;
                let k = str_arg(self, &args[1])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = call i8* @runtime_json_get(i8* {}, i8* {})\n", tmp, j, k));
                // Wrap as Option(Json): check if result is null
                let is_null = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i8* {}, null\n", is_null, tmp));
                let slot = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeOption, align 8\n", slot));
                // Store has_value
                let has_val_ptr = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr %LimeOption, %LimeOption* {}, i32 0, i32 0\n", has_val_ptr, slot));
                let i1_flag = self.fresh_temp();
                self.out.push_str(&format!("  {} = xor i1 {}, true\n", i1_flag, is_null));
                self.out.push_str(&format!("  store i1 {}, i1* {}\n", i1_flag, has_val_ptr));
                // Store value
                let val_ptr = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr %LimeOption, %LimeOption* {}, i32 0, i32 1\n", val_ptr, slot));
                self.out.push_str(&format!("  store i8* {}, i8** {}\n", tmp, val_ptr));
                let loaded = self.fresh_temp();
                self.out.push_str(&format!("  {} = load %LimeOption, %LimeOption* {}\n", loaded, slot));
                Ok(Some((loaded, Type::Option(Box::new(Type::Json)))))
            }
            "json_has" => {
                let j = arg(self, &args[0], &Type::Json)?;
                let k = str_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_json_has", &[j, k]);
                Ok(Some((v, t)))
            }
            "json_len" => {
                let j = arg(self, &args[0], &Type::Json)?;
                let (v, t) = call_i64(self, "runtime_json_len", &[j]);
                Ok(Some((v, t)))
            }
            "json_at" => {
                let j = arg(self, &args[0], &Type::Json)?;
                let i = i64_arg(self, &args[1])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = call i8* @runtime_json_at(i8* {}, i64 {})\n", tmp, j, i));
                Ok(Some((tmp, Type::Json)))
            }
            "json_as_string" => {
                let j = arg(self, &args[0], &Type::Json)?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = call i8* @runtime_json_as_string(i8* {})\n", tmp, j));
                Ok(Some((tmp, Type::String)))
            }
            "json_as_int" => {
                let j = arg(self, &args[0], &Type::Json)?;
                let (v, t) = call_i64(self, "runtime_json_as_int", &[j]);
                Ok(Some((v, t)))
            }
            "json_as_float" => {
                let j = arg(self, &args[0], &Type::Json)?;
                let (v, t) = call_f64(self, "runtime_json_as_float", &[j]);
                Ok(Some((v, t)))
            }
            "json_as_bool" => {
                let j = arg(self, &args[0], &Type::Json)?;
                let (v, t) = call_bool(self, "runtime_json_as_bool", &[j]);
                Ok(Some((v, t)))
            }
            "json_null" => {
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = call i8* @runtime_json_null()\n", tmp));
                Ok(Some((tmp, Type::Json)))
            }
            "json_object" => {
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = call i8* @runtime_json_object()\n", tmp));
                Ok(Some((tmp, Type::Json)))
            }
            "json_array" => {
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = call i8* @runtime_json_array()\n", tmp));
                Ok(Some((tmp, Type::Json)))
            }
            "json_set" => {
                let j = arg(self, &args[0], &Type::Json)?;
                let k = str_arg(self, &args[1])?;
                let v = arg(self, &args[2], &Type::Json)?;
                let (r, t) = call_bool(self, "runtime_json_set", &[j, k, v]);
                Ok(Some((r, t)))
            }
            "json_push" => {
                let j = arg(self, &args[0], &Type::Json)?;
                let e = arg(self, &args[1], &Type::Json)?;
                let (r, t) = call_bool(self, "runtime_json_push", &[j, e]);
                Ok(Some((r, t)))
            }
            // ===== Option builtins =====
            "option_some" => {
                let v = any_arg(self, &args[0])?;
                let slot = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %Option, align 8\n", slot));
                let tag_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 0\n", tag_gep, slot));
                self.out.push_str(&format!("  store i32 0, ptr {}, align 4\n", tag_gep));
                let payload_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 1, i32 0\n", payload_gep, slot));
                let converted = self.convert_to_i64(&v, &Type::Unknown)?;
                self.out.push_str(&format!("  store i64 {}, ptr {}, align 8\n", converted, payload_gep));
                let loaded = self.fresh_temp();
                self.out.push_str(&format!("  {} = load %Option, ptr {}, align 8\n", loaded, slot));
                Ok(Some((loaded, Type::Option(Box::new(Type::Unknown)))))
            }
            "option_none" => {
                let slot = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %Option, align 8\n", slot));
                let tag_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 0\n", tag_gep, slot));
                self.out.push_str(&format!("  store i32 1, ptr {}, align 4\n", tag_gep));
                let payload_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 1, i32 0\n", payload_gep, slot));
                self.out.push_str(&format!("  store i64 0, ptr {}, align 8\n", payload_gep));
                let loaded = self.fresh_temp();
                self.out.push_str(&format!("  {} = load %Option, ptr {}, align 8\n", loaded, slot));
                Ok(Some((loaded, Type::Option(Box::new(Type::Unknown)))))
            }
            "option_is_some" => {
                let v = any_arg(self, &args[0])?;
                let tag = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 0\n", tag, self.bare_value(&v)));
                let cmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i32 {}, 0\n", cmp, tag));
                Ok(Some((cmp, Type::Bool)))
            }
            "option_is_none" => {
                let v = any_arg(self, &args[0])?;
                let tag = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 0\n", tag, self.bare_value(&v)));
                let cmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i32 {}, 1\n", cmp, tag));
                Ok(Some((cmp, Type::Bool)))
            }
            "option_extract" => {
                let v = any_arg(self, &args[0])?;
                let tag = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 0\n", tag, self.bare_value(&v)));
                let is_none = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp ne i32 {}, 0\n", is_none, tag));
                let panic_bb = self.fresh_block();
                let ok_bb = self.fresh_block();
                self.out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", is_none, panic_bb, ok_bb));
                self.out.push_str(&format!("{}:\n", panic_bb));
                let msg = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds [31 x i8], ptr @.str.panic_msg, i64 0, i64 0\n", msg));
                self.out.push_str(&format!("  call void @runtime_panic(i8* {})\n", msg));
                self.out.push_str("  unreachable\n");
                self.out.push_str(&format!("{}:\n", ok_bb));
                let payload = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 1, 0\n", payload, self.bare_value(&v)));
                Ok(Some((payload, Type::Unknown)))
            }
            "option_extract_or" => {
                let v = any_arg(self, &args[0])?;
                let default = any_arg(self, &args[1])?;
                let tag = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 0\n", tag, self.bare_value(&v)));
                let is_some = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i32 {}, 0\n", is_some, tag));
                let payload = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 1, 0\n", payload, self.bare_value(&v)));
                let result = self.fresh_temp();
                self.out.push_str(&format!("  {} = select i1 {}, i64 {}, i64 {}\n", result, is_some, payload, self.bare_value(&default)));
                Ok(Some((result, Type::Int)))
            }
            "option_and" => {
                let a = any_arg(self, &args[0])?;
                let tag_a = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 0\n", tag_a, self.bare_value(&a)));
                let is_some = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i32 {}, 0\n", is_some, tag_a));
                let b_val = any_arg(self, &args[1])?;
                let b_tag = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 0\n", b_tag, self.bare_value(&b_val)));
                let b_payload = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 1, 0\n", b_payload, self.bare_value(&b_val)));
                let res_slot = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %Option, align 8\n", res_slot));
                let res_tag_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 0\n", res_tag_gep, res_slot));
                let chosen_tag = self.fresh_temp();
                self.out.push_str(&format!("  {} = select i1 {}, i32 {}, i32 1\n", chosen_tag, is_some, b_tag));
                self.out.push_str(&format!("  store i32 {}, ptr {}, align 4\n", chosen_tag, res_tag_gep));
                let res_payload_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 1, i32 0\n", res_payload_gep, res_slot));
                let chosen_payload = self.fresh_temp();
                self.out.push_str(&format!("  {} = select i1 {}, i64 {}, i64 0\n", chosen_payload, is_some, b_payload));
                self.out.push_str(&format!("  store i64 {}, ptr {}, align 8\n", chosen_payload, res_payload_gep));
                let loaded = self.fresh_temp();
                self.out.push_str(&format!("  {} = load %Option, ptr {}, align 8\n", loaded, res_slot));
                Ok(Some((loaded, Type::Option(Box::new(Type::Unknown)))))
            }
            "option_or" => {
                let a = any_arg(self, &args[0])?;
                let tag_a = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 0\n", tag_a, self.bare_value(&a)));
                let is_some = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i32 {}, 0\n", is_some, tag_a));
                let payload_a = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 1, 0\n", payload_a, self.bare_value(&a)));
                let b_val = any_arg(self, &args[1])?;
                let b_payload = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 1, 0\n", b_payload, self.bare_value(&b_val)));
                let res_slot = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %Option, align 8\n", res_slot));
                let res_tag_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 0\n", res_tag_gep, res_slot));
                let chosen_tag = self.fresh_temp();
                self.out.push_str(&format!("  {} = select i1 {}, i32 0, i32 1\n", chosen_tag, is_some));
                self.out.push_str(&format!("  store i32 {}, ptr {}, align 4\n", chosen_tag, res_tag_gep));
                let res_payload_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 1, i32 0\n", res_payload_gep, res_slot));
                let chosen_payload = self.fresh_temp();
                self.out.push_str(&format!("  {} = select i1 {}, i64 {}, i64 {}\n", chosen_payload, is_some, payload_a, self.bare_value(&b_payload)));
                self.out.push_str(&format!("  store i64 {}, ptr {}, align 8\n", chosen_payload, res_payload_gep));
                let loaded = self.fresh_temp();
                self.out.push_str(&format!("  {} = load %Option, ptr {}, align 8\n", loaded, res_slot));
                Ok(Some((loaded, Type::Option(Box::new(Type::Unknown)))))
            }
            "option_equals" => {
                let a = any_arg(self, &args[0])?;
                let b = any_arg(self, &args[1])?;
                let tag_a = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 0\n", tag_a, self.bare_value(&a)));
                let tag_b = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 0\n", tag_b, self.bare_value(&b)));
                let tags_eq = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i32 {}, {}\n", tags_eq, tag_a, tag_b));
                let both_none = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i32 {}, 1\n", both_none, tag_a));
                let both_none_and_eq = self.fresh_temp();
                self.out.push_str(&format!("  {} = and i1 {}, {}\n", both_none_and_eq, tags_eq, both_none));
                let payload_a = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 1, 0\n", payload_a, self.bare_value(&a)));
                let payload_b = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Option {}, 1, 0\n", payload_b, self.bare_value(&b)));
                let payloads_eq = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i64 {}, {}\n", payloads_eq, payload_a, payload_b));
                let both_some = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i32 {}, 0\n", both_some, tag_a));
                let both_some_and_eq = self.fresh_temp();
                self.out.push_str(&format!("  {} = and i1 {}, {}\n", both_some_and_eq, tags_eq, both_some));
                let payload_check = self.fresh_temp();
                self.out.push_str(&format!("  {} = and i1 {}, {}\n", payload_check, both_some_and_eq, payloads_eq));
                let result = self.fresh_temp();
                self.out.push_str(&format!("  {} = or i1 {}, {}\n", result, both_none_and_eq, payload_check));
                Ok(Some((result, Type::Bool)))
            }
            // ===== Result builtins =====
            "result_success" => {
                let v = any_arg(self, &args[0])?;
                let slot = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %Result, align 8\n", slot));
                let tag_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Result, ptr {}, i64 0, i32 0\n", tag_gep, slot));
                self.out.push_str(&format!("  store i32 0, ptr {}, align 4\n", tag_gep));
                let payload_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Result, ptr {}, i64 0, i32 1, i32 0\n", payload_gep, slot));
                let converted = self.convert_to_i64(&v, &Type::Unknown)?;
                self.out.push_str(&format!("  store i64 {}, ptr {}, align 8\n", converted, payload_gep));
                let loaded = self.fresh_temp();
                self.out.push_str(&format!("  {} = load %Result, ptr {}, align 8\n", loaded, slot));
                Ok(Some((loaded, Type::State("Result(unknown,unknown)".to_string()))))
            }
            "result_error" => {
                let v = any_arg(self, &args[0])?;
                let slot = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %Result, align 8\n", slot));
                let tag_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Result, ptr {}, i64 0, i32 0\n", tag_gep, slot));
                self.out.push_str(&format!("  store i32 1, ptr {}, align 4\n", tag_gep));
                let payload_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Result, ptr {}, i64 0, i32 1, i32 0\n", payload_gep, slot));
                let converted = self.convert_to_i64(&v, &Type::Unknown)?;
                self.out.push_str(&format!("  store i64 {}, ptr {}, align 8\n", converted, payload_gep));
                let loaded = self.fresh_temp();
                self.out.push_str(&format!("  {} = load %Result, ptr {}, align 8\n", loaded, slot));
                Ok(Some((loaded, Type::State("Result(unknown,unknown)".to_string()))))
            }
            "result_is_success" => {
                let v = any_arg(self, &args[0])?;
                let tag = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 0\n", tag, self.bare_value(&v)));
                let cmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i32 {}, 0\n", cmp, tag));
                Ok(Some((cmp, Type::Bool)))
            }
            "result_is_error" => {
                let v = any_arg(self, &args[0])?;
                let tag = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 0\n", tag, self.bare_value(&v)));
                let cmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i32 {}, 1\n", cmp, tag));
                Ok(Some((cmp, Type::Bool)))
            }
            "result_extract" => {
                let v = any_arg(self, &args[0])?;
                let tag = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 0\n", tag, self.bare_value(&v)));
                let is_error = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp ne i32 {}, 0\n", is_error, tag));
                let panic_bb = self.fresh_block();
                let ok_bb = self.fresh_block();
                self.out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", is_error, panic_bb, ok_bb));
                self.out.push_str(&format!("{}:\n", panic_bb));
                let msg = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds [31 x i8], ptr @.str.panic_msg, i64 0, i64 0\n", msg));
                self.out.push_str(&format!("  call void @runtime_panic(i8* {})\n", msg));
                self.out.push_str("  unreachable\n");
                self.out.push_str(&format!("{}:\n", ok_bb));
                let payload = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 1, 0\n", payload, self.bare_value(&v)));
                Ok(Some((payload, Type::Unknown)))
            }
            "result_extract_or" => {
                let v = any_arg(self, &args[0])?;
                let default = any_arg(self, &args[1])?;
                let tag = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 0\n", tag, self.bare_value(&v)));
                let is_success = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i32 {}, 0\n", is_success, tag));
                let payload = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 1, 0\n", payload, self.bare_value(&v)));
                let result = self.fresh_temp();
                self.out.push_str(&format!("  {} = select i1 {}, i64 {}, i64 {}\n", result, is_success, payload, self.bare_value(&default)));
                Ok(Some((result, Type::Int)))
            }
            "result_and" => {
                let a = any_arg(self, &args[0])?;
                let tag_a = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 0\n", tag_a, self.bare_value(&a)));
                let is_success = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i32 {}, 0\n", is_success, tag_a));
                let b_val = any_arg(self, &args[1])?;
                let b_tag = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 0\n", b_tag, self.bare_value(&b_val)));
                let b_payload = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 1, 0\n", b_payload, self.bare_value(&b_val)));
                let a_payload = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 1, 0\n", a_payload, self.bare_value(&a)));
                let res_slot = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %Result, align 8\n", res_slot));
                let res_tag_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Result, ptr {}, i64 0, i32 0\n", res_tag_gep, res_slot));
                let chosen_tag = self.fresh_temp();
                self.out.push_str(&format!("  {} = select i1 {}, i32 {}, i32 {}\n", chosen_tag, is_success, b_tag, tag_a));
                self.out.push_str(&format!("  store i32 {}, ptr {}, align 4\n", chosen_tag, res_tag_gep));
                let res_payload_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Result, ptr {}, i64 0, i32 1, i32 0\n", res_payload_gep, res_slot));
                let chosen_payload = self.fresh_temp();
                self.out.push_str(&format!("  {} = select i1 {}, i64 {}, i64 {}\n", chosen_payload, is_success, b_payload, a_payload));
                self.out.push_str(&format!("  store i64 {}, ptr {}, align 8\n", chosen_payload, res_payload_gep));
                let loaded = self.fresh_temp();
                self.out.push_str(&format!("  {} = load %Result, ptr {}, align 8\n", loaded, res_slot));
                Ok(Some((loaded, Type::State("Result(unknown,unknown)".to_string()))))
            }
            "result_or" => {
                let a = any_arg(self, &args[0])?;
                let tag_a = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 0\n", tag_a, self.bare_value(&a)));
                let is_success = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i32 {}, 0\n", is_success, tag_a));
                let payload_a = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 1, 0\n", payload_a, self.bare_value(&a)));
                let b_val = any_arg(self, &args[1])?;
                let b_payload = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 1, 0\n", b_payload, self.bare_value(&b_val)));
                let res_slot = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %Result, align 8\n", res_slot));
                let res_tag_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Result, ptr {}, i64 0, i32 0\n", res_tag_gep, res_slot));
                let chosen_tag = self.fresh_temp();
                self.out.push_str(&format!("  {} = select i1 {}, i32 0, i32 1\n", chosen_tag, is_success));
                self.out.push_str(&format!("  store i32 {}, ptr {}, align 4\n", chosen_tag, res_tag_gep));
                let res_payload_gep = self.fresh_temp();
                self.out.push_str(&format!("  {} = getelementptr inbounds %Result, ptr {}, i64 0, i32 1, i32 0\n", res_payload_gep, res_slot));
                let chosen_payload = self.fresh_temp();
                self.out.push_str(&format!("  {} = select i1 {}, i64 {}, i64 {}\n", chosen_payload, is_success, payload_a, self.bare_value(&b_payload)));
                self.out.push_str(&format!("  store i64 {}, ptr {}, align 8\n", chosen_payload, res_payload_gep));
                let loaded = self.fresh_temp();
                self.out.push_str(&format!("  {} = load %Result, ptr {}, align 8\n", loaded, res_slot));
                Ok(Some((loaded, Type::State("Result(unknown,unknown)".to_string()))))
            }
            "result_equals" => {
                let a = any_arg(self, &args[0])?;
                let b = any_arg(self, &args[1])?;
                let tag_a = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 0\n", tag_a, self.bare_value(&a)));
                let tag_b = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 0\n", tag_b, self.bare_value(&b)));
                let tags_eq = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i32 {}, {}\n", tags_eq, tag_a, tag_b));
                let payload_a = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 1, 0\n", payload_a, self.bare_value(&a)));
                let payload_b = self.fresh_temp();
                self.out.push_str(&format!("  {} = extractvalue %Result {}, 1, 0\n", payload_b, self.bare_value(&b)));
                let payloads_eq = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp eq i64 {}, {}\n", payloads_eq, payload_a, payload_b));
                let result = self.fresh_temp();
                self.out.push_str(&format!("  {} = and i1 {}, {}\n", result, tags_eq, payloads_eq));
                Ok(Some((result, Type::Bool)))
            }
            // ===== Path operations (Phase C-1.8) =====
            "path_join" => {
                let a = str_arg(self, &args[0])?;
                let b = str_arg(self, &args[1])?;
                let call = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_path_join(i8* {}, i8* {})\n", call, a, b
                ));
                Ok(Some((call, Type::String)))
            }
            "path_basename" => {
                let a = str_arg(self, &args[0])?;
                let call = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_path_basename(i8* {})\n", call, a
                ));
                Ok(Some((call, Type::String)))
            }
            "path_dirname" => {
                let a = str_arg(self, &args[0])?;
                let call = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_path_dirname(i8* {})\n", call, a
                ));
                Ok(Some((call, Type::String)))
            }
            "path_filename" => {
                let a = str_arg(self, &args[0])?;
                let call = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_path_filename(i8* {})\n", call, a
                ));
                Ok(Some((call, Type::String)))
            }
            "path_extension" => {
                let a = str_arg(self, &args[0])?;
                let call = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_path_extension(i8* {})\n", call, a
                ));
                Ok(Some((call, Type::String)))
            }
            "path_is_absolute" => {
                let a = str_arg(self, &args[0])?;
                let call = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i32 @runtime_path_is_absolute(i8* {})\n", call, a
                ));
                Ok(Some((call, Type::Bool)))
            }
            "path_normalize" => {
                let a = str_arg(self, &args[0])?;
                let call = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_path_normalize(i8* {})\n", call, a
                ));
                Ok(Some((call, Type::String)))
            }
            "path_equals" => {
                let a = str_arg(self, &args[0])?;
                let b = str_arg(self, &args[1])?;
                let call = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i32 @runtime_path_equals(i8* {}, i8* {})\n", call, a, b
                ));
                Ok(Some((call, Type::Bool)))
            }
            "path_parent" => {
                let a = str_arg(self, &args[0])?;
                let call = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_path_parent(i8* {})\n", call, a
                ));
                Ok(Some((call, Type::String)))
            }
            // ===== OS operations (Phase C-1.9) =====
            "os_name" => {
                let (v, t) = call_str(self, "runtime_os_name", &[]);
                Ok(Some((v, t)))
            }
            "os_arch" => {
                let (v, t) = call_str(self, "runtime_os_arch", &[]);
                Ok(Some((v, t)))
            }
            "os_platform" => {
                let (v, t) = call_str(self, "runtime_os_platform", &[]);
                Ok(Some((v, t)))
            }
            "os_hostname" => {
                let (v, t) = call_str(self, "runtime_os_hostname", &[]);
                Ok(Some((v, t)))
            }
            "os_cwd" => {
                let (v, t) = call_str(self, "runtime_os_cwd", &[]);
                Ok(Some((v, t)))
            }
            "os_set_cwd" => {
                let a = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_os_set_cwd", &[a]);
                Ok(Some((v, t)))
            }
            // ===== ENV operations (Phase C-1.9) =====
            "env_get" => {
                let key = str_arg(self, &args[0])?;
                let ptr_val = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_env_get(i8* {})\n", ptr_val, key
                ));
                // Check if NULL -> None, else -> Some(value)
                let is_null = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = icmp eq i8* {}, null\n", is_null, ptr_val
                ));
                let slot = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = alloca %Option, align 8\n", slot
                ));
                let tag_gep = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 0\n", tag_gep, slot
                ));
                // tag = 1 if null (None), 0 if not null (Some)
                let zext_temp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = zext i1 {} to i32\n", zext_temp, is_null
                ));
                // Use select: if is_null then 1 else 0
                let tag_val = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = select i1 {}, i32 1, i32 0\n", tag_val, is_null
                ));
                self.out.push_str(&format!(
                    "  store i32 {}, ptr {}, align 4\n", tag_val, tag_gep
                ));
                let payload_gep = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 1, i32 0\n", payload_gep, slot
                ));
                // If null, store 0; otherwise store ptrtoint of the pointer
                let ptr_as_i64 = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = ptrtoint i8* {} to i64\n", ptr_as_i64, ptr_val
                ));
                self.out.push_str(&format!(
                    "  store i64 {}, ptr {}, align 8\n", ptr_as_i64, payload_gep
                ));
                let loaded = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = load %Option, ptr {}, align 8\n", loaded, slot
                ));
                Ok(Some((loaded, Type::Option(Box::new(Type::String)))))
            }
            "env_has" => {
                let key = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_env_has", &[key]);
                Ok(Some((v, t)))
            }
            "env_set" => {
                let key = str_arg(self, &args[0])?;
                let val = str_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_env_set", &[key, val]);
                Ok(Some((v, t)))
            }
            "env_remove" => {
                let key = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_env_remove", &[key]);
                Ok(Some((v, t)))
            }
            "env_all" => {
                let slot = self.fresh_temp();
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = alloca %LimeMap, align 8\n", slot
                ));
                self.out.push_str(&format!(
                    "  call void @runtime_env_all(ptr sret(%LimeMap) {})\n", slot
                ));
                self.out.push_str(&format!(
                    "  {} = load %LimeMap, ptr {}, align 8\n", tmp, slot
                ));
                Ok(Some((tmp, Type::Unknown)))
            }
            // ===== Regex operations (Phase C-1.10) =====
            "regex_compile" => {
                let pat = str_arg(self, &args[0])?;
                let ptr_val = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_regex_compile(i8* {})\n", ptr_val, pat
                ));
                let is_null = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = icmp eq i8* {}, null\n", is_null, ptr_val
                ));
                let slot = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = alloca %Option, align 8\n", slot
                ));
                let tag_gep = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 0\n", tag_gep, slot
                ));
                let zext_temp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = zext i1 {} to i32\n", zext_temp, is_null
                ));
                let tag_val = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = select i1 {}, i32 1, i32 0\n", tag_val, is_null
                ));
                self.out.push_str(&format!(
                    "  store i32 {}, ptr {}, align 4\n", tag_val, tag_gep
                ));
                let payload_gep = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 1, i32 0\n", payload_gep, slot
                ));
                let ptr_as_i64 = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = ptrtoint i8* {} to i64\n", ptr_as_i64, ptr_val
                ));
                self.out.push_str(&format!(
                    "  store i64 {}, ptr {}, align 8\n", ptr_as_i64, payload_gep
                ));
                let loaded = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = load %Option, ptr {}, align 8\n", loaded, slot
                ));
                Ok(Some((loaded, Type::Option(Box::new(Type::String)))))
            }
            "regex_is_match" => {
                let pat = str_arg(self, &args[0])?;
                let text = str_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_regex_is_match", &[pat, text]);
                Ok(Some((v, t)))
            }
            "regex_match" | "regex_find" => {
                let pat = str_arg(self, &args[0])?;
                let text = str_arg(self, &args[1])?;
                let ptr_val = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_regex_find(i8* {}, i8* {})\n", ptr_val, pat, text
                ));
                let is_null = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = icmp eq i8* {}, null\n", is_null, ptr_val
                ));
                let slot = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = alloca %Option, align 8\n", slot
                ));
                let tag_gep = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 0\n", tag_gep, slot
                ));
                let tag_val = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = select i1 {}, i32 1, i32 0\n", tag_val, is_null
                ));
                self.out.push_str(&format!(
                    "  store i32 {}, ptr {}, align 4\n", tag_val, tag_gep
                ));
                let payload_gep = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 1, i32 0\n", payload_gep, slot
                ));
                let ptr_as_i64 = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = ptrtoint i8* {} to i64\n", ptr_as_i64, ptr_val
                ));
                self.out.push_str(&format!(
                    "  store i64 {}, ptr {}, align 8\n", ptr_as_i64, payload_gep
                ));
                let loaded = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = load %Option, ptr {}, align 8\n", loaded, slot
                ));
                Ok(Some((loaded, Type::Option(Box::new(Type::String)))))
            }
            "regex_find_all" => {
                let pat = str_arg(self, &args[0])?;
                let text = str_arg(self, &args[1])?;
                let (v, t) = call_list(self, "runtime_regex_find_all", &[pat, text]);
                Ok(Some((v, t)))
            }
            "regex_replace" => {
                let pat = str_arg(self, &args[0])?;
                let text = str_arg(self, &args[1])?;
                let repl = str_arg(self, &args[2])?;
                let (v, t) = call_str(self, "runtime_regex_replace", &[pat, text, repl]);
                Ok(Some((v, t)))
            }
            "regex_replace_all" => {
                let pat = str_arg(self, &args[0])?;
                let text = str_arg(self, &args[1])?;
                let repl = str_arg(self, &args[2])?;
                let (v, t) = call_str(self, "runtime_regex_replace_all", &[pat, text, repl]);
                Ok(Some((v, t)))
            }
            "regex_split" => {
                let pat = str_arg(self, &args[0])?;
                let text = str_arg(self, &args[1])?;
                let (v, t) = call_list(self, "runtime_regex_split", &[pat, text]);
                Ok(Some((v, t)))
            }
            // ===== Process operations (Phase C-1.11) =====
            "process_spawn" => {
                let cmd = str_arg(self, &args[0])?;
                let arg_list = list_arg(self, &args[1])?;
                let (v, t) = call_i64(self, "runtime_process_spawn", &[cmd, arg_list]);
                Ok(Some((v, t)))
            }
            "process_run" => {
                let cmd = str_arg(self, &args[0])?;
                let arg_list = list_arg(self, &args[1])?;
                let (v, t) = call_str(self, "runtime_process_run", &[cmd, arg_list]);
                Ok(Some((v, t)))
            }
            "process_output" => {
                let cmd = str_arg(self, &args[0])?;
                let arg_list = list_arg(self, &args[1])?;
                let (v, t) = call_str(self, "runtime_process_output", &[cmd, arg_list]);
                Ok(Some((v, t)))
            }
            "process_wait" => {
                let pid = i64_arg(self, &args[0])?;
                let (v, t) = call_i64(self, "runtime_process_wait", &[pid]);
                Ok(Some((v, t)))
            }
            "process_kill" => {
                let pid = i64_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_process_kill", &[pid]);
                Ok(Some((v, t)))
            }
            "process_status" => {
                let pid = i64_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_process_status", &[pid]);
                Ok(Some((v, t)))
            }
            "process_args" => {
                let (v, t) = call_list(self, "runtime_process_args", &[]);
                Ok(Some((v, t)))
            }
            // ===== Requests operations (Phase C-1.12) =====
            "requests_client_new" => {
                let (v, t) = call_str(self, "runtime_requests_client_new", &[]);
                Ok(Some((v, t)))
            }
            "requests_client_builder_new" => {
                let (v, t) = call_str(self, "runtime_requests_client_builder_new", &[]);
                Ok(Some((v, t)))
            }
            "requests_client_builder_build" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_requests_client_builder_build", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_client_builder_default_headers" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  call void @runtime_requests_client_builder_default_headers(i8* {}, i8* {})\n", a0, a1
                ));
                Ok(Some((tmp, Type::Unknown)))
            }
            "requests_client_builder_timeout" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = i64_arg(self, &args[1])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  call void @runtime_requests_client_builder_timeout(i8* {}, i64 {})\n", a0, a1
                ));
                Ok(Some((tmp, Type::Unknown)))
            }
            "requests_client_builder_redirect_limit" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = i64_arg(self, &args[1])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  call void @runtime_requests_client_builder_redirect_limit(i8* {}, i64 {})\n", a0, a1
                ));
                Ok(Some((tmp, Type::Unknown)))
            }
            "requests_client_builder_redirect_disabled" => {
                let a0 = str_arg(self, &args[0])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  call void @runtime_requests_client_builder_redirect_disabled(i8* {})\n", a0
                ));
                Ok(Some((tmp, Type::Unknown)))
            }
            "requests_client_builder_proxy" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  call void @runtime_requests_client_builder_proxy(i8* {}, i8* {})\n", a0, a1
                ));
                Ok(Some((tmp, Type::Unknown)))
            }
            "requests_client_builder_tls_config" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  call void @runtime_requests_client_builder_tls_config(i8* {}, i8* {})\n", a0, a1
                ));
                Ok(Some((tmp, Type::Unknown)))
            }
            "requests_request_builder_new" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let a2 = str_arg(self, &args[2])?;
                let (v, t) = call_str(self, "runtime_requests_request_builder_new", &[a0, a1, a2]);
                Ok(Some((v, t)))
            }
            "requests_request_builder_header" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let a2 = str_arg(self, &args[2])?;
                let (v, t) = call_bool(self, "runtime_requests_request_builder_header", &[a0, a1, a2]);
                Ok(Some((v, t)))
            }
            "requests_request_builder_headers" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_requests_request_builder_headers", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_request_builder_query" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = list_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_requests_request_builder_query", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_request_builder_body_bytes" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let a1_len = i64_arg(self, &args[1])?; // placeholder
                let (v, t) = call_bool(self, "runtime_requests_request_builder_body_bytes", &[a0, a1, a1_len]);
                Ok(Some((v, t)))
            }
            "requests_request_builder_body_str" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_requests_request_builder_body_str", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_request_builder_json" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_requests_request_builder_json", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_request_builder_form" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = list_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_requests_request_builder_form", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_request_builder_multipart" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_requests_request_builder_multipart", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_request_builder_timeout" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = i64_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_requests_request_builder_timeout", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_request_builder_redirect_limit" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = i64_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_requests_request_builder_redirect_limit", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_request_builder_redirect_disabled" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_requests_request_builder_redirect_disabled", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_request_builder_basic_auth" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let a2 = str_arg(self, &args[2])?;
                let (v, t) = call_bool(self, "runtime_requests_request_builder_basic_auth", &[a0, a1, a2]);
                Ok(Some((v, t)))
            }
            "requests_request_builder_bearer_auth" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_requests_request_builder_bearer_auth", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_send" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_requests_send", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_response_status" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_i64(self, "runtime_requests_response_status", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_response_headers" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_requests_response_headers", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_response_url" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_requests_response_url", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_response_text" => {
                let a0 = str_arg(self, &args[0])?;
                let ptr_val = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_requests_response_text(i8* {})\n", ptr_val, a0
                ));
                let is_null = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = icmp eq i8* {}, null\n", is_null, ptr_val
                ));
                let slot = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = alloca %Option, align 8\n", slot
                ));
                let tag_gep = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 0\n", tag_gep, slot
                ));
                let tag_val = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = select i1 {}, i32 1, i32 0\n", tag_val, is_null
                ));
                self.out.push_str(&format!(
                    "  store i32 {}, ptr {}, align 4\n", tag_val, tag_gep
                ));
                let payload_gep = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 1, i32 0\n", payload_gep, slot
                ));
                let ptr_as_i64 = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = ptrtoint i8* {} to i64\n", ptr_as_i64, ptr_val
                ));
                self.out.push_str(&format!(
                    "  store i64 {}, ptr {}, align 8\n", ptr_as_i64, payload_gep
                ));
                let loaded = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = load %Option, ptr {}, align 8\n", loaded, slot
                ));
                Ok(Some((loaded, Type::Option(Box::new(Type::String)))))
            }
            "requests_response_bytes" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_requests_response_bytes", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_response_json" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_requests_response_json", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_response_content_length" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_i64(self, "runtime_requests_response_content_length", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_response_is_success" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_requests_response_is_success", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_response_is_client_error" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_requests_response_is_client_error", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_response_is_server_error" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_requests_response_is_server_error", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_response_error_for_status" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_requests_response_error_for_status", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_status_code_code" => {
                let a0 = i64_arg(self, &args[0])?;
                let (v, t) = call_i64(self, "runtime_requests_status_code_code", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_status_code_is_success" => {
                let a0 = i64_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_requests_status_code_is_success", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_status_code_is_client_error" => {
                let a0 = i64_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_requests_status_code_is_client_error", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_status_code_is_server_error" => {
                let a0 = i64_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_requests_status_code_is_server_error", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_status_code_is_redirect" => {
                let a0 = i64_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_requests_status_code_is_redirect", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_header_map_new" => {
                let (v, t) = call_str(self, "runtime_requests_header_map_new", &[]);
                Ok(Some((v, t)))
            }
            "requests_header_map_insert" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let a2 = str_arg(self, &args[2])?;
                let (v, t) = call_bool(self, "runtime_requests_header_map_insert", &[a0, a1, a2]);
                Ok(Some((v, t)))
            }
            "requests_header_map_append" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let a2 = str_arg(self, &args[2])?;
                let (v, t) = call_bool(self, "runtime_requests_header_map_append", &[a0, a1, a2]);
                Ok(Some((v, t)))
            }
            "requests_header_map_remove" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let a2 = str_arg(self, &args[2])?;
                let (v, t) = call_bool(self, "runtime_requests_header_map_remove", &[a0, a1, a2]);
                Ok(Some((v, t)))
            }
            "requests_header_map_get" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let ptr_val = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_requests_header_map_get(i8* {}, i8* {})\n", ptr_val, a0, a1
                ));
                let is_null = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = icmp eq i8* {}, null\n", is_null, ptr_val
                ));
                let slot = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = alloca %Option, align 8\n", slot
                ));
                let tag_gep = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 0\n", tag_gep, slot
                ));
                let tag_val = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = select i1 {}, i32 1, i32 0\n", tag_val, is_null
                ));
                self.out.push_str(&format!(
                    "  store i32 {}, ptr {}, align 4\n", tag_val, tag_gep
                ));
                let payload_gep = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = getelementptr inbounds %Option, ptr {}, i64 0, i32 1, i32 0\n", payload_gep, slot
                ));
                let ptr_as_i64 = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = ptrtoint i8* {} to i64\n", ptr_as_i64, ptr_val
                ));
                self.out.push_str(&format!(
                    "  store i64 {}, ptr {}, align 8\n", ptr_as_i64, payload_gep
                ));
                let loaded = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = load %Option, ptr {}, align 8\n", loaded, slot
                ));
                Ok(Some((loaded, Type::Option(Box::new(Type::String)))))
            }
            "requests_header_map_contains" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_requests_header_map_contains", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_multipart_new" => {
                let (v, t) = call_str(self, "runtime_requests_multipart_new", &[]);
                Ok(Some((v, t)))
            }
            "requests_multipart_text" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let a2 = str_arg(self, &args[2])?;
                let (v, t) = call_bool(self, "runtime_requests_multipart_text", &[a0, a1, a2]);
                Ok(Some((v, t)))
            }
            "requests_multipart_file" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let a2 = str_arg(self, &args[2])?;
                let (v, t) = call_bool(self, "runtime_requests_multipart_file", &[a0, a1, a2]);
                Ok(Some((v, t)))
            }
            "requests_multipart_file_with_metadata" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let a2 = str_arg(self, &args[2])?;
                let a3 = str_arg(self, &args[3])?;
                let a4 = str_arg(self, &args[4])?;
                let (v, t) = call_bool(self, "runtime_requests_multipart_file_with_metadata", &[a0, a1, a2, a3, a4]);
                Ok(Some((v, t)))
            }
            "requests_tls_config_new" => {
                let (v, t) = call_str(self, "runtime_requests_tls_config_new", &[]);
                Ok(Some((v, t)))
            }
            "requests_tls_config_add_ca_cert" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_requests_tls_config_add_ca_cert", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_tls_config_add_client_cert" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let a2 = str_arg(self, &args[2])?;
                let (v, t) = call_bool(self, "runtime_requests_tls_config_add_client_cert", &[a0, a1, a2]);
                Ok(Some((v, t)))
            }
            "requests_tls_config_danger_accept_invalid_certs" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_requests_tls_config_danger_accept_invalid_certs", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_tls_config_danger_accept_invalid_hostnames" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_requests_tls_config_danger_accept_invalid_hostnames", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_cookie_jar_new" => {
                let (v, t) = call_str(self, "runtime_requests_cookie_jar_new", &[]);
                Ok(Some((v, t)))
            }
            "requests_cookie_jar_add" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let (v, t) = call_bool(self, "runtime_requests_cookie_jar_add", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_cookie_parse" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_requests_cookie_parse", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_response_copy_to" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let (v, t) = call_i64(self, "runtime_requests_response_copy_to", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_response_chunks" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = i64_arg(self, &args[1])?;
                let (v, t) = call_list(self, "runtime_requests_response_chunks", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_response_stream" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_str(self, "runtime_requests_response_stream", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_stream_read" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = i64_arg(self, &args[1])?;
                let (v, t) = call_str(self, "runtime_requests_stream_read", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_stream_has_more" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_bool(self, "runtime_requests_stream_has_more", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_client_free" | "requests_request_builder_free" | "requests_response_free"
            | "requests_header_map_free" | "requests_multipart_free"
            | "requests_tls_config_free" | "requests_cookie_jar_free" | "requests_stream_free" => {
                let a0 = str_arg(self, &args[0])?;
                let free_name = format!("runtime_{}", func);
                self.out.push_str(&format!(
                    "  call void @{}(i8* {})\n", free_name, a0
                ));
                let tmp = self.fresh_temp();
                Ok(Some((tmp, Type::Unit)))
            }
            // ===== Requests Session operations (Phase C-1.12) =====
            "requests_session_new" => {
                let (v, t) = call_str(self, "runtime_requests_session_new", &[]);
                Ok(Some((v, t)))
            }
            "requests_session_request" => {
                let a0 = str_arg(self, &args[0])?;  // session
                let a1 = str_arg(self, &args[1])?;  // method
                let a2 = str_arg(self, &args[2])?;  // url
                let (v, t) = call_str(self, "runtime_requests_session_request", &[a0, a1, a2]);
                Ok(Some((v, t)))
            }
            "requests_session_free" => {
                let a0 = str_arg(self, &args[0])?;
                self.out.push_str(&format!("  call void @runtime_requests_session_free(i8* {})\n", a0));
                Ok(Some((self.fresh_temp(), Type::Unit)))
            }
            "requests_request_builder_set_headers" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = list_arg(self, &args[1])?;
                let (v, t) = call_i32(self, "runtime_requests_request_builder_set_headers", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_request_builder_verify" => {
                let a0 = str_arg(self, &args[0])?;
                let (val, _ty) = self.codegen_expr(&args[1])?;
                let bool_val = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp ne i32 {}, 0\n", bool_val, val));
                let (v, t) = call_i32(self, "runtime_requests_request_builder_verify", &[a0, bool_val]);
                Ok(Some((v, t)))
            }
            "requests_response_headers_list" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_list(self, "runtime_requests_response_headers_list", &[a0]);
                Ok(Some((v, t)))
            }
            // Cookie jar extended operations
            "requests_cookie_jar_add_parsed" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let (v, t) = call_i32(self, "runtime_requests_cookie_jar_add_parsed", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_cookie_jar_update_from_response" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let a2 = str_arg(self, &args[2])?;
                self.out.push_str(&format!("  call void @runtime_requests_cookie_jar_update_from_response(i8* {}, i8* {}, i8* {})\n", a0, a1, a2));
                Ok(Some((self.fresh_temp(), Type::Unit)))
            }
            "requests_cookie_jar_get_cookie_header" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let (v, t) = call_str(self, "runtime_requests_cookie_jar_get_cookie_header", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_cookie_jar_get_all" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_list(self, "runtime_requests_cookie_jar_get_all", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_cookie_jar_get" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let (v, t) = call_str(self, "runtime_requests_cookie_jar_get", &[a0, a1]);
                Ok(Some((v, t)))
            }
            // Session setters
            "requests_session_set_default_headers" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = list_arg(self, &args[1])?;
                let (v, t) = call_i32(self, "runtime_requests_session_set_default_headers", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_session_set_default_params" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = list_arg(self, &args[1])?;
                let (v, t) = call_i32(self, "runtime_requests_session_set_default_params", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_session_set_timeout" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let (v, t) = call_i32(self, "runtime_requests_session_set_timeout", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_session_set_verify" => {
                let a0 = str_arg(self, &args[0])?;
                let (val, _ty) = self.codegen_expr(&args[1])?;
                let bool_val = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp ne i32 {}, 0\n", bool_val, val));
                let (v, t) = call_i32(self, "runtime_requests_session_set_verify", &[a0, bool_val]);
                Ok(Some((v, t)))
            }
            "requests_session_set_redirect_limit" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let (v, t) = call_i32(self, "runtime_requests_session_set_redirect_limit", &[a0, a1]);
                Ok(Some((v, t)))
            }
            "requests_session_set_disable_redirects" => {
                let a0 = str_arg(self, &args[0])?;
                let (val, _ty) = self.codegen_expr(&args[1])?;
                let bool_val = self.fresh_temp();
                self.out.push_str(&format!("  {} = icmp ne i32 {}, 0\n", bool_val, val));
                let (v, t) = call_i32(self, "runtime_requests_session_set_disable_redirects", &[a0, bool_val]);
                Ok(Some((v, t)))
            }
            "requests_session_cookies" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_list(self, "runtime_requests_session_cookies", &[a0]);
                Ok(Some((v, t)))
            }
            // Redirect history
            "requests_redirect_history_new" => {
                let (v, t) = call_str(self, "runtime_requests_redirect_history_new", &[]);
                Ok(Some((v, t)))
            }
            "requests_redirect_history_add" => {
                let a0 = str_arg(self, &args[0])?;
                let a1 = str_arg(self, &args[1])?;
                let a2 = str_arg(self, &args[2])?;
                let a3 = str_arg(self, &args[3])?;
                self.out.push_str(&format!("  call void @runtime_requests_redirect_history_add(i8* {}, i64 {}, i8* {}, i8* {})\n", a0, a1, a2, a3));
                Ok(Some((self.fresh_temp(), Type::Unit)))
            }
            "requests_redirect_history_list" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_list(self, "runtime_requests_redirect_history_list", &[a0]);
                Ok(Some((v, t)))
            }
            "requests_redirect_history_free" => {
                let a0 = str_arg(self, &args[0])?;
                self.out.push_str(&format!("  call void @runtime_requests_redirect_history_free(i8* {})\n", a0));
                Ok(Some((self.fresh_temp(), Type::Unit)))
            }
            "requests_response_redirect_history" => {
                let a0 = str_arg(self, &args[0])?;
                let (v, t) = call_list(self, "runtime_requests_response_redirect_history", &[a0]);
                Ok(Some((v, t)))
            }
            _ => Ok(None),
        }
    }

    /// Phase 3: struct constructor codegen (insertvalue chain)
    fn codegen_struct_ctor(
        &mut self,
        name: &str,
        sdef: &StructDef,
        args: &[Expr],
    ) -> Result<(String, Type), String> {
        if args.len() != sdef.fields.len() {
            return Err(format!(
                "{} expects {} field(s), got {}",
                name,
                sdef.fields.len(),
                args.len()
            ));
        }
        let struct_type = format!("%{}", name);
        let mut current = String::from("undef");
        for (i, (arg, (_, ftype))) in args.iter().zip(&sdef.fields).enumerate() {
            let (v, _) = self.codegen_expr(arg)?;
            let ft = type_from_str(ftype, self.defs);
            let llft = llvm_type_name(&ft);
            let val = if v.starts_with('%') {
                format!("{} {}", llft, v)
            } else {
                v.clone()
            };
            let tmp = self.fresh_temp();
            self.out.push_str(&format!(
                "  {} = insertvalue {} {}, {}, {}\n",
                tmp, struct_type, current, val, i
            ));
            current = tmp;
        }
        Ok((current, Type::Struct(name.to_string())))
    }

    /// Phase 3: field access codegen (extractvalue)
    fn codegen_field_access(&mut self, object: &Expr, field: &str) -> Result<(String, Type), String> {
        let (obj_v, obj_t) = self.codegen_expr(object)?;
        match obj_t {
            Type::Struct(ref sname) => {
                let sdef = self.defs.structs.get(sname).ok_or_else(|| {
                    format!("unknown struct '{}'", sname)
                })?;
                let field_idx = sdef.fields.iter().position(|(fn_, _)| fn_ == field).ok_or_else(|| {
                    format!("unknown field '{}' on struct '{}'", field, sname)
                })?;
                let field_type = type_from_str(&sdef.fields[field_idx].1, self.defs);
                let llvm_struct_type = format!("%{}", sname);
                let _llvm_field_type = llvm_type_name(&field_type);
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = extractvalue {} {}, {}\n",
                    tmp, llvm_struct_type, obj_v, field_idx
                ));
                Ok((tmp, field_type))
            }
            _ => Err(format!("Field access on non-struct value")),
        }
    }

    /// Phase 4: state variant constructor codegen (tagged union)
    fn codegen_state_ctor(
        &mut self,
        state_name: &str,
        variant: &str,
        args: &[Expr],
    ) -> Result<(String, Type), String> {
        let llvm_type = format!("%{}", state_name);
        let variants = self.defs.states.get(state_name).ok_or_else(|| {
            format!("unknown state '{}'", state_name)
        })?;
        let tag = variants.iter().position(|v| v == variant).ok_or_else(|| {
            format!("variant '{}' not in state '{}'", variant, state_name)
        })?;

        let tmp_alloca = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = alloca {}, align 8\n",
            tmp_alloca, llvm_type
        ));

        // Store tag
        let tag_gep = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = getelementptr inbounds {}, ptr {}, i64 0, i32 0\n",
            tag_gep, llvm_type, tmp_alloca
        ));
        self.out.push_str(&format!(
            "  store i32 {}, i32* {}, align 4\n",
            tag as i32, tag_gep
        ));

        // Store payload values converted to i64
        for (i, arg) in args.iter().enumerate() {
            let (v, t) = self.codegen_expr(arg)?;
            let payload_gep = self.fresh_temp();
            self.out.push_str(&format!(
                "  {} = getelementptr inbounds {}, ptr {}, i64 0, i32 1, i32 {}\n",
                payload_gep, llvm_type, tmp_alloca, i
            ));
            let converted = self.convert_to_i64(&v, &t)?;
            self.out.push_str(&format!(
                "  store i64 {}, i64* {}, align 8\n",
                converted, payload_gep
            ));
        }

        let tmp = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = load {}, {}* {}, align 8\n",
            tmp, llvm_type, llvm_type, tmp_alloca
        ));
        Ok((tmp, Type::State(state_name.to_string())))
    }

    /// Phase 4: convert a value to i64 for state payload storage
    fn convert_to_i64(&mut self, v: &str, t: &Type) -> Result<String, String> {
        match t {
            Type::Int => Ok(self.bare_value(v).to_string()),
            Type::Float => {
                let src = if v.starts_with('%') {
                    format!("double {}", v)
                } else {
                    v.to_string()
                };
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = bitcast {} to i64\n", tmp, src));
                Ok(tmp)
            }
            Type::Bool => {
                let src = if v.starts_with('%') {
                    format!("i1 {}", v)
                } else {
                    v.to_string()
                };
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = zext {} to i64\n", tmp, src));
                Ok(tmp)
            }
            Type::String => {
                let src = if v.starts_with('%') {
                    format!("i8* {}", v)
                } else {
                    v.to_string()
                };
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = ptrtoint {} to i64\n", tmp, src));
                Ok(tmp)
            }
            _ => Err(format!(
                "Phase 5: conversion to i64 not supported for {:?}",
                t
            )),
        }
    }

    /// Phase 2: codegen print/println for one argument (Int, Float, Bool)
    fn codegen_print(&mut self, arg: &Expr, add_nl: bool) -> Result<(), String> {
        let (v, t) = self.codegen_expr(arg)?;
        let nl_suffix = if add_nl { "_nl" } else { "" };
        match t {
            Type::Int => {
                let arg_str = if v.starts_with('%') {
                    format!("i64 {}", v)
                } else {
                    v.clone()
                };
                let arr_size = if add_nl { "6" } else { "5" };
                self.out.push_str(&format!(
                    "  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([{} x i8], ptr @.str.int{}, i64 0, i64 0), {})\n",
                    arr_size, nl_suffix, arg_str
                ));
            }
            Type::Float => {
                let arg_str = if v.starts_with('%') {
                    format!("double {}", v)
                } else {
                    v.clone()
                };
                let arr_size = if add_nl { "7" } else { "6" };
                self.out.push_str(&format!(
                    "  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([{} x i8], ptr @.str.float{}, i64 0, i64 0), {})\n",
                    arr_size, nl_suffix, arg_str
                ));
            }
            Type::Bool => {
                let bare = self.bare_value(&v).to_string();
                let str_tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = select i1 {}, i8* getelementptr inbounds ([5 x i8], ptr @.str.true, i64 0, i64 0), i8* getelementptr inbounds ([6 x i8], ptr @.str.false, i64 0, i64 0)\n",
                    str_tmp, bare
                ));
                let arr_size_str = if add_nl { "4" } else { "3" };
                self.out.push_str(&format!(
                    "  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([{} x i8], ptr @.str.str{}, i64 0, i64 0), i8* {})\n",
                    arr_size_str, nl_suffix, str_tmp
                ));
            }
            Type::String => {
                let arg_str = if v.starts_with('%') {
                    format!("i8* {}", v)
                } else {
                    v.clone()
                };
                let arr_size = if add_nl { "4" } else { "3" };
                self.out.push_str(&format!(
                    "  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([{} x i8], ptr @.str.str{}, i64 0, i64 0), {})\n",
                    arr_size, nl_suffix, arg_str
                ));
            }
            Type::Json => {
                // JSON: stringify first, then print the resulting string
                let json_str = self.fresh_temp();
                let arg_str = if v.starts_with('%') {
                    format!("i8* {}", v)
                } else {
                    v.clone()
                };
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_json_stringify(i8* {})\n",
                    json_str, arg_str
                ));
                let arr_size = if add_nl { "4" } else { "3" };
                self.out.push_str(&format!(
                    "  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([{} x i8], ptr @.str.str{}, i64 0, i64 0), i8* {})\n",
                    arr_size, nl_suffix, json_str
                ));
            }
            _ => {
                // Fallback: convert to string via str(), then print the
                // resulting i8* — covers Option, Result, State, List, and
                // future aggregate types without requiring per-type
                // formatting logic in the println handler.
                let (str_val, _str_ty) = self.codegen_call("str", &[(*arg).clone()])?;
                let bare = self.bare_value(&str_val).to_string();
                let arr_size = if add_nl { "4" } else { "3" };
                self.out.push_str(&format!(
                    "  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([{} x i8], ptr @.str.str{}, i64 0, i64 0), i8* {})\n",
                    arr_size, nl_suffix, bare
                ));
            }
        }
        Ok(())
    }

    /// Format a value for use as a call argument: add type prefix if needed.
    fn fmt_call_arg(&self, v: &str, t: &Type) -> String {
        if v.is_empty() {
            return v.to_string();
        }
        if v.starts_with('%') {
            format!("{} {}", llvm_type_name(t), v)
        } else {
            v.to_string()
        }
    }

    /// Extract bare value (strip type prefix) from a typed constant.
    fn bare_value<'b>(&self, v: &'b str) -> &'b str {
        for p in &["i64 ", "double ", "i1 ", "i8* ", "void "] {
            if let Some(rest) = v.strip_prefix(p) {
                return rest;
            }
        }
        v
    }

    /// Phase 4: match codegen (switch on tag + extract payload + bind)
    fn codegen_match(&mut self, expr: &Expr, arms: &[(Pattern, Vec<Stmt>)]) -> Result<(), String> {
        let (val, ty) = self.codegen_expr(expr)?;
        // `Option(T)` is represented as the `Option` tagged-union state at the
        // LLVM level (see emit_aggregate_decls), so normalize the dual type
        // form before looking up the state's variants.
        let ty = match ty {
            Type::Option(inner) => Type::State(format!("Option({})", crate::type_to_string(&inner))),
            other => other,
        };
        match ty {
            Type::State(ref state_name) => {
                // Generic states are registered under their base name
                // (e.g. `Option` for `Option(int)`); look up by base.
                let base = crate::struct_base(state_name);
                let variants = self.defs.states.get(&base).ok_or_else(|| {
                    format!("unknown state '{}'", base)
                })?;
                let llvm_state = format!("%{}", base);

                let tag = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = extractvalue {} {}, 0\n",
                    tag, llvm_state, val
                ));

                let merge_b = self.fresh_block();
                let arm_blocks: Vec<String> = (0..arms.len()).map(|_| self.fresh_block()).collect();

                self.out.push_str(&format!(
                    "  switch i32 {}, label %{} [\n",
                    tag, merge_b
                ));
                for (i, (pattern, _)) in arms.iter().enumerate() {
                    if let Pattern::Variant { name: pname, .. } = pattern {
                        if let Some(vidx) = variants.iter().position(|v| v == pname) {
                            self.out.push_str(&format!(
                                "    i32 {}, label %{}\n",
                                vidx as i32, arm_blocks[i]
                            ));
                        }
                    }
                }
                self.out.push_str("  ]\n");

                for (i, (pattern, body)) in arms.iter().enumerate() {
                    self.out.push_str(&format!("{}:\n", arm_blocks[i]));
                    self.current_block = arm_blocks[i].clone();

                    if let Pattern::Variant { name: pname, bindings } = pattern {
                        if let Some(vidx) = variants.iter().position(|v| v == pname) {
                            for (j, binding) in bindings.iter().enumerate() {
                                if binding != "Ignore" {
                                    let payload = self.fresh_temp();
                                    self.out.push_str(&format!(
                                        "  {} = extractvalue {} {}, 1, {}\n",
                                        payload, llvm_state, val, j
                                    ));
                                    let ptr = self.fresh_temp();
                                    self.out.push_str(&format!(
                                        "  {} = alloca i64, align 8\n",
                                        ptr
                                    ));
                                    self.out.push_str(&format!(
                                        "  store i64 {}, i64* {}, align 8\n",
                                        payload, ptr
                                    ));
                                    self.env.insert(binding.clone(), Type::Int);
                                    self.named.insert(binding.clone(), ptr);
                                }
                            }
                            let _ = vidx;
                        }
                    }

                    self.codegen_stmts(body)?;
                    // If the arm already returned (block terminated), a trailing
                    // `br` would be an instruction after a terminator.
                    if !self.block_terminated() {
                        self.out.push_str(&format!("  br label %{}\n", merge_b));
                    }
                }

                self.out.push_str(&format!("{}:\n", merge_b));
                self.current_block = merge_b;
                // If the match is the final statement of a function and every
                // arm terminated, the merge block is empty; the function-end
                // ret (or an enclosing `br`) terminates it. Nothing to emit here.
                Ok(())
            }
            _ => Err(format!(
                "Phase 4: match only supports state types, got {:?}",
                ty
            )),
        }
    }

    // Phase 5: int method codegen (chr -> single-char owned string)
    fn codegen_int_method(&mut self, obj: &str, method: &str, args: &[Expr]) -> Result<(String, Type), String> {
        match method {
            "chr" => {
                if args.len() != 0 {
                    return Err("chr() takes no arguments (receiver is the byte)".to_string());
                }
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_str_from_byte({})\n",
                    tmp,
                    self.fmt_call_arg(obj, &Type::Int)
                ));
                Ok((tmp, Type::String))
            }
            other => Err(format!("unknown int method '{}'", other)),
        }
    }

    // Phase 5: string literal -> global constant pointer
    fn codegen_string_lit(&mut self, s: &str) -> Result<(String, Type), String> {
        let global_name = self.string_literals.get(s).ok_or_else(|| {
            format!("string literal '{}' not found in globals", s)
        })?;
        let len = s.len() as i64; // bytes (excludes NUL); capacity for runtime_str_new
        let gsrc = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = getelementptr inbounds [{} x i8], ptr @{}, i64 0, i64 0\n",
            gsrc, len + 1, global_name
        ));
        // OPT-002: emit every string literal as an OWNED, header-backed string
        // (via runtime_str_new) so it carries the OWNED marker. This makes
        // runtime_str_concat's in-place reuse always memory-safe.
        let owned = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = call i8* @runtime_str_new(i64 {})\n",
            owned, len
        ));
        self.out.push_str(&format!(
            "  call void @llvm.memcpy.p0.p0.i64(i8* {}, i8* {}, i64 {}, i1 false)\n",
            owned, gsrc, len + 1
        ));
        Ok((owned, Type::String))
    }

    // Phase 5: array literal -> construct %LimeList on stack
    fn codegen_array_lit(&mut self, items: &[Expr]) -> Result<(String, Type), String> {
        let count = items.len();
        if count == 0 {
            // Empty list literal: use runtime_list_empty() (returns {NULL,0,0})
            // rather than hand-rolling a runtime_alloc(8) + insertvalue. This keeps
            // the data pointer NULL so grow_list's first realloc behaves correctly.
            let tmp = self.fresh_temp();
            self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", tmp));
            self.out.push_str(&format!("  call void @runtime_list_empty(ptr {})\n", tmp));
            let loaded = self.fresh_temp();
            self.out.push_str(&format!("  {} = load %LimeList, ptr {}, align 8\n", loaded, tmp));
            return Ok((loaded, Type::List(Box::new(Type::Unknown))));
        }
        let mut elem_values: Vec<(String, Type)> = Vec::new();
        for item in items {
            elem_values.push(self.codegen_expr(item)?);
        }
        let elem_type = if count > 0 {
            elem_values[0].1.clone()
        } else {
            Type::Unknown
        };
        let arr_size = if count > 0 { count } else { 1 };

        let arr_ptr = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = call i8* @runtime_alloc(i64 {}, i64 8)\n",
            arr_ptr, (arr_size as i64) * 8
        ));

        for (i, (v, t)) in elem_values.iter().enumerate() {
            let converted = self.convert_to_i64(v, t)?;
            let elem_gep = self.fresh_temp();
            self.out.push_str(&format!(
                "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n",
                elem_gep, arr_ptr, i
            ));
            self.out.push_str(&format!(
                "  store i64 {}, i64* {}, align 8\n",
                converted, elem_gep
            ));
        }

        let mut cur = String::from("undef");
        let t1 = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = insertvalue %LimeList {}, i8* {}, 0\n",
            t1, cur, arr_ptr
        ));
        cur = t1;
        let t2 = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = insertvalue %LimeList {}, i64 {}, 1\n",
            t2, cur, count as i64
        ));
        cur = t2;
        let t3 = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = insertvalue %LimeList {}, i64 {}, 2\n",
            t3, cur, count as i64
        ));

        Ok((t3, Type::List(Box::new(elem_type))))
    }

    // Phase 5/7: method call dispatch by object type
    fn codegen_method_call(&mut self, object: &Expr, method: &str, args: &[Expr]) -> Result<(String, Type), String> {
        // Phase B.1: if `object` is a length-tracked string variable and the
        // method is a length query, return the tracked length directly instead
        // of calling strlen.
        if matches!(method, "len" | "byte_len" | "length") {
            if let Expr::Ident(name) = object {
                if let Some(len_ptr) = self.string_len_trackers.get(name).cloned() {
                    let loaded = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = load i64, i64* {}\n",
                        loaded, len_ptr
                    ));
                    return Ok((loaded, Type::Int));
                }
            }
        }
        let (obj_v, obj_t) = self.codegen_expr(object)?;
        match obj_t {
            Type::String => self.codegen_string_method(&obj_v, method, args),
            Type::Int | Type::Long => self.codegen_int_method(&obj_v, method, args),
            Type::List(ref elem) => {
                // `add`/`set` mutate the receiver: the runtime returns a new
                // list, which must be stored back into the receiver variable
                // (mirrors the interpreter's rebind in eval_expr). Non-ident
                // receivers (e.g. temporaries) stay pure.
                let rebind_slot = if matches!(method, "add" | "set") {
                    match object {
                        Expr::Ident(name) => self.named.get(name).cloned(),
                        _ => None,
                    }
                } else {
                    None
                };
                self.codegen_list_method(&obj_v, method, args, elem, rebind_slot)
            }
            // Phase 7: struct method call -> direct LLVM function call
            Type::Struct(ref sname) => {
                self.codegen_struct_method_call(sname, &obj_v, method, args)
            }
            // Phase 7: interface dispatch stub (concrete type unknown at compile time)
            Type::Interface(ref iface, _) => {
                let idef = self.defs.interfaces.get(iface).ok_or_else(|| {
                    format!("unknown interface '{}'", iface)
                })?;
                let im = idef.methods.iter().find(|m| m.name == method).ok_or_else(|| {
                    format!("unknown method '{}.{}'", iface, method)
                })?;
                // Stub: emit zero value of the interface method's return type
                let ret_type = match &im.return_type {
                    Some(rt) => {
                        let t = type_from_str(rt, self.defs);
                        (llvm_type_name(&t), t)
                    }
                    None => ("void".to_string(), Type::Unit),
                };
                if ret_type.0 == "void" {
                    Ok((String::new(), Type::Unit))
                } else {
                    let val = format!("{} {}", ret_type.0, zero_value_for_type(&ret_type.1));
                    Ok((val, ret_type.1))
                }
            }
            _ => Err(format!(
                "Phase 5: method calls not supported on {:?}",
                obj_t
            )),
        }
    }

    // Phase 7: struct method call -> direct LLVM call @StructName_method
    fn codegen_struct_method_call(
        &mut self,
        sname: &str,
        obj_v: &str,
        method: &str,
        args: &[Expr],
    ) -> Result<(String, Type), String> {
        let sdef = self.defs.structs.get(sname).ok_or_else(|| {
            format!("unknown struct '{}'", sname)
        })?;
        let mdef = sdef.methods.get(method).ok_or_else(|| {
            format!("unknown method '{}.{}'", sname, method)
        })?;
        let method_func = format!("{}_{}", sname, method);
        let mut call_args = vec![self.fmt_call_arg(obj_v, &Type::Struct(sname.to_string()))];
        for (arg, (_, ptype)) in args.iter().zip(&mdef.params) {
            let (v, _) = self.codegen_expr(arg)?;
            let t = type_from_str(ptype, self.defs);
            call_args.push(self.fmt_call_arg(&v, &t));
        }
        let call_args_str = call_args.join(", ");
        let ret_type = match &mdef.return_type {
            Some(rt) => {
                let t = type_from_str(rt, self.defs);
                (llvm_type_name(&t), t)
            }
            None => ("void".to_string(), Type::Unit),
        };
        if ret_type.0 == "void" {
            self.out.push_str(&format!(
                "  call void @{}({})\n",
                method_func, call_args_str
            ));
            Ok((String::new(), Type::Unit))
        } else {
            let tmp = self.fresh_temp();
            self.out.push_str(&format!(
                "  {} = call {} @{}({})\n",
                tmp, ret_type.0, method_func, call_args_str
            ));
            Ok((tmp, ret_type.1))
        }
    }

    // Phase 5: string method codegen (len, byte_len, slice, chars, bytes, byte)
    fn codegen_string_method(&mut self, obj: &str, method: &str, args: &[Expr]) -> Result<(String, Type), String> {
        match method {
            "len" | "byte_len" | "length" => {
                let tmp = self.fresh_temp();
                let obj_arg = if obj.starts_with('%') {
                    format!("i8* {}", obj)
                } else {
                    obj.to_string()
                };
                self.out.push_str(&format!(
                    "  {} = call i64 @strlen({})\n",
                    tmp, obj_arg
                ));
                Ok((tmp, Type::Int))
            }
            "byte" => {
                if args.len() != 1 {
                    return Err("byte() takes exactly 1 argument (index)".to_string());
                }
                let (idx_v, _) = self.codegen_expr(&args[0])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i64 @runtime_str_byte(i8* {}, {})\n",
                    tmp,
                    obj,
                    self.fmt_call_arg(&idx_v, &Type::Int)
                ));
                Ok((tmp, Type::Int))
            }
            "chr" => {
                if args.len() != 1 {
                    return Err("chr() takes exactly 1 argument (byte)".to_string());
                }
                let (idx_v, _) = self.codegen_expr(&args[0])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_str_from_byte({})\n",
                    tmp,
                    self.fmt_call_arg(&idx_v, &Type::Int)
                ));
                Ok((tmp, Type::String))
            }
            "push_byte" => {
                if args.len() != 1 {
                    return Err("push_byte() takes exactly 1 argument (byte)".to_string());
                }
                let (idx_v, _) = self.codegen_expr(&args[0])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_str_push_byte(i8* {}, {})
",
                    tmp,
                    obj,
                    self.fmt_call_arg(&idx_v, &Type::Int)
                ));
                Ok((tmp, Type::String))
            }
            "slice" => {
                if args.len() != 2 {
                    return Err("slice() takes exactly 2 arguments (start, end)".to_string());
                }
                let (start_v, _) = self.codegen_expr(&args[0])?;
                let (end_v, _) = self.codegen_expr(&args[1])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_str_slice(i8* {}, {}, {})\n",
                    tmp,
                    obj,
                    self.fmt_call_arg(&start_v, &Type::Int),
                    self.fmt_call_arg(&end_v, &Type::Int)
                ));
                Ok((tmp, Type::String))
            }
            "chars" => {
                let slot = self.fresh_temp();
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", slot));
                self.out.push_str(&format!(
                    "  call void @runtime_str_chars(ptr sret(%LimeList) {}, ptr {})\n",
                    slot, obj
                ));
                self.out.push_str(&format!(
                    "  {} = load %LimeList, ptr {}, align 8\n",
                    tmp, slot
                ));
                Ok((tmp, Type::List(Box::new(Type::String))))
            }
            "bytes" => {
                let slot = self.fresh_temp();
                let tmp = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", slot));
                self.out.push_str(&format!(
                    "  call void @runtime_str_bytes(ptr sret(%LimeList) {}, ptr {})\n",
                    slot, obj
                ));
                self.out.push_str(&format!(
                    "  {} = load %LimeList, ptr {}, align 8\n",
                    tmp, slot
                ));
                Ok((tmp, Type::List(Box::new(Type::Int))))
            }
            _ => Err(format!("Phase 5: unknown String method '{}'", method)),
        }
    }

    // Phase 5: list method codegen (len, get, add, set)
    fn codegen_list_method(
        &mut self,
        obj: &str,
        method: &str,
        args: &[Expr],
        elem: &Type,
        rebind_slot: Option<String>,
    ) -> Result<(String, Type), String> {
        match method {
            "len" => {
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = extractvalue %LimeList {}, 1\n",
                    tmp, obj
                ));
                Ok((tmp, Type::Int))
            }
            "get" => {
                if args.len() != 1 {
                    return Err("get() takes exactly 1 argument".to_string());
                }
                if *elem != Type::Int {
                    // The element buffer stores i64 slots; only Int lists can
                    // be lowered faithfully today. Refuse rather than emit a
                    // value that would be silently misinterpreted.
                    return Err(format!(
                        "List.get() is only supported for lists of Int (element type is {:?})",
                        elem
                    ));
                }
                let (idx_v, _) = self.codegen_expr(&args[0])?;
                let data = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = extractvalue %LimeList {}, 0\n",
                    data, obj
                ));
                let elem_ptr = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n",
                    elem_ptr, data, self.bare_value(&idx_v)
                ));
                let val = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = load i64, i64* {}, align 8\n",
                    val, elem_ptr
                ));
                Ok((val, Type::Int))
            }
            "add" => {
                if args.len() != 1 {
                    return Err("add() takes exactly 1 argument".to_string());
                }
                let (elem_v, elem_t) = self.codegen_expr(&args[0])?;
                let converted = self.convert_to_i64(&elem_v, &elem_t)?;
                // C ABI: void runtime_list_add(LimeList* list, int64_t elem).
                // When the receiver is a local variable (rebind_slot is its
                // alloca ptr), mutate it in place - no by-value copies. This
                // matches the interpreter's rebind semantics while eliminating
                // the alloca/store/load/store round-trip that otherwise makes
                // hot collection loops (e.g. set_ops) slower than needed.
                if let Some(slot) = rebind_slot {
                    self.out.push_str(&format!("  call void @runtime_list_add(ptr {}, i64 {})\n", slot, converted));
                    let result = self.fresh_temp();
                    self.out.push_str(&format!("  {} = load %LimeList, ptr {}, align 8\n", result, slot));
                    Ok((result, Type::List(Box::new(elem_t))))
                } else {
                    let arg_slot = self.fresh_temp();
                    let result = self.fresh_temp();
                    self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", arg_slot));
                    self.out.push_str(&format!("  store %LimeList {}, ptr {}, align 8\n", obj, arg_slot));
                    self.out.push_str(&format!(
                        "  call void @runtime_list_add(ptr {}, i64 {})\n",
                        arg_slot, converted
                    ));
                    self.out.push_str(&format!("  {} = load %LimeList, ptr {}, align 8\n", result, arg_slot));
                    if let Some(slot) = rebind_slot {
                        self.out.push_str(&format!("  store %LimeList {}, ptr {}, align 8\n", result, slot));
                    }
                    Ok((result, Type::List(Box::new(elem_t))))
                }
            }
            "set" => {
                if args.len() != 2 {
                    return Err("set() takes exactly 2 arguments (index, value)".to_string());
                }
                let (idx_v, _) = self.codegen_expr(&args[0])?;
                let (elem_v, elem_t) = self.codegen_expr(&args[1])?;
                let converted = self.convert_to_i64(&elem_v, &elem_t)?;
                // C ABI: void runtime_list_set(LimeList* list, int64_t index, int64_t elem).
                let arg_slot = self.fresh_temp();
                let result = self.fresh_temp();
                self.out.push_str(&format!("  {} = alloca %LimeList, align 8\n", arg_slot));
                self.out.push_str(&format!("  store %LimeList {}, ptr {}, align 8\n", obj, arg_slot));
                self.out.push_str(&format!(
                    "  call void @runtime_list_set(ptr {}, i64 {}, i64 {})\n",
                    arg_slot, self.bare_value(&idx_v), converted
                ));
                self.out.push_str(&format!("  {} = load %LimeList, ptr {}, align 8\n", result, arg_slot));
                if let Some(slot) = rebind_slot {
                    self.out.push_str(&format!("  store %LimeList {}, ptr {}, align 8\n", result, slot));
                }
                Ok((result, Type::List(Box::new(elem_t))))
            }
            _ => Err(format!("Phase 5: unknown List method '{}'", method)),
        }
    }
}

/// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｿ縺ｪ縺・ Phase 1 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・蝗ｲ繧縺ｮ縺ｿ縺ｪ縺・
fn body_supported(stmts: &[Stmt]) -> bool {
    for s in stmts {
        if !stmt_supported(s) {
            return false;
        }
    }
    true
}

fn stmt_supported(s: &Stmt) -> bool {
    match s {
        Stmt::Let { value, .. } => expr_supported(value),
        Stmt::Return { explicit_type: _, value } => match value {
            Some(e) => expr_supported(e),
            None => true,
        },
        Stmt::Expr(e) => expr_supported(e),
        Stmt::Assign { value, .. } => expr_supported(value),
        Stmt::If { cond, then_branch, else_branch } => {
            expr_supported(cond)
                && body_supported(then_branch)
                && else_branch.as_ref().map(|b| body_supported(b)).unwrap_or(true)
        }
        Stmt::While { cond, body } => expr_supported(cond) && body_supported(body),
        // Phase 4: match (supports all arms if expr and bodies are supported)
        Stmt::Match { expr, arms } => {
            expr_supported(expr) && arms.iter().all(|(_, body)| body_supported(body))
        }
        _ => false,
    }
}

fn expr_supported(e: &Expr) -> bool {
    match e {
        Expr::IntLit(_) | Expr::FloatLit(_) | Expr::BoolLit(_) | Expr::Ident(_) => true,
        Expr::StringLit(_) => true,
        Expr::BinOp { left, op, right, .. } => {
            let ok_op = matches!(
                op.as_str(),
                "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<" | ">" | "<=" | ">=" | "and" | "or"
            );
            ok_op && expr_supported(left) && expr_supported(right)
        }
        Expr::UnOp { op, operand } => op == "not" && expr_supported(operand),
        // Phase 2+3: function calls (print/println builtin + struct ctor + user functions)
        Expr::Call { func, args } => {
            func == "print" || func == "println" || args.iter().all(|a| expr_supported(a))
        }
        // Phase 3: field access on struct
        Expr::FieldAccess { object, .. } => expr_supported(object),
        // Phase 5: method calls (string/list methods)
        Expr::MethodCall { object, args, .. } => {
            expr_supported(object) && args.iter().all(|a| expr_supported(a))
        }
        // Phase 5: array/list literals
        Expr::Array(items) => items.iter().all(|a| expr_supported(a)),
        Expr::Await(inner) => expr_supported(inner),
        Expr::FnDef { params: _, body } => body_supported(body),
        _ => false,
    }
}

/// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｿ縺ｪ縺・(Step 1-7) 縺ｦ繧､繝ｳ繝・・ｽ・ｽ繝医ｒ髢峨�ｸｦ縺ｧ繧｢蜷阪→。
pub fn codegen_function(defs: &Defs, memory: &HashMap<String, MemoryPlace>, string_literals: &HashMap<String, String>, mono_name_map: &HashMap<String, String>, mono_fdefs: &HashMap<String, FunctionDef>, name: &str, fdef: &FunctionDef) -> (String, Vec<String>) {
    let mut cg = Cg::new(defs, memory, string_literals, mono_name_map, mono_fdefs);
    let ir = cg.codegen_function(name, fdef);
    (ir, cg.warnings)
}
