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
            "\n; Function {}(){}\ndefine {} @{} ({}) {{\n",
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
        let _ = self.codegen_stmts(&fdef.body);
        if !self.terminated {
            if ret_ty == "void" {
                self.out.push_str("  ret void\n");
            } else {
                self.out.push_str(&format!("  ret {} {}\n", ret_ty, zero_value_for_type(&type_from_str(
                    fdef.return_type.as_deref().unwrap_or(""), self.defs))));
            }
        }
        self.out.push_str("}\n");
        format!("{}{}", head, self.out)
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
            Stmt::Let { name, value, place, .. } => {
                let (v, ty) = self.codegen_expr(value)?;
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
                    let ptr = self.fresh_temp();
                    self.out.push_str(&format!(
                        "  {} = bitcast i8* {} to {}*\n",
                        ptr, raw, llty
                    ));
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
                    llty, v, llty, ptr, align
                ));
                self.env.insert(name.clone(), ty);
                self.named.insert(name.clone(), ptr);
                Ok(())
            }
            Stmt::Return(e) => {
                self.terminated = true;
                match e {
                    Some(expr) => {
                        let (v, ty) = self.codegen_expr(expr)?;
                        let llty = llvm_type_name(&ty);
                        self.out.push_str(&format!("  ret {} {}\n", llty, v));
                        Ok(())
                    }
                    None => {
                        self.out.push_str("  ret void\n");
                        Ok(())
                    }
                }
            },
            Stmt::Expr(e) => {
                self.codegen_expr(e)?;
                Ok(())
            }
            Stmt::Assign { name, value } => {
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
                    llty, v, llty, ptr, align_of(&ty)
                ));
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
                            .push_str(&format!("  br i1 {}, label %{}, label %{}\n", c, then_b, else_b));
                    }
                    None => {
                        self.out
                            .push_str(&format!("  br i1 {}, label %{}, label %{}\n", c, then_b, merge_b));
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
                self.out.push_str(&format!("  br label %{}\n", cond_b));
                self.out.push_str(&format!("{}:\n", cond_b));
                self.current_block = cond_b.clone();
                let (c, _ct) = self.codegen_expr(cond)?;
                self.out
                    .push_str(&format!("  br i1 {}, label %{}, label %{}\n", c, body_b, merge_b));
                self.out.push_str(&format!("{}:\n", body_b));
                self.current_block = body_b;
                self.codegen_stmts(body)?;
                self.out.push_str(&format!("  br label %{}\n", cond_b));
                self.out.push_str(&format!("{}:\n", merge_b));
                self.current_block = merge_b;
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
                let ptr = self
                    .named
                    .get(n)
                    .cloned()
                    .ok_or_else(|| format!("undefined variable '{}'", n))?;
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
                Ok((tmp, ty))
            }
            Expr::BinOp { left, op, right, resolved_operator } => self.codegen_binop(left, op, right, resolved_operator),
            Expr::UnOp { op, operand } => {
                if op == "not" {
                    let (v, _t) = self.codegen_expr(operand)?;
                    let tmp = self.fresh_temp();
                    self.out.push_str(&format!("  {} = xor i1 {}, true\n", tmp, v));
                    Ok((tmp, Type::Bool))
                } else {
                    Err(format!("Phase 1: unsupported unary operator '{}'", op))
                }
            }
            Expr::FieldAccess { object, field } => self.codegen_field_access(object, field),
            Expr::Call { func, args } => self.codegen_call(func, args),
            Expr::StringLit(s) => self.codegen_string_lit(s),
            Expr::MethodCall { object, method, args } => self.codegen_method_call(object, method, args),
            Expr::Array(items) => self.codegen_array_lit(items),
            _ => Err("Phase 5: unsupported expression in codegen".to_string()),
        }
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
                self.out.push_str(&format!(
                    "  {} = {} {} {}, {}\n",
                    tmp, instr, llty, lv, rv
                ));
                let ty = if float { Type::Float } else { Type::Int };
                Ok((tmp, ty))
            }
            "==" | "!=" | "<" | ">" | "<=" | ">=" => {
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
                    tmp, instr, llty, lv, rv
                ));
                Ok((tmp, Type::Bool))
            }
            "and" => {
                let prev = self.current_block.clone();
                let rhs_b = self.fresh_block();
                let end_b = self.fresh_block();
                self.out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", lv, rhs_b, end_b));
                self.out.push_str(&format!("{}:\n", rhs_b));
                self.current_block = rhs_b.clone();
                self.out.push_str(&format!("  br label %{}\n", end_b));
                self.out.push_str(&format!("{}:\n", end_b));
                self.current_block = end_b;
                let res = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = phi i1 [ false, %{} ], [ {}, %{} ]\n",
                    res, prev, rv, rhs_b
                ));
                Ok((res, Type::Bool))
            }
            "or" => {
                let prev = self.current_block.clone();
                let rhs_b = self.fresh_block();
                let end_b = self.fresh_block();
                self.out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", lv, end_b, rhs_b));
                self.out.push_str(&format!("{}:\n", rhs_b));
                self.current_block = rhs_b.clone();
                self.out.push_str(&format!("  br label %{}\n", end_b));
                self.out.push_str(&format!("{}:\n", end_b));
                self.current_block = end_b;
                let res = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = phi i1 [ true, %{} ], [ {}, %{} ]\n",
                    res, prev, rv, rhs_b
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
        // Phase 3: struct constructor
        if let Some(sdef) = self.defs.structs.get(func) {
            return self.codegen_struct_ctor(func, sdef, args);
        }
        // Phase 4: state variant constructor (Success/Error or user state variant)
        if let Some(state_name) = self.defs.state_variants.get(func) {
            return self.codegen_state_ctor(state_name, func, args);
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
        let fdef = self.defs.functions.get(func).ok_or_else(|| {
            format!("undefined function '{}'", func)
        })?;
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
                let llvm_field_type = llvm_type_name(&field_type);
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
            "  {} = getelementptr inbounds {}, {}* {}, i64 0, i32 0\n",
            tag_gep, llvm_type, llvm_type, tmp_alloca
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
                "  {} = getelementptr inbounds {}, {}* {}, i64 0, i32 1, i32 {}\n",
                payload_gep, llvm_type, llvm_type, tmp_alloca, i
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
                    "  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([{} x i8], [{} x i8]* @.str.int{}, i64 0, i64 0), {})\n",
                    arr_size, arr_size, nl_suffix, arg_str
                ));
            }
            Type::Float => {
                let arg_str = if v.starts_with('%') {
                    format!("double {}", v)
                } else {
                    v.clone()
                };
                let arr_size = if add_nl { "4" } else { "3" };
                self.out.push_str(&format!(
                    "  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([{} x i8], [{} x i8]* @.str.float{}, i64 0, i64 0), {})\n",
                    arr_size, arr_size, nl_suffix, arg_str
                ));
            }
            Type::Bool => {
                let bare = self.bare_value(&v).to_string();
                let str_tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = select i1 {}, i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str.true, i64 0, i64 0), i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str.false, i64 0, i64 0)\n",
                    str_tmp, bare
                ));
                let arr_size_str = if add_nl { "4" } else { "3" };
                self.out.push_str(&format!(
                    "  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([{} x i8], [{} x i8]* @.str.str{}, i64 0, i64 0), i8* {})\n",
                    arr_size_str, arr_size_str, nl_suffix, str_tmp
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
                    "  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([{} x i8], [{} x i8]* @.str.str{}, i64 0, i64 0), {})\n",
                    arr_size, arr_size, nl_suffix, arg_str
                ));
            }
            _ => {
                return Err(format!(
                    "Phase 2: print/println not supported for {}",
                    llvm_type_name(&t)
                ))
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
        match ty {
            Type::State(ref state_name) => {
                let variants = self.defs.states.get(state_name).ok_or_else(|| {
                    format!("unknown state '{}'", state_name)
                })?;
                let llvm_state = format!("%{}", state_name);

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
                    self.out.push_str(&format!("  br label %{}\n", merge_b));
                }

                self.out.push_str(&format!("{}:\n", merge_b));
                self.current_block = merge_b;
                Ok(())
            }
            _ => Err(format!(
                "Phase 4: match only supports state types, got {:?}",
                ty
            )),
        }
    }

    // Phase 5: string literal -> global constant pointer
    fn codegen_string_lit(&mut self, s: &str) -> Result<(String, Type), String> {
        let global_name = self.string_literals.get(s).ok_or_else(|| {
            format!("string literal '{}' not found in globals", s)
        })?;
        let len = s.len() + 1;
        let tmp = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = getelementptr inbounds ([{} x i8], [{} x i8]* @{}, i64 0, i64 0)\n",
            tmp, len, len, global_name
        ));
        Ok((tmp, Type::String))
    }

    // Phase 5: array literal -> construct %LimeList on stack
    fn codegen_array_lit(&mut self, items: &[Expr]) -> Result<(String, Type), String> {
        let count = items.len();
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
            "  {} = alloca [{} x i64], align 8\n",
            arr_ptr, arr_size
        ));

        for (i, (v, t)) in elem_values.iter().enumerate() {
            let converted = self.convert_to_i64(v, t)?;
            let elem_gep = self.fresh_temp();
            self.out.push_str(&format!(
                "  {} = getelementptr inbounds [{} x i64], [{} x i64]* {}, i64 0, i64 {}\n",
                elem_gep, arr_size, arr_size, arr_ptr, i
            ));
            self.out.push_str(&format!(
                "  store i64 {}, i64* {}, align 8\n",
                converted, elem_gep
            ));
        }

        let data_ptr = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = bitcast [{} x i64]* {} to i8*\n",
            data_ptr, arr_size, arr_ptr
        ));

        let mut cur = String::from("undef");
        let t1 = self.fresh_temp();
        self.out.push_str(&format!(
            "  {} = insertvalue %LimeList {}, i8* {}, 0\n",
            t1, cur, data_ptr
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
        let (obj_v, obj_t) = self.codegen_expr(object)?;
        match obj_t {
            Type::String => self.codegen_string_method(&obj_v, method, args),
            Type::List(_) => self.codegen_list_method(&obj_v, method, args),
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

    // Phase 5: string method codegen (len, byte_len, slice, chars, bytes)
    fn codegen_string_method(&mut self, obj: &str, method: &str, args: &[Expr]) -> Result<(String, Type), String> {
        match method {
            "len" | "byte_len" => {
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
            "slice" => {
                if args.len() != 2 {
                    return Err("slice() takes exactly 2 arguments (start, end)".to_string());
                }
                let (start_v, _) = self.codegen_expr(&args[0])?;
                let (end_v, _) = self.codegen_expr(&args[1])?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call i8* @runtime_str_slice(i8* {}, {}, {})\n",
                    tmp, obj, start_v, end_v
                ));
                Ok((tmp, Type::String))
            }
            "chars" => {
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call %LimeList @runtime_str_chars(i8* {})\n",
                    tmp, obj
                ));
                Ok((tmp, Type::List(Box::new(Type::String))))
            }
            "bytes" => {
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call %LimeList @runtime_str_bytes(i8* {})\n",
                    tmp, obj
                ));
                Ok((tmp, Type::List(Box::new(Type::Int))))
            }
            _ => Err(format!("Phase 5: unknown String method '{}'", method)),
        }
    }

    // Phase 5: list method codegen (len, get, add, set)
    fn codegen_list_method(&mut self, obj: &str, method: &str, args: &[Expr]) -> Result<(String, Type), String> {
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
                let (idx_v, _) = self.codegen_expr(&args[0])?;
                let data = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = extractvalue %LimeList {}, 0\n",
                    data, obj
                ));
                let cast = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = bitcast i8* {} to i64*\n",
                    cast, data
                ));
                let elem_ptr = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = getelementptr inbounds i64, i64* {}, i64 {}\n",
                    elem_ptr, cast, idx_v
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
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call %LimeList @runtime_list_add(%LimeList {}, i64 {})\n",
                    tmp, obj, converted
                ));
                Ok((tmp, Type::List(Box::new(elem_t))))
            }
            "set" => {
                if args.len() != 2 {
                    return Err("set() takes exactly 2 arguments (index, value)".to_string());
                }
                let (idx_v, _) = self.codegen_expr(&args[0])?;
                let (elem_v, elem_t) = self.codegen_expr(&args[1])?;
                let converted = self.convert_to_i64(&elem_v, &elem_t)?;
                let tmp = self.fresh_temp();
                self.out.push_str(&format!(
                    "  {} = call %LimeList @runtime_list_set(%LimeList {}, i64 {}, i64 {})\n",
                    tmp, obj, idx_v, converted
                ));
                Ok((tmp, Type::List(Box::new(elem_t))))
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
        Stmt::Return(e) => match e {
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
        _ => false,
    }
}

/// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・縺ｮ縺ｿ縺ｪ縺・(Step 1-7) 縺ｦ繧､繝ｳ繝・・ｽ・ｽ繝医ｒ髢峨�ｸｦ縺ｧ繧｢蜷阪→。
pub fn codegen_function(defs: &Defs, memory: &HashMap<String, MemoryPlace>, string_literals: &HashMap<String, String>, mono_name_map: &HashMap<String, String>, mono_fdefs: &HashMap<String, FunctionDef>, name: &str, fdef: &FunctionDef) -> String {
    let mut cg = Cg::new(defs, memory, string_literals, mono_name_map, mono_fdefs);
    cg.codegen_function(name, fdef)
}
