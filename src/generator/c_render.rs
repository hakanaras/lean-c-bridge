use crate::clang::types::CType;
use std::fmt::Write;

pub(crate) fn render_c_function(
    name: &str,
    return_type: &str,
    signature: &str,
    body_lines: &[String],
) -> String {
    let mut source = String::new();
    let _ = writeln!(&mut source, "{} {}({}) {{", return_type, name, signature);
    for line in body_lines {
        let _ = writeln!(&mut source, "    {}", line);
    }
    source.push('}');
    source
}

pub(crate) fn render_c_type(ty: &CType) -> String {
    match ty {
        CType::Void => "void".to_string(),
        CType::Bool => "bool".to_string(),
        CType::Char => "char".to_string(),
        CType::UChar => "unsigned char".to_string(),
        CType::Short => "short".to_string(),
        CType::UShort => "unsigned short".to_string(),
        CType::Int => "int".to_string(),
        CType::UInt => "unsigned int".to_string(),
        CType::Long => "long".to_string(),
        CType::ULong => "unsigned long".to_string(),
        CType::LongLong => "long long".to_string(),
        CType::ULongLong => "unsigned long long".to_string(),
        CType::Float => "float".to_string(),
        CType::Double => "double".to_string(),
        CType::LongDouble => "long double".to_string(),
        CType::SizeT => "size_t".to_string(),
        CType::PtrdiffT => "ptrdiff_t".to_string(),
        CType::Pointer { is_const, pointee } => {
            if *is_const {
                format!("const {}*", render_c_type(pointee))
            } else {
                format!("{}*", render_c_type(pointee))
            }
        }
        CType::Array { element, size } => match size {
            Some(size) => format!("{}[{}]", render_c_type(element), size),
            None => format!("{}[]", render_c_type(element)),
        },
        CType::Struct(name) => format!("struct {}", name),
        CType::Union(name) => format!("union {}", name),
        CType::Enum(name) => format!("enum {}", name),
        CType::Typedef(name) => name.clone(),
        CType::FunctionPointer {
            return_type,
            parameters,
        } => format!(
            "{}(*)({})",
            render_c_type(return_type),
            parameters
                .iter()
                .map(render_c_type)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        CType::IncompleteArray { element } => format!("{}[]", render_c_type(element)),
        CType::Unknown(name) => name.clone(),
    }
}

pub(crate) fn render_array_declaration(element_ty: &CType, name: &str, size: usize) -> String {
    format!("{} {}[{}]", render_c_type(element_ty), name, size)
}

pub(crate) fn render_zero_initialized_declaration(ty: &CType, name: &str) -> String {
    match ty {
        CType::Array {
            element,
            size: Some(size),
        } => format!("{} = {{0}}", render_array_declaration(element, name, *size)),
        _ => format!("{} {} = {{0}}", render_c_type(ty), name),
    }
}

pub(crate) fn deallocator_name(free_function: Option<&str>) -> &str {
    match free_function.map(str::trim).filter(|name| !name.is_empty()) {
        Some(name) => name,
        None => "free",
    }
}
