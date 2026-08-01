




use crate::Type;





pub fn llvm_type_name(ty: &Type) -> String {
    match ty {
        Type::Int => "i64".to_string(),
        Type::Long => "i64".to_string(),
        Type::Float => "double".to_string(),
        Type::Bool => "i1".to_string(),
        Type::String => "i8*".to_string(), 
        Type::Struct(name) => format!("%{}", name),
        Type::State(name) => format!("%{}", name),
        Type::List(_) => "%LimeList".to_string(),
        Type::Option(_) => "%LimeOption".to_string(),
        Type::Interface(_, _) => "%LimeIface".to_string(),
        Type::Array(_) => "i8*".to_string(), 
        Type::Slice(_) => "i8*".to_string(), 
        Type::Tuple(_) => "i64".to_string(), 
        Type::Unit => "void".to_string(),
        Type::Unknown => "i64".to_string(), 
        Type::Var(_) => "i64".to_string(), 
    }
}



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
        Type::Long => 8,
        Type::Float => 8,
        Type::String => 8,
        Type::List(_) => 8,
        Type::Option(_) => 8,
        Type::Array(_) => 8,
        Type::Slice(_) => 8,
        Type::Tuple(_) => 8,
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
