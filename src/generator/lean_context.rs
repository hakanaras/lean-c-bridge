use crate::generator::type_declarations::{DeclarationSource, declare};

pub struct LeanContext {
    declarations: Vec<DeclarationSource>,
}

impl LeanContext {
    pub fn new() -> Self {
        Self {
            declarations: Vec::new(),
        }
    }

    pub fn render(&self) -> String {
        self.declarations
            .iter()
            .map(|d| d.source())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn declare(&mut self, name: String, source: String) {
        declare(&mut self.declarations, name, source);
    }
}
