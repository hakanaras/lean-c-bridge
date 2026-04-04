use crate::{
    clang::types::CFunction,
    generator::{TypeRegistry, c_context::CContext, generate_function, lean_context::LeanContext},
    options::interface_choices::FunctionChoices,
};

pub fn preview_lean_function(
    registry: &TypeRegistry,
    function: &CFunction,
    choices: &FunctionChoices,
    max_lines: usize,
) -> String {
    let mut lean_ctx = LeanContext::new();
    let mut c_ctx = CContext::new();
    generate_function(&mut lean_ctx, &mut c_ctx, registry, function, Some(choices));

    fit_preview_to_lines(lean_ctx.render(), max_lines)
}

fn fit_preview_to_lines(rendered: String, max_lines: usize) -> String {
    if max_lines == 0 {
        return String::new();
    }

    let lines: Vec<&str> = rendered.lines().collect();
    if lines.len() <= max_lines {
        return rendered;
    }

    let Some(extern_index) = lines.iter().position(|line| line.starts_with("@[extern")) else {
        return lines
            .into_iter()
            .take(max_lines)
            .collect::<Vec<_>>()
            .join("\n");
    };

    if extern_index == 0 {
        return lines
            .into_iter()
            .take(max_lines)
            .collect::<Vec<_>>()
            .join("\n");
    }

    let function_lines = &lines[extern_index..];
    let mut trimmed = Vec::with_capacity(function_lines.len() + 1);
    if max_lines > function_lines.len() + 1 {
        let prefix_lines = &lines[..(max_lines - function_lines.len() - 1)];
        trimmed.extend_from_slice(prefix_lines);
    }
    trimmed.push("...");
    trimmed.extend_from_slice(function_lines);
    return trimmed.join("\n");
}
