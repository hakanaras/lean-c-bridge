pub struct DeclarationSource {
    name: String,
    source: String,
}

impl DeclarationSource {
    pub fn source(&self) -> &str {
        &self.source
    }
}

pub fn declare(type_declarations: &mut Vec<DeclarationSource>, name: String, source: String) {
    let existing = type_declarations.iter().find(|decl| decl.name == name);
    match existing {
        Some(decl) => {
            if decl.source != source {
                eprintln!(
                    "Warning: Type '{}' is being redeclared with different source.\n\nPrevious:\n{}\n\nNew:\n{}",
                    name, decl.source, source
                );
            }
            return;
        }
        None => {
            type_declarations.push(DeclarationSource { name, source });
        }
    }
}
