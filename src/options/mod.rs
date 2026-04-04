use crate::options::types::Options;

pub mod interface_choices;
mod parser;
pub mod types;

impl Options {
    pub fn parse() -> Self {
        parser::parse_options()
    }

    pub fn should_process_function(&self, function_name: &str) -> bool {
        if self.function_blacklist.contains(function_name) {
            return false;
        }
        self.function_whitelist.is_empty() || self.function_whitelist.contains(function_name)
    }
}
