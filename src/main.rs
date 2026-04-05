use crate::{
    clang::types::CFunction,
    generator::{TypeRegistry, c_context::CContext, generate_function, lean_context::LeanContext},
    options::types::Options,
};

mod clang;
mod generator;
mod options;
mod ui;

const HELPER_FUNCTIONS_C: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/static/helper_functions.c"
));

fn main() {
    let options = Options::parse();

    let parsed = clang::parse_header(&options.input_header, &options);
    let registry = TypeRegistry::from_declarations(&parsed);

    let functions: Vec<CFunction> = parsed
        .iter()
        .filter_map(|d| match d {
            clang::types::CDeclaration::Function {
                name,
                return_type,
                parameters,
                is_variadic: false,
            } => Some(CFunction {
                name: name.clone(),
                return_type: return_type.clone(),
                parameters: parameters.clone(),
            }),
            _ => return None,
        })
        .filter(|f| options.should_process_function(&f.name))
        .collect();

    let choices = if options.ui {
        let choices = ui::run(
            options.interface_choices,
            functions.clone(),
            registry.clone(),
        )
        .unwrap();
        if !options.dont_save_interface_choices {
            choices
                .save(&format!(
                    "./lean-c-api-choices_{}.json",
                    chrono::Local::now().format("%a_%H-%M-%S").to_string()
                ))
                .unwrap();
        }
        choices
    } else {
        options.interface_choices
    };

    let mut lean_ctx = LeanContext::new();
    let mut c_ctx = CContext::new();

    for function in functions {
        let choices = choices.functions.iter().find(|fc| fc.name == function.name);

        generate_function(&mut lean_ctx, &mut c_ctx, &registry, &function, choices);
    }

    let lean_output = format!(
        "namespace {}\n\n{}",
        &options.lean_namespace,
        lean_ctx.render()
    );

    let c_prefix = format!(
        "#include <lean/lean.h>\n#include <stdlib.h>\n#include <string.h>\n#include \"{}\"\n\n",
        options.input_header
    );
    let c_body = c_ctx.render();
    let c_output = if c_body.is_empty() {
        format!("{}{}", c_prefix, HELPER_FUNCTIONS_C.trim())
    } else {
        format!("{}{}\n\n{}", c_prefix, HELPER_FUNCTIONS_C.trim(), c_body)
    };

    let output_dir_str = options.output_dir;
    let lean_filename = options.lean_module_name.clone() + ".lean";
    let c_filename = options.lean_module_name + "_glue.c";

    std::fs::write(format!("{}/{}", output_dir_str, lean_filename), lean_output).unwrap();
    std::fs::write(format!("{}/{}", output_dir_str, c_filename), c_output).unwrap();
}
