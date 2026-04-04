use std::collections::HashSet;
use std::fs;

use clap::{Arg, Command};

use crate::options::interface_choices::InterfaceChoices;
use crate::options::types::Options;

pub fn parse_options() -> Options {
    let matches = Command::new("lean-c-bridge")
        .about("Generate Lean 4 FFI bindings from C header files")
        .arg(
            Arg::new("ui")
                .long("ui")
                .action(clap::ArgAction::SetTrue)
                .help(
                    "Run an interactive terminal UI to configure FFI generation in greater detail",
                ),
        )
        .arg(
            Arg::new("dont-save-interface-choices")
                .long("dont-save-interface-choices")
                .action(clap::ArgAction::SetTrue)
                .help("Do not save interface choices to a file after running the UI"),
        )
        .arg(
            Arg::new("clang-arg")
                .long("clang-arg")
                .value_name("arg")
                .action(clap::ArgAction::Append)
                .help("Command line arguments to be passed along to clang (repeatable)"),
        )
        .arg(
            Arg::new("function-blacklist")
                .long("function-blacklist")
                .value_name("filepath")
                .help("File containing one function name per line to exclude from processing"),
        )
        .arg(
            Arg::new("function-whitelist")
                .long("function-whitelist")
                .value_name("filepath")
                .help("File containing one function name per line; only these will be processed"),
        )
        .arg(
            Arg::new("interface-choices")
                .long("interface-choices")
                .value_name("filepath")
                .help("Load a file of interface choices, saved from a previous UI run"),
        )
        .arg(
            Arg::new("lean-module-name")
                .long("lean-module-name")
                .value_name("module-name")
                .required(true)
                .help("The name to be used for the Lean module"),
        )
        .arg(
            Arg::new("lean-namespace")
                .long("lean-namespace")
                .value_name("namespace")
                .help(
                    "Namespace to contain the Lean declarations (defaults to --lean-module-name)",
                ),
        )
        .arg(
            Arg::new("output-dir")
                .long("output-dir")
                .value_name("dir")
                .default_value(".")
                .help(
                    "Where to generate the Lean and C output files (defaults to current directory)",
                ),
        )
        .arg(
            Arg::new("header")
                .value_name("header-filepath")
                .required(true)
                .help("C header file or source file without externally linked symbols"),
        )
        .get_matches();

    let ui = matches.get_flag("ui");
    let dont_save_interface_choices = matches.get_flag("dont-save-interface-choices");

    let clang_args = matches
        .get_many::<String>("clang-arg")
        .map(|vals| vals.cloned().collect())
        .unwrap_or_default();

    let function_blacklist = read_names_file(matches.get_one::<String>("function-blacklist"));
    let function_whitelist = read_names_file(matches.get_one::<String>("function-whitelist"));

    let lean_module_name = matches
        .get_one::<String>("lean-module-name")
        .unwrap()
        .clone();

    let lean_namespace = matches
        .get_one::<String>("lean-namespace")
        .cloned()
        .unwrap_or_else(|| lean_module_name.clone());

    let output_dir = matches.get_one::<String>("output-dir").unwrap().clone();

    let input_header = matches.get_one::<String>("header").unwrap().clone();

    let interface_choices = if let Some(filepath) = matches.get_one::<String>("interface-choices") {
        InterfaceChoices::load(filepath).unwrap()
    } else {
        InterfaceChoices {
            functions: Vec::new(),
        }
    };

    Options {
        ui,
        clang_args,
        function_blacklist,
        function_whitelist,
        interface_choices,
        lean_module_name,
        lean_namespace,
        output_dir,
        input_header,
        dont_save_interface_choices,
    }
}

fn read_names_file(path: Option<&String>) -> HashSet<String> {
    match path {
        Some(p) => fs::read_to_string(p)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", p, e))
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        None => HashSet::new(),
    }
}
