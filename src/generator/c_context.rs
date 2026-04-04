use crate::generator::type_declarations::{DeclarationSource, declare};

pub struct CContext {
    declarations: Vec<DeclarationSource>,
}

impl CContext {
    pub fn new() -> Self {
        Self {
            declarations: Vec::new(),
        }
    }

    pub fn declare(&mut self, name: String, source: String) {
        declare(&mut self.declarations, name, source);
    }

    pub fn render(&self) -> String {
        self.declarations
            .iter()
            .map(|d| d.source())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}
