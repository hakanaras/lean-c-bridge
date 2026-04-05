pub(crate) fn sanitize_c_ident(name: &str) -> String {
    let mut sanitized = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }

    if sanitized.is_empty() {
        sanitized.push_str("ffi_value");
    }
    if sanitized
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit())
    {
        sanitized.insert_str(0, "ffi_");
    }
    sanitized
}

pub(crate) fn sanitize_lean_type_name(name: &str) -> String {
    let mut sanitized = sanitize_c_ident(name);
    if is_lean_keyword(&sanitized) {
        sanitized.insert_str(0, "Ffi");
    }
    sanitized
}

pub(crate) fn sanitize_lean_field_name(name: &str) -> String {
    let mut sanitized = sanitize_c_ident(name);
    if is_lean_keyword(&sanitized) {
        sanitized.insert_str(0, "ffi_");
    }
    sanitized
}

pub(crate) fn sanitize_lean_ctor_name(name: &str) -> String {
    let mut sanitized = sanitize_lean_field_name(name);
    if sanitized == "other" {
        sanitized = "other_".to_string();
    }
    sanitized
}

pub(crate) fn is_lean_keyword(name: &str) -> bool {
    matches!(
        name,
        "Type"
            | "abbrev"
            | "axiom"
            | "by"
            | "class"
            | "def"
            | "do"
            | "else"
            | "end"
            | "export"
            | "forall"
            | "fun"
            | "if"
            | "import"
            | "inductive"
            | "in"
            | "instance"
            | "let"
            | "match"
            | "mut"
            | "namespace"
            | "opaque"
            | "open"
            | "private"
            | "structure"
            | "then"
            | "theorem"
            | "where"
    )
}
