// Phase 0 (Step 10): LLVM backend foundation.
// Lime 縺ｮ Type 縺ｯ LLVM IR 縺ｮ型名(縺ｮ縺ｿ縺ｪ縺・) 縺ｦ榆ｩ縺ｧ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・
// Phase 0 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・(aggregates 縺ｯ Pad 縺ｾ縺ｧ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・)
// Inkwell 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・繝ｼ繝牙ｒ繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・ llvm::Type 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・

use crate::Type;

/// LLVM IR 縺ｮ型名縺ｯ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・(String) 縺ｦ股�ｸｦ縺ｧ繧｢蜷阪→。
/// 蝣ｴ蜷隗｣譫怜ｸ・縺ｮ縺ｿ縺ｪ縺・(aggregates) 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
/// 蜈ｷ雎｡縺ｮ縺ｪ縺ｿ縺ｪ縺・蜻蜷代″縺ｮ縺ｪ縺ｿ縺ｪ縺・ %Name 縺ｯ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・
/// (struct/state/list/option/interface 縺ｯ縺ｮ縺ｿ縺ｪ縺・繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・ mod.rs 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・)
pub fn llvm_type_name(ty: &Type) -> String {
    match ty {
        Type::Int => "i64".to_string(),
        Type::Float => "double".to_string(),
        Type::Bool => "i1".to_string(),
        Type::String => "i8*".to_string(), // Phase 0: fat pointer 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
        Type::Struct(name) => format!("%{}", name),
        Type::State(name) => format!("%{}", name),
        Type::List(_) => "%LimeList".to_string(),
        Type::Option(_) => "%LimeOption".to_string(),
        Type::Interface(_, _) => "%LimeIface".to_string(),
        Type::Array(_) => "i8*".to_string(), // Phase 0: placeholder
        Type::Unit => "void".to_string(),
        Type::Unknown => "i64".to_string(), // Phase 0: placeholder
        Type::Var(_) => "i64".to_string(), // Phase 0: monomorphization 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・
    }
}

/// 蝙区枚蟄怜・ Type::Var(T) 縺ｯ繧｢蜷阪→縺ｮ縺ｪ縺ｿ縺ｪ縺・(Type::Unknown 縺ｾ縺ｧ繧｢蜷阪→) 縺ｮ縺ｪ縺ｿ縺ｪ縺・
/// (Phase 5 monomorphization 縺ｯ繧｢繝ｩ繝ｼ・ｽE・ｽE縺ｮ縺ｪ縺ｿ縺ｪ縺・具象型縺ｯ縺ｪ縺ｿ縺ｪ縺・)
pub fn resolve_var(ty: &Type) -> Type {
    match ty {
        Type::Var(_) => Type::Unknown,
        other => other.clone(),
    }
}

pub fn is_float(ty: &Type) -> bool {
    matches!(ty, Type::Float)
}

pub fn align_of(ty: &Type) -> usize {
    match ty {
        Type::Bool => 1,
        Type::Int => 8,
        Type::Float => 8,
        Type::String => 8,
        Type::List(_) => 8,
        Type::Option(_) => 8,
        Type::Array(_) => 8,
        Type::Struct(_) => 8,
        Type::State(_) => 8,
        Type::Interface(_, _) => 8,
        Type::Unit => 0,
        Type::Unknown => 8,
        Type::Var(_) => 8,
    }
}

pub fn zero_value_for_type(ty: &Type) -> String {
    match ty {
        Type::Float => "0.0".to_string(),
        Type::Bool => "false".to_string(),
        Type::Unit => "".to_string(),
        _ => "0".to_string(),
    }
}
