use std::collections::HashSet;

use crate::options::interface_choices::InterfaceChoices;

#[derive(Debug, Clone)]
pub struct Options {
    pub ui: bool,
    pub dont_save_interface_choices: bool,
    pub clang_args: Vec<String>,
    pub function_blacklist: HashSet<String>,
    pub function_whitelist: HashSet<String>,
    pub interface_choices: InterfaceChoices,
    pub lean_module_name: String,
    pub lean_namespace: String,
    pub output_dir: String,
    pub input_header: String,
}
