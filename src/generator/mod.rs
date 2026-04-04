use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

use crate::{
    clang::types::{CDeclaration, CEnumVariant, CField, CFunction, CType},
    generator::{c_context::CContext, lean_context::LeanContext},
    options::interface_choices::{
        FunctionChoices, ParameterSpecialConversion, ReturnValueSpecialConversion,
    },
};

pub mod c_context;
pub mod lean_context;
mod type_declarations;

#[derive(Debug, Clone, Default)]
pub struct TypeRegistry {
    structs: HashMap<String, Vec<CField>>,
    enums: HashMap<String, Vec<CEnumVariant>>,
    typedefs: HashMap<String, CType>,
    unions: HashSet<String>,
}

impl TypeRegistry {
    pub fn from_declarations(declarations: &[CDeclaration]) -> Self {
        let mut registry = Self::default();

        for declaration in declarations {
            match declaration {
                CDeclaration::Struct {
                    name: Some(name),
                    fields,
                } => {
                    registry.structs.insert(name.clone(), fields.clone());
                }
                CDeclaration::Union {
                    name: Some(name), ..
                } => {
                    registry.unions.insert(name.clone());
                }
                CDeclaration::Enum {
                    name: Some(name),
                    variants,
                } => {
                    registry.enums.insert(name.clone(), variants.clone());
                }
                CDeclaration::Typedef {
                    name,
                    underlying_type,
                } => {
                    registry
                        .typedefs
                        .insert(name.clone(), underlying_type.clone());
                }
                _ => {}
            }
        }

        registry
    }

    fn resolve_alias_type(&self, ty: &CType) -> CType {
        self.resolve_alias_type_inner(ty, &mut HashSet::new())
    }

    fn resolve_alias_type_inner(&self, ty: &CType, seen: &mut HashSet<String>) -> CType {
        match ty {
            CType::Typedef(name) => {
                if !seen.insert(name.clone()) {
                    return ty.clone();
                }

                let resolved = self
                    .typedefs
                    .get(name)
                    .map(|underlying| self.resolve_alias_type_inner(underlying, seen))
                    .unwrap_or_else(|| ty.clone());

                seen.remove(name);
                resolved
            }
            CType::Pointer { is_const, pointee } => CType::Pointer {
                is_const: *is_const,
                pointee: Box::new(self.resolve_alias_type_inner(pointee, seen)),
            },
            CType::Array { element, size } => CType::Array {
                element: Box::new(self.resolve_alias_type_inner(element, seen)),
                size: *size,
            },
            CType::IncompleteArray { element } => CType::IncompleteArray {
                element: Box::new(self.resolve_alias_type_inner(element, seen)),
            },
            CType::FunctionPointer {
                return_type,
                parameters,
            } => CType::FunctionPointer {
                return_type: Box::new(self.resolve_alias_type_inner(return_type, seen)),
                parameters: parameters
                    .iter()
                    .map(|parameter| self.resolve_alias_type_inner(parameter, seen))
                    .collect(),
            },
            _ => ty.clone(),
        }
    }

    fn named_type_name(&self, ty: &CType) -> Option<String> {
        match ty {
            CType::Struct(name)
            | CType::Enum(name)
            | CType::Union(name)
            | CType::Typedef(name) => Some(name.clone()),
            _ => None,
        }
    }

    fn struct_info<'a>(&'a self, ty: &CType) -> Option<(String, &'a [CField])> {
        let lean_name = self.named_type_name(ty)?;
        match self.resolve_alias_type(ty) {
            CType::Struct(name) => self
                .structs
                .get(&name)
                .map(|fields| (sanitize_lean_type_name(&lean_name), fields.as_slice())),
            _ => None,
        }
    }

    fn enum_info<'a>(&'a self, ty: &CType) -> Option<(String, &'a [CEnumVariant])> {
        let lean_name = self.named_type_name(ty)?;
        match self.resolve_alias_type(ty) {
            CType::Enum(name) => self
                .enums
                .get(&name)
                .map(|variants| (sanitize_lean_type_name(&lean_name), variants.as_slice())),
            _ => None,
        }
    }

    fn is_char_pointer_like(&self, ty: &CType) -> bool {
        matches!(
            self.resolve_alias_type(ty),
            CType::Pointer { pointee, .. } if matches!(*pointee, CType::Char)
        )
    }

    fn pointer_element_type(&self, ty: &CType) -> Option<CType> {
        match self.resolve_alias_type(ty) {
            CType::Pointer { pointee, .. } => Some(*pointee),
            CType::IncompleteArray { element } => Some(*element),
            CType::Array {
                element,
                size: None,
            } => Some(*element),
            _ => None,
        }
    }

    fn is_pointer_like(&self, ty: &CType) -> bool {
        matches!(
            self.resolve_alias_type(ty),
            CType::Pointer { .. } | CType::IncompleteArray { .. } | CType::Array { size: None, .. }
        )
    }
}

#[derive(Default)]
struct NameGen {
    next_id: usize,
}

impl NameGen {
    fn next(&mut self, prefix: &str) -> String {
        let id = self.next_id;
        self.next_id += 1;
        format!("ffi_{}_{}", sanitize_c_ident(prefix), id)
    }
}

#[derive(Clone)]
struct LeanParam {
    name: String,
    ty: String,
}

struct PreparedParam {
    lean_param: Option<LeanParam>,
    pre: Vec<String>,
    arg_expr: String,
    post: Vec<String>,
    deferred_length_of: Option<usize>,
    out_param: Option<OutParam>,
}

struct OutParam {
    value_ty: CType,
    value_expr: String,
    value_strategy: Option<ReturnValueSpecialConversion>,
    lean_return_ty: String,
}

struct PreparedValue {
    pre: Vec<String>,
    expr: String,
    post: Vec<String>,
    length_expr: Option<String>,
}

pub fn generate_function(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    function: &CFunction,
    choices: Option<&FunctionChoices>,
) {
    if choices.is_some_and(|c| c.omit) {
        return;
    }

    let use_io = !(choices.is_some_and(|c| c.no_io) && can_be_no_io(function, registry));
    let mut name_gen = NameGen::default();
    let mut length_exprs = HashMap::new();
    let mut params = Vec::with_capacity(function.parameters.len());

    for (index, parameter) in function.parameters.iter().enumerate() {
        let strategy = parameter_strategy(choices, index);
        let parameter_name = parameter
            .name
            .as_deref()
            .map(sanitize_c_ident)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| format!("arg{}", index));

        match strategy {
            Some(ParameterSpecialConversion::Length { of_param_index }) => {
                params.push(PreparedParam {
                    lean_param: None,
                    pre: Vec::new(),
                    arg_expr: String::new(),
                    post: Vec::new(),
                    deferred_length_of: Some(*of_param_index),
                    out_param: None,
                });
            }
            Some(ParameterSpecialConversion::StaticExpr {
                pre_statements,
                expr,
                post_statements,
            }) => {
                params.push(PreparedParam {
                    lean_param: None,
                    pre: pre_statements.clone(),
                    arg_expr: expr.clone(),
                    post: post_statements.clone(),
                    deferred_length_of: None,
                    out_param: None,
                });
            }
            Some(ParameterSpecialConversion::StringBuffer { buffer_size }) => {
                let (prepared, out_param) = match prepare_string_buffer_parameter(
                    lean_ctx,
                    c_ctx,
                    registry,
                    &mut name_gen,
                    &parameter_name,
                    &parameter.ty,
                    *buffer_size,
                ) {
                    Ok(prepared) => prepared,
                    Err(reason) => {
                        emit_omitted_function(lean_ctx, c_ctx, function, &reason);
                        return;
                    }
                };

                if let Some(length_expr) = &prepared.length_expr {
                    length_exprs.insert(index, length_expr.clone());
                }

                params.push(PreparedParam {
                    lean_param: None,
                    pre: prepared.pre,
                    arg_expr: prepared.expr,
                    post: prepared.post,
                    deferred_length_of: None,
                    out_param: Some(out_param),
                });
            }
            Some(ParameterSpecialConversion::Out { element_conversion }) => {
                let out_param = match prepare_out_parameter(
                    lean_ctx,
                    c_ctx,
                    registry,
                    &mut name_gen,
                    &parameter_name,
                    &parameter.ty,
                    element_conversion.as_deref(),
                ) {
                    Ok(out_param) => out_param,
                    Err(reason) => {
                        emit_omitted_function(lean_ctx, c_ctx, function, &reason);
                        return;
                    }
                };

                params.push(PreparedParam {
                    lean_param: None,
                    pre: out_param_stack_prelude(&out_param),
                    arg_expr: format!("&{}", out_param.value_expr),
                    post: Vec::new(),
                    deferred_length_of: None,
                    out_param: Some(out_param),
                });
            }
            _ => {
                let lean_ty = match lean_type_for_parameter(
                    lean_ctx,
                    c_ctx,
                    registry,
                    &parameter.ty,
                    normalize_top_level_strategy(strategy),
                ) {
                    Ok(lean_ty) => lean_ty,
                    Err(reason) => {
                        emit_omitted_function(lean_ctx, c_ctx, function, &reason);
                        return;
                    }
                };

                let lean_name = format!("lean_{}_{}", index, parameter_name);
                let prepared = match prepare_parameter_value(
                    lean_ctx,
                    c_ctx,
                    registry,
                    &mut name_gen,
                    &lean_name,
                    &parameter.ty,
                    normalize_top_level_strategy(strategy),
                    true,
                ) {
                    Ok(prepared) => prepared,
                    Err(reason) => {
                        emit_omitted_function(lean_ctx, c_ctx, function, &reason);
                        return;
                    }
                };

                if let Some(length_expr) = &prepared.length_expr {
                    length_exprs.insert(index, length_expr.clone());
                }

                params.push(PreparedParam {
                    lean_param: Some(LeanParam {
                        name: lean_name,
                        ty: lean_ty,
                    }),
                    pre: prepared.pre,
                    arg_expr: prepared.expr,
                    post: prepared.post,
                    deferred_length_of: None,
                    out_param: None,
                });
            }
        }
    }

    for parameter in &mut params {
        if let Some(length_index) = parameter.deferred_length_of {
            match length_exprs.get(&length_index) {
                Some(length_expr) => {
                    parameter.arg_expr = length_expr.clone();
                    parameter.deferred_length_of = None;
                }
                None => {
                    emit_omitted_function(
                        lean_ctx,
                        c_ctx,
                        function,
                        &format!(
                            "parameter length conversion references unsupported parameter {}",
                            length_index
                        ),
                    );
                    return;
                }
            }
        }
    }

    let return_strategy = choices.and_then(|c| c.return_value.as_ref());
    let base_lean_return_ty = match lean_type_for_return(
        lean_ctx,
        c_ctx,
        registry,
        &function.return_type,
        return_strategy,
    ) {
        Ok(lean_ty) => lean_ty,
        Err(reason) => {
            emit_omitted_function(lean_ctx, c_ctx, function, &reason);
            return;
        }
    };
    let out_return_types = params
        .iter()
        .filter_map(|parameter| parameter.out_param.as_ref())
        .map(|out_param| out_param.lean_return_ty.clone())
        .collect::<Vec<_>>();
    let has_out_params = !out_return_types.is_empty();
    let lean_return_ty = combine_lean_return_type(
        &base_lean_return_ty,
        matches!(registry.resolve_alias_type(&function.return_type), CType::Void),
        &out_return_types,
    );

    let lean_signature = {
        let mut parameter_types = params
            .iter()
            .filter_map(|parameter| parameter.lean_param.as_ref())
            .map(|parameter| format!("@& {}", parameter.ty))
            .collect::<Vec<_>>();

        let result_ty = if use_io {
            format!("IO {}", parenthesize_lean_type(&lean_return_ty))
        } else {
            lean_return_ty
        };

        parameter_types.push(result_ty);
        parameter_types.join(" -> ")
    };

    lean_ctx.declare(
        format!("fn_{}", function.name),
        format!(
            "@[extern \"lean_ffi_{}\"]\nopaque {} : {}",
            function.name, function.name, lean_signature
        ),
    );

    let c_signature = {
        let mut parts = params
            .iter()
            .filter_map(|parameter| parameter.lean_param.as_ref())
            .map(|parameter| format!("{} {}", adapter_parameter_c_type(parameter), parameter.name))
            .collect::<Vec<_>>();

        if use_io {
            parts.push("lean_obj_arg world".to_string());
        }

        parts.join(", ")
    };

    let mut body_lines = Vec::new();
    if use_io {
        body_lines.push("(void)world;".to_string());
    }
    if has_out_params {
        body_lines.push("lean_obj_res returnValue = lean_box(0);".to_string());
    }

    for parameter in &params {
        body_lines.extend(parameter.pre.iter().cloned());
    }

    let call_expr = format!(
        "{}({})",
        function.name,
        params
            .iter()
            .map(|parameter| parameter.arg_expr.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    if matches!(registry.resolve_alias_type(&function.return_type), CType::Void) {
        body_lines.push(format!("{};", call_expr));
    } else {
        body_lines.push(format!(
            "{} c_result = {};",
            render_c_type(&function.return_type),
            call_expr
        ));
    }

    for parameter in params.iter().rev() {
        body_lines.extend(parameter.post.iter().cloned());
    }

    let return_expr = if has_out_params {
        let mut return_components = Vec::new();

        if !matches!(registry.resolve_alias_type(&function.return_type), CType::Void) {
            let return_value = match prepare_return_value(
                lean_ctx,
                c_ctx,
                registry,
                &mut name_gen,
                Some("c_result"),
                &function.return_type,
                return_strategy,
                true,
            ) {
                Ok(value) => value,
                Err(reason) => {
                    emit_omitted_function(lean_ctx, c_ctx, function, &reason);
                    return;
                }
            };
            return_components.push(return_value);
        }

        for out_param in params.iter().filter_map(|parameter| parameter.out_param.as_ref()) {
            let return_value = match prepare_return_value(
                lean_ctx,
                c_ctx,
                registry,
                &mut name_gen,
                Some(&out_param.value_expr),
                &out_param.value_ty,
                out_param.value_strategy.as_ref(),
                true,
            ) {
                Ok(value) => value,
                Err(reason) => {
                    emit_omitted_function(lean_ctx, c_ctx, function, &reason);
                    return;
                }
            };
            return_components.push(return_value);
        }

        for return_value in return_components.into_iter().rev() {
            body_lines.extend(return_value.pre);
            body_lines.push(format!(
                "returnValue = lean_tuple_prepend(returnValue, {});",
                return_value.expr
            ));
            body_lines.extend(return_value.post);
        }

        "returnValue".to_string()
    } else {
        let return_value = match prepare_return_value(
            lean_ctx,
            c_ctx,
            registry,
            &mut name_gen,
            if matches!(registry.resolve_alias_type(&function.return_type), CType::Void) {
                None
            } else {
                Some("c_result")
            },
            &function.return_type,
            return_strategy,
            use_io || !is_lean_float_return(&function.return_type, return_strategy, registry),
        ) {
            Ok(value) => value,
            Err(reason) => {
                emit_omitted_function(lean_ctx, c_ctx, function, &reason);
                return;
            }
        };

        body_lines.extend(return_value.pre.iter().cloned());
        body_lines.extend(return_value.post.iter().cloned());
        return_value.expr
    };

    if use_io {
        body_lines.push(format!(
            "return lean_io_result_mk_ok({});",
            return_expr
        ));
    } else {
        body_lines.push(format!("return {};", return_expr));
    }

    c_ctx.declare(
        format!("lean_ffi_{}", function.name),
        render_c_function(
            &format!("lean_ffi_{}", function.name),
            &adapter_return_c_type(
                use_io,
                has_out_params,
                &function.return_type,
                return_strategy,
                registry,
            ),
            &c_signature,
            &body_lines,
        ),
    );
}

fn emit_omitted_function(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    function: &CFunction,
    reason: &str,
) {
    lean_ctx.declare(
        format!("omitted_{}", function.name),
        format!("-- Omitted {}: {}", function.name, reason),
    );
    c_ctx.declare(
        format!("omitted_{}", function.name),
        format!("/* Omitted {}: {} */", function.name, reason),
    );
}

fn render_c_function(name: &str, return_type: &str, signature: &str, body_lines: &[String]) -> String {
    let mut source = String::new();
    let _ = writeln!(&mut source, "{} {}({}) {{", return_type, name, signature);
    for line in body_lines {
        let _ = writeln!(&mut source, "    {}", line);
    }
    source.push('}');
    source
}

fn adapter_parameter_c_type(parameter: &LeanParam) -> &'static str {
    if parameter.ty == "Float" {
        "double"
    } else {
        "b_lean_obj_arg"
    }
}

fn adapter_return_c_type(
    use_io: bool,
    has_out_params: bool,
    ty: &CType,
    strategy: Option<&ReturnValueSpecialConversion>,
    registry: &TypeRegistry,
) -> String {
    if !use_io && !has_out_params && is_lean_float_return(ty, strategy, registry) {
        "double".to_string()
    } else {
        "lean_obj_res".to_string()
    }
}

fn is_lean_float_return(
    ty: &CType,
    strategy: Option<&ReturnValueSpecialConversion>,
    registry: &TypeRegistry,
) -> bool {
    strategy.is_none()
        && matches!(
            registry.resolve_alias_type(ty),
            CType::Float | CType::Double | CType::LongDouble
        )
}

fn parameter_strategy<'a>(choices: Option<&'a FunctionChoices>, index: usize) -> Option<&'a ParameterSpecialConversion> {
    choices
        .and_then(|choices| choices.parameters.get(index))
        .and_then(|parameter| parameter.conversion_strategy.as_ref())
}

fn normalize_top_level_strategy(
    strategy: Option<&ParameterSpecialConversion>,
) -> Option<&ParameterSpecialConversion> {
    match strategy {
        Some(ParameterSpecialConversion::Out { .. }) => None,
        other => other,
    }
}

fn prepare_out_parameter(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    name_gen: &mut NameGen,
    parameter_name: &str,
    ty: &CType,
    strategy: Option<&ReturnValueSpecialConversion>,
) -> Result<OutParam, String> {
    let value_ty = registry
        .pointer_element_type(ty)
        .ok_or_else(|| "out conversion requires a pointer parameter".to_string())?;
    ensure_out_stack_type_supported(&value_ty)?;

    Ok(OutParam {
        lean_return_ty: lean_type_for_return(lean_ctx, c_ctx, registry, &value_ty, strategy)?,
        value_expr: name_gen.next(&format!("out_{}", parameter_name)),
        value_strategy: strategy.cloned(),
        value_ty,
    })
}

fn out_param_stack_prelude(out_param: &OutParam) -> Vec<String> {
    vec![format!(
        "{};",
        render_zero_initialized_declaration(&out_param.value_ty, &out_param.value_expr)
    )]
}

fn lean_type_for_parameter(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    ty: &CType,
    strategy: Option<&ParameterSpecialConversion>,
) -> Result<String, String> {
    match strategy {
        Some(ParameterSpecialConversion::String) => {
            if registry.is_char_pointer_like(ty) {
                Ok("String".to_string())
            } else {
                Err("string conversion requires a char* parameter".to_string())
            }
        }
        Some(ParameterSpecialConversion::StringBuffer { .. }) => {
            Err("string buffer parameters do not have Lean input types".to_string())
        }
        Some(ParameterSpecialConversion::Array { element_conversion }) => {
            let element_ty = registry
                .pointer_element_type(ty)
                .ok_or_else(|| "array conversion requires a pointer parameter".to_string())?;
            let nested = normalize_nested_strategy(element_conversion.as_deref());
            if matches!(nested, Some(ParameterSpecialConversion::Length { .. }))
                || matches!(nested, Some(ParameterSpecialConversion::StaticExpr { .. }))
            {
                return Err("array element conversions do not support length or static expressions".to_string());
            }

            Ok(format!(
                "Array {}",
                parenthesize_lean_type(&lean_type_for_parameter(
                    lean_ctx,
                    c_ctx,
                    registry,
                    &element_ty,
                    nested,
                )?)
            ))
        }
        Some(ParameterSpecialConversion::Length { .. })
        | Some(ParameterSpecialConversion::StaticExpr { .. }) => {
            Err("omitted parameters do not have Lean types".to_string())
        }
        _ => lean_type_for_default(lean_ctx, c_ctx, registry, ty),
    }
}

fn lean_type_for_return(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    ty: &CType,
    strategy: Option<&ReturnValueSpecialConversion>,
) -> Result<String, String> {
    if matches!(strategy, Some(ReturnValueSpecialConversion::String { .. })) {
        if registry.is_char_pointer_like(ty) {
            return Ok("String".to_string());
        }
        return Err("string return conversion requires a char* return type".to_string());
    }

    if let Some(ReturnValueSpecialConversion::NullTerminatedArray {
        element_conversion,
        ..
    }) = strategy
    {
        let element_ty = registry
            .pointer_element_type(ty)
            .ok_or_else(|| "null-terminated array return conversion requires a pointer-to-pointer return type".to_string())?;

        if !registry.is_pointer_like(&element_ty) {
            return Err("null-terminated array return conversion requires a pointer-to-pointer return type".to_string());
        }

        return Ok(format!(
            "Array {}",
            parenthesize_lean_type(&lean_type_for_return(
                lean_ctx,
                c_ctx,
                registry,
                &element_ty,
                element_conversion.as_deref(),
            )?)
        ));
    }

    lean_type_for_default(lean_ctx, c_ctx, registry, ty)
}

fn lean_type_for_default(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    ty: &CType,
) -> Result<String, String> {
    let resolved = registry.resolve_alias_type(ty);
    match resolved {
        CType::Void => Ok("Unit".to_string()),
        CType::Bool
        | CType::UChar
        | CType::UShort
        | CType::UInt
        | CType::ULong
        | CType::ULongLong
        | CType::SizeT => Ok("Nat".to_string()),
        CType::Char
        | CType::Short
        | CType::Int
        | CType::Long
        | CType::LongLong
        | CType::PtrdiffT => Ok("Int".to_string()),
        CType::Float | CType::Double | CType::LongDouble => Ok("Float".to_string()),
        CType::Enum(_) => ensure_enum_decl(lean_ctx, c_ctx, registry, ty),
        CType::Struct(_) => ensure_struct_decl(lean_ctx, c_ctx, registry, ty),
        CType::Pointer { .. }
        | CType::IncompleteArray { .. }
        | CType::Array { size: None, .. } => ensure_pointer_decl(lean_ctx, ty),
        CType::Array {
            ref element,
            size: Some(size),
        } => {
            if size == 0 {
                return Err("zero-sized arrays are not supported".to_string());
            }
            Ok(format!(
                "Array {}",
                parenthesize_lean_type(&lean_type_for_default(
                    lean_ctx,
                    c_ctx,
                    registry,
                    element,
                )?)
            ))
        }
        CType::Union(name) => Err(format!("union {} is unsupported", name)),
        CType::FunctionPointer { .. } => Err("function pointers are unsupported".to_string()),
        CType::Unknown(name) => Err(format!("unsupported type {}", name)),
        CType::Typedef(_) => Err("unresolved typedef is unsupported".to_string()),
    }
}

fn ensure_pointer_decl(lean_ctx: &mut LeanContext, ty: &CType) -> Result<String, String> {
    let name = pointer_opaque_name(ty)?;
    lean_ctx.declare(
        format!("opaque_{}", name),
        format!("opaque {} : Type", sanitize_lean_type_name(&name)),
    );
    Ok(sanitize_lean_type_name(&name))
}

fn pointer_opaque_name(ty: &CType) -> Result<String, String> {
    match ty {
        CType::Pointer { pointee, .. } => pointee_type_name(pointee).map(|name| format!("{}Ptr", name)),
        CType::IncompleteArray { element } => pointee_type_name(element).map(|name| format!("{}Ptr", name)),
        CType::Array { element, size: None } => pointee_type_name(element).map(|name| format!("{}Ptr", name)),
        _ => Err("type is not pointer-like".to_string()),
    }
}

fn pointee_type_name(ty: &CType) -> Result<String, String> {
    match ty {
        CType::Pointer { .. } | CType::IncompleteArray { .. } | CType::Array { size: None, .. } => {
            pointer_opaque_name(ty)
        }
        CType::Typedef(name) | CType::Struct(name) | CType::Enum(name) | CType::Union(name) => {
            Ok(sanitize_lean_type_name(name))
        }
        CType::Void => Ok("Void".to_string()),
        CType::Bool => Ok("Bool".to_string()),
        CType::Char => Ok("Char".to_string()),
        CType::UChar => Ok("UChar".to_string()),
        CType::Short => Ok("Short".to_string()),
        CType::UShort => Ok("UShort".to_string()),
        CType::Int => Ok("Int".to_string()),
        CType::UInt => Ok("UInt".to_string()),
        CType::Long => Ok("Long".to_string()),
        CType::ULong => Ok("ULong".to_string()),
        CType::LongLong => Ok("LongLong".to_string()),
        CType::ULongLong => Ok("ULongLong".to_string()),
        CType::Float => Ok("Float".to_string()),
        CType::Double => Ok("Double".to_string()),
        CType::LongDouble => Ok("LongDouble".to_string()),
        CType::SizeT => Ok("SizeT".to_string()),
        CType::PtrdiffT => Ok("PtrdiffT".to_string()),
        CType::Array {
            element,
            size: Some(_),
        } => Ok(format!("{}Array", pointee_type_name(element)?)),
        CType::FunctionPointer { .. } => Err("function pointers are unsupported".to_string()),
        CType::Unknown(name) => Ok(sanitize_lean_type_name(name)),
    }
}

fn ensure_enum_decl(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    ty: &CType,
) -> Result<String, String> {
    let (lean_name, variants) = registry
        .enum_info(ty)
        .ok_or_else(|| "enum metadata is not available".to_string())?;

    let variant_data = enum_variants(variants);
    let constructors = variant_data
        .iter()
        .map(|(name, _)| format!("  | {}", name))
        .collect::<Vec<_>>()
        .join("\n");
    let to_arms = variant_data
        .iter()
        .map(|(name, value)| format!("  | {} => .{}", value, name))
        .collect::<Vec<_>>()
        .join("\n");
    let from_arms = variant_data
        .iter()
        .map(|(name, value)| format!("  | .{} => {}", name, value))
        .collect::<Vec<_>>()
        .join("\n");

    let lean_source = format!(
        "inductive {} where\n{}\n  | other (value : Int)\n  deriving Repr, BEq\n\n@[export ffi_to_{}]\ndef ffi_to_{} (value : Int) : {} :=\n  match value with\n{}\n  | other => .other other\n\n@[export ffi_from_{}]\ndef ffi_from_{} (value : {}) : Int :=\n  match value with\n{}\n  | .other other => other",
        lean_name,
        constructors,
        lean_name,
        lean_name,
        lean_name,
        to_arms,
        lean_name,
        lean_name,
        lean_name,
        from_arms,
    );
    lean_ctx.declare(format!("enum_{}", lean_name), lean_source);

    c_ctx.declare(
        format!("ffi_to_{}_decl", lean_name),
        format!(
            "extern lean_obj_res ffi_to_{}(lean_obj_arg value);\nextern lean_obj_res ffi_from_{}(lean_obj_arg value);",
            lean_name, lean_name
        ),
    );

    Ok(lean_name)
}

fn ensure_struct_decl(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    ty: &CType,
) -> Result<String, String> {
    let (lean_name, fields) = registry
        .struct_info(ty)
        .ok_or_else(|| "struct metadata is not available".to_string())?;

    let mut lean_fields = Vec::with_capacity(fields.len());
    let mut helper_params = Vec::with_capacity(fields.len());
    let mut getter_defs = Vec::with_capacity(fields.len());
    let mut getter_decls = Vec::with_capacity(fields.len());

    for field in fields {
        if field.name.is_empty() {
            return Err(format!("struct {} contains an unnamed field", lean_name));
        }

        let lean_field_name = sanitize_lean_field_name(&field.name);
        let lean_field_ty = lean_type_for_default(lean_ctx, c_ctx, registry, &field.ty)?;
        lean_fields.push(format!("  {} : {}", lean_field_name, lean_field_ty));
        helper_params.push(format!("({} : {})", lean_field_name, lean_field_ty));

        let getter_name = struct_getter_name(&lean_name, &field.name);
        getter_defs.push(format!(
            "@[export {getter}]\ndef {getter} (value : {ty}) : {field_ty} :=\n  value.{field_name}",
            getter = getter_name,
            ty = lean_name,
            field_ty = lean_field_ty,
            field_name = lean_field_name,
        ));
        getter_decls.push(format!(
            "extern {} {getter}(lean_obj_arg value);",
            struct_helper_return_c_type(&field.ty, registry),
            getter = getter_name,
        ));
    }

    let constructor_body = if fields.is_empty() {
        "{}".to_string()
    } else {
        format!(
            "{{ {} }}",
            fields
                .iter()
                .map(|field| {
                    let lean_field_name = sanitize_lean_field_name(&field.name);
                    format!("{} := {}", lean_field_name, lean_field_name)
                })
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let lean_source = format!(
        "structure {} where\n{}\n\n@[export ffi_to_{}]\ndef ffi_to_{} {} : {} :=\n  {}\n\n{}",
        lean_name,
        if lean_fields.is_empty() {
            "".to_string()
        } else {
            lean_fields.join("\n")
        },
        lean_name,
        lean_name,
        helper_params.join(" "),
        lean_name,
        constructor_body,
        getter_defs.join("\n\n"),
    );
    lean_ctx.declare(format!("struct_{}", lean_name), lean_source.trim_end().to_string());

    let ffi_to_decl = if fields.is_empty() {
        format!("extern lean_obj_res ffi_to_{}(void);", lean_name)
    } else {
        format!(
            "extern lean_obj_res ffi_to_{}({});",
            lean_name,
            fields
                .iter()
                .map(|field| struct_helper_param_c_type(&field.ty, registry))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let mut c_decl = ffi_to_decl;
    if !getter_decls.is_empty() {
        c_decl.push('\n');
        c_decl.push_str(&getter_decls.join("\n"));
    }
    c_ctx.declare(format!("struct_{}_decl", lean_name), c_decl);

    Ok(lean_name)
}

fn struct_helper_param_c_type(ty: &CType, registry: &TypeRegistry) -> String {
    if is_lean_float_type(ty, registry) {
        "double".to_string()
    } else {
        "lean_obj_arg".to_string()
    }
}

fn struct_helper_return_c_type(ty: &CType, registry: &TypeRegistry) -> String {
    if is_lean_float_type(ty, registry) {
        "double".to_string()
    } else {
        "lean_obj_res".to_string()
    }
}

fn is_lean_float_type(ty: &CType, registry: &TypeRegistry) -> bool {
    matches!(
        registry.resolve_alias_type(ty),
        CType::Float | CType::Double | CType::LongDouble
    )
}

fn prepare_parameter_value(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    name_gen: &mut NameGen,
    lean_expr: &str,
    ty: &CType,
    strategy: Option<&ParameterSpecialConversion>,
    top_level_adapter_param: bool,
) -> Result<PreparedValue, String> {
    match strategy {
        Some(ParameterSpecialConversion::String) => prepare_string_parameter(lean_expr, name_gen, registry, ty),
        Some(ParameterSpecialConversion::StringBuffer { .. }) => {
            Err("string buffer parameters are handled separately".to_string())
        }
        Some(ParameterSpecialConversion::Array { element_conversion }) => prepare_array_parameter(
            lean_ctx,
            c_ctx,
            registry,
            name_gen,
            lean_expr,
            ty,
            normalize_nested_strategy(element_conversion.as_deref()),
        ),
        Some(ParameterSpecialConversion::Length { .. })
        | Some(ParameterSpecialConversion::StaticExpr { .. }) => {
            Err("omitted parameters are handled separately".to_string())
        }
        _ => prepare_default_parameter_value(
            lean_ctx,
            c_ctx,
            registry,
            name_gen,
            lean_expr,
            ty,
            top_level_adapter_param,
        ),
    }
}

fn prepare_string_parameter(
    lean_expr: &str,
    name_gen: &mut NameGen,
    registry: &TypeRegistry,
    ty: &CType,
) -> Result<PreparedValue, String> {
    if !registry.is_char_pointer_like(ty) {
        return Err("string conversion requires a char* parameter".to_string());
    }

    let bytes_var = name_gen.next("string_bytes");
    let cstr_var = name_gen.next("string_cstr");
    Ok(PreparedValue {
        pre: vec![
            format!("size_t {} = lean_string_size({}) - 1;", bytes_var, lean_expr),
            format!(
                "char * {} = (char *)malloc({} + 1);",
                cstr_var, bytes_var
            ),
            format!(
                "if ({} != NULL) memcpy({}, lean_string_cstr({}), {} + 1); else lean_internal_panic_out_of_memory();",
                cstr_var, cstr_var, lean_expr, bytes_var
            ),
        ],
        expr: cstr_var.clone(),
        post: vec![format!("free({});", cstr_var)],
        length_expr: Some(bytes_var),
    })
}

fn prepare_string_buffer_parameter(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    name_gen: &mut NameGen,
    parameter_name: &str,
    ty: &CType,
    buffer_size: usize,
) -> Result<(PreparedValue, OutParam), String> {
    if buffer_size == 0 {
        return Err("string buffer conversion requires a positive buffer size".to_string());
    }
    if !registry.is_char_pointer_like(ty) {
        return Err("string buffer conversion requires a char* parameter".to_string());
    }

    let size_var = name_gen.next(&format!("{}_string_buffer_size", parameter_name));
    let buffer_var = name_gen.next(&format!("{}_string_buffer", parameter_name));
    let return_strategy = ReturnValueSpecialConversion::String { free: true };
    let lean_return_ty = lean_type_for_return(lean_ctx, c_ctx, registry, ty, Some(&return_strategy))?;

    Ok((
        PreparedValue {
            pre: vec![
                format!("size_t {} = {};", size_var, buffer_size),
                format!(
                    "char * {} = (char *)calloc({}, sizeof(char));",
                    buffer_var, size_var
                ),
                format!(
                    "if ({} == NULL) lean_internal_panic_out_of_memory();",
                    buffer_var
                ),
            ],
            expr: buffer_var.clone(),
            post: Vec::new(),
            length_expr: Some(size_var),
        },
        OutParam {
            value_ty: ty.clone(),
            value_expr: buffer_var,
            value_strategy: Some(return_strategy),
            lean_return_ty,
        },
    ))
}

fn prepare_array_parameter(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    name_gen: &mut NameGen,
    lean_expr: &str,
    ty: &CType,
    element_strategy: Option<&ParameterSpecialConversion>,
) -> Result<PreparedValue, String> {
    let element_ty = registry
        .pointer_element_type(ty)
        .ok_or_else(|| "array conversion requires a pointer parameter".to_string())?;
    if matches!(element_strategy, Some(ParameterSpecialConversion::Array { .. })) {
        return Err("nested array element conversions are not supported".to_string());
    }
    if matches!(element_strategy, Some(ParameterSpecialConversion::StaticExpr { .. }))
        || matches!(element_strategy, Some(ParameterSpecialConversion::Length { .. }))
    {
        return Err("array element conversions do not support static expressions or lengths".to_string());
    }

    let len_var = name_gen.next("array_len");
    let data_var = name_gen.next("array_data");
    let index_var = name_gen.next("i");
    let element_c_ty = render_c_type(&element_ty);

    let mut pre = vec![
        format!("size_t {} = lean_array_size({});", len_var, lean_expr),
        format!(
            "{} * {} = {} == 0 ? NULL : ({} *)malloc(sizeof({}) * {});",
            element_c_ty, data_var, len_var, element_c_ty, element_c_ty, len_var
        ),
        format!(
            "if ({} != 0 && {} == NULL) lean_internal_panic_out_of_memory();",
            len_var, data_var
        ),
        format!("for (size_t {} = 0; {} < {}; ++{}) {{", index_var, index_var, len_var, index_var),
    ];

    let mut post = Vec::new();
    match element_strategy {
        Some(ParameterSpecialConversion::String) => {
            if !registry.is_char_pointer_like(&element_ty) {
                return Err("string element conversion requires char* array elements".to_string());
            }
            let bytes_var = name_gen.next("elem_bytes");
            let string_var = name_gen.next("elem_cstr");
            pre.push(format!(
                "    lean_object * ffi_elem = lean_array_get_core({}, {});",
                lean_expr, index_var
            ));
            pre.push(format!(
                "    size_t {} = lean_string_size(ffi_elem) - 1;",
                bytes_var
            ));
            pre.push(format!(
                "    char * {} = (char *)malloc({} + 1);",
                string_var, bytes_var
            ));
            pre.push(format!(
                "    if ({} == NULL) lean_internal_panic_out_of_memory();",
                string_var
            ));
            pre.push(format!(
                "    memcpy({}, lean_string_cstr(ffi_elem), {} + 1);",
                string_var, bytes_var
            ));
            pre.push(format!("    {}[{}] = {};", data_var, index_var, string_var));

            post.push(format!(
                "for (size_t {} = 0; {} < {}; ++{}) {{",
                index_var, index_var, len_var, index_var
            ));
            post.push(format!("    free({}[{}]);", data_var, index_var));
            post.push("}".to_string());
        }
        _ => {
            pre.push(format!(
                "    lean_object * ffi_elem = lean_array_get_core({}, {});",
                lean_expr, index_var
            ));
            let prepared = prepare_default_parameter_value(
                lean_ctx,
                c_ctx,
                registry,
                name_gen,
                "ffi_elem",
                &element_ty,
                false,
            )?;
            for line in prepared.pre {
                pre.push(format!("    {}", line));
            }
            pre.push(format!("    {}[{}] = {};", data_var, index_var, prepared.expr));
            for line in prepared.post {
                pre.push(format!("    {}", line));
            }
        }
    }
    pre.push("}".to_string());
    post.push(format!("free({});", data_var));

    Ok(PreparedValue {
        pre,
        expr: data_var,
        post,
        length_expr: Some(len_var),
    })
}

fn prepare_default_parameter_value(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    name_gen: &mut NameGen,
    lean_expr: &str,
    ty: &CType,
    top_level_adapter_param: bool,
) -> Result<PreparedValue, String> {
    match registry.resolve_alias_type(ty) {
        CType::Bool
        | CType::UChar
        | CType::UShort
        | CType::UInt
        | CType::ULong
        | CType::ULongLong
        | CType::SizeT => Ok(PreparedValue {
            pre: Vec::new(),
            expr: format!("({})lean_uint64_of_nat({})", render_c_type(ty), lean_expr),
            post: Vec::new(),
            length_expr: None,
        }),
        CType::Char
        | CType::Short
        | CType::Int
        | CType::Long
        | CType::LongLong
        | CType::PtrdiffT => Ok(PreparedValue {
            pre: Vec::new(),
            expr: format!("({})lean_int64_of_int({})", render_c_type(ty), lean_expr),
            post: Vec::new(),
            length_expr: None,
        }),
        CType::Float | CType::Double | CType::LongDouble => Ok(PreparedValue {
            pre: Vec::new(),
            expr: if top_level_adapter_param {
                format!("({}){}", render_c_type(ty), lean_expr)
            } else {
                format!("({})lean_unbox_float({})", render_c_type(ty), lean_expr)
            },
            post: Vec::new(),
            length_expr: None,
        }),
        CType::Enum(_) => prepare_enum_from_lean(lean_ctx, c_ctx, registry, name_gen, lean_expr, ty),
        CType::Pointer { .. }
        | CType::IncompleteArray { .. }
        | CType::Array { size: None, .. } => Ok(PreparedValue {
            pre: Vec::new(),
            expr: format!("({})lean_unbox_usize({})", pointer_cast_type(ty, registry)?, lean_expr),
            post: Vec::new(),
            length_expr: None,
        }),
        CType::Struct(_) => prepare_struct_from_lean(lean_ctx, c_ctx, registry, name_gen, lean_expr, ty),
        CType::Array {
            ref element,
            size: Some(size),
        } => prepare_static_array_from_lean(
            lean_ctx,
            c_ctx,
            registry,
            name_gen,
            lean_expr,
            element,
            size,
            None,
        ),
        CType::Void => Err("void parameters are unsupported".to_string()),
        CType::Union(name) => Err(format!("union {} parameters are unsupported", name)),
        CType::FunctionPointer { .. } => Err("function pointer parameters are unsupported".to_string()),
        CType::Unknown(name) => Err(format!("unsupported parameter type {}", name)),
        CType::Typedef(_) => Err("unresolved typedef parameter is unsupported".to_string()),
    }
}

fn prepare_enum_from_lean(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    name_gen: &mut NameGen,
    lean_expr: &str,
    ty: &CType,
) -> Result<PreparedValue, String> {
    let enum_name = ensure_enum_decl(lean_ctx, c_ctx, registry, ty)?;
    let arg_var = name_gen.next("enum_arg");
    let int_var = name_gen.next("enum_int");
    Ok(PreparedValue {
        pre: vec![
            format!("lean_object * {} = {};", arg_var, lean_expr),
            format!("lean_inc({});", arg_var),
            format!("lean_obj_res {} = ffi_from_{}({});", int_var, enum_name, arg_var),
        ],
        expr: format!("({})lean_scalar_to_int64({})", render_c_type(ty), int_var),
        post: vec![format!("lean_dec({});", int_var)],
        length_expr: None,
    })
}

fn prepare_struct_from_lean(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    name_gen: &mut NameGen,
    lean_expr: &str,
    ty: &CType,
) -> Result<PreparedValue, String> {
    let (lean_name, fields) = registry
        .struct_info(ty)
        .ok_or_else(|| "struct metadata is not available".to_string())?;
    ensure_struct_decl(lean_ctx, c_ctx, registry, ty)?;

    let struct_var = name_gen.next("struct_value");
    let mut pre = vec![format!("{} {} = {{0}};", render_c_type(ty), struct_var)];

    for field in fields {
        if field.name.is_empty() {
            return Err(format!("struct {} contains an unnamed field", lean_name));
        }
        let getter_name = struct_getter_name(&lean_name, &field.name);
        let arg_var = name_gen.next("struct_arg");
        let field_var = name_gen.next("field_value");
        let field_is_float = is_lean_float_type(&field.ty, registry);
        pre.push(format!("lean_object * {} = {};", arg_var, lean_expr));
        pre.push(format!("lean_inc({});", arg_var));
        pre.push(format!(
            "{} {} = {}({});",
            if field_is_float { "double" } else { "lean_obj_res" },
            field_var,
            getter_name,
            arg_var
        ));

        match registry.resolve_alias_type(&field.ty) {
            CType::Array {
                ref element,
                size: Some(size),
            } => {
                let assignment = prepare_static_array_from_lean(
                    lean_ctx,
                    c_ctx,
                    registry,
                    name_gen,
                    &field_var,
                    element,
                    size,
                    Some(format!("{}.{}", struct_var, field.name)),
                )?;
                pre.extend(assignment.pre);
                if !field_is_float {
                    pre.push(format!("lean_dec({});", field_var));
                }
            }
            _ => {
                let assignment = prepare_default_parameter_value(
                    lean_ctx,
                    c_ctx,
                    registry,
                    name_gen,
                    &field_var,
                    &field.ty,
                    field_is_float,
                )?;
                pre.extend(assignment.pre);
                pre.push(format!("{}.{} = {};", struct_var, field.name, assignment.expr));
                pre.extend(assignment.post);
                if !field_is_float {
                    pre.push(format!("lean_dec({});", field_var));
                }
            }
        }
    }

    Ok(PreparedValue {
        pre,
        expr: struct_var,
        post: Vec::new(),
        length_expr: None,
    })
}

fn prepare_static_array_from_lean(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    name_gen: &mut NameGen,
    lean_expr: &str,
    element_ty: &CType,
    size: usize,
    target_expr: Option<String>,
) -> Result<PreparedValue, String> {
    if size == 0 {
        return Err("zero-sized arrays are not supported".to_string());
    }

    let array_var = target_expr.unwrap_or_else(|| name_gen.next("static_array"));
    let len_var = name_gen.next("array_len");
    let limit_var = name_gen.next("array_limit");
    let index_var = name_gen.next("i");
    let mut pre = Vec::new();

    if !array_var.contains('.') {
        pre.push(format!(
            "{} = {{0}};",
            render_array_declaration(element_ty, &array_var, size)
        ));
    }
    pre.push(format!("size_t {} = lean_array_size({});", len_var, lean_expr));
    pre.push(format!(
        "size_t {} = {} < {} ? {} : {};",
        limit_var, len_var, size, len_var, size
    ));
    pre.push(format!("for (size_t {} = 0; {} < {}; ++{}) {{", index_var, index_var, limit_var, index_var));
    pre.push(format!(
        "    lean_object * ffi_elem = lean_array_get_core({}, {});",
        lean_expr, index_var
    ));
    let prepared = prepare_default_parameter_value(
        lean_ctx,
        c_ctx,
        registry,
        name_gen,
        "ffi_elem",
        element_ty,
        false,
    )?;
    for line in prepared.pre {
        pre.push(format!("    {}", line));
    }
    pre.push(format!("    {}[{}] = {};", array_var, index_var, prepared.expr));
    for line in prepared.post {
        pre.push(format!("    {}", line));
    }
    pre.push("}".to_string());

    Ok(PreparedValue {
        pre,
        expr: array_var,
        post: Vec::new(),
        length_expr: None,
    })
}

fn prepare_return_value(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    name_gen: &mut NameGen,
    c_expr: Option<&str>,
    ty: &CType,
    strategy: Option<&ReturnValueSpecialConversion>,
    box_float_return: bool,
) -> Result<PreparedValue, String> {
    if c_expr.is_none() {
        return Ok(PreparedValue {
            pre: Vec::new(),
            expr: "lean_box(0)".to_string(),
            post: Vec::new(),
            length_expr: None,
        });
    }

    if let Some(ReturnValueSpecialConversion::String { free }) = strategy {
        if !registry.is_char_pointer_like(ty) {
            return Err("string return conversion requires a char* return type".to_string());
        }
        let result_var = name_gen.next("lean_result");
        let mut post = Vec::new();
        if *free {
            post.push(format!("free((void *){});", c_expr.unwrap()));
        }
        return Ok(PreparedValue {
            pre: vec![format!(
                "lean_obj_res {} = lean_mk_string({} == NULL ? \"\" : {});",
                result_var,
                c_expr.unwrap(),
                c_expr.unwrap()
            )],
            expr: result_var,
            post,
            length_expr: None,
        });
    }

    if let Some(ReturnValueSpecialConversion::NullTerminatedArray {
        element_conversion,
        free_array_after_conversion,
    }) = strategy
    {
        let element_ty = registry
            .pointer_element_type(ty)
            .ok_or_else(|| "null-terminated array return conversion requires a pointer-to-pointer return type".to_string())?;

        if !registry.is_pointer_like(&element_ty) {
            return Err("null-terminated array return conversion requires a pointer-to-pointer return type".to_string());
        }

        let result_var = name_gen.next("lean_array");
        let len_var = name_gen.next("array_len");
        let index_var = name_gen.next("i");
        let element_var = name_gen.next("array_elem");
        let element_c_ty = render_c_type(&element_ty);
        let mut pre = vec![format!("size_t {} = 0;", len_var)];
        pre.push(format!("while ({} != NULL && {}[{}] != NULL) {{", c_expr.unwrap(), c_expr.unwrap(), len_var));
        pre.push(format!("    ++{};", len_var));
        pre.push("}".to_string());
        pre.push(format!(
            "lean_obj_res {} = lean_mk_empty_array_with_capacity(lean_box({}));",
            result_var, len_var
        ));
        pre.push(format!("for (size_t {} = 0; {} < {}; ++{}) {{", index_var, index_var, len_var, index_var));
        pre.push(format!("    {} {} = {}[{}];", element_c_ty, element_var, c_expr.unwrap(), index_var));

        let prepared = prepare_return_value(
            lean_ctx,
            c_ctx,
            registry,
            name_gen,
            Some(&element_var),
            &element_ty,
            element_conversion.as_deref(),
            true,
        )?;

        pre.extend(prepared.pre.into_iter().map(|line| format!("    {}", line)));
        pre.push(format!("    {} = lean_array_push({}, {});", result_var, result_var, prepared.expr));
        pre.extend(prepared.post.into_iter().map(|line| format!("    {}", line)));
        pre.push("}".to_string());

        let mut post = Vec::new();
        if *free_array_after_conversion {
            post.push(format!("free((void *){});", c_expr.unwrap()));
        }

        return Ok(PreparedValue {
            pre,
            expr: result_var,
            post,
            length_expr: None,
        });
    }

    prepare_default_return_value(
        lean_ctx,
        c_ctx,
        registry,
        name_gen,
        c_expr.unwrap(),
        ty,
        box_float_return,
    )
}

fn prepare_default_return_value(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    name_gen: &mut NameGen,
    c_expr: &str,
    ty: &CType,
    box_float_return: bool,
) -> Result<PreparedValue, String> {
    match registry.resolve_alias_type(ty) {
        CType::Void => Ok(PreparedValue {
            pre: Vec::new(),
            expr: "lean_box(0)".to_string(),
            post: Vec::new(),
            length_expr: None,
        }),
        CType::Bool
        | CType::UChar
        | CType::UShort
        | CType::UInt
        | CType::ULong
        | CType::ULongLong
        | CType::SizeT => Ok(PreparedValue {
            pre: Vec::new(),
            expr: format!("lean_uint64_to_nat((uint64_t)({}))", c_expr),
            post: Vec::new(),
            length_expr: None,
        }),
        CType::Char
        | CType::Short
        | CType::Int
        | CType::Long
        | CType::LongLong
        | CType::PtrdiffT => Ok(PreparedValue {
            pre: Vec::new(),
            expr: format!("lean_int64_to_int((int64_t)({}))", c_expr),
            post: Vec::new(),
            length_expr: None,
        }),
        CType::Float | CType::Double | CType::LongDouble => Ok(PreparedValue {
            pre: Vec::new(),
            expr: if box_float_return {
                format!("lean_box_float((double)({}))", c_expr)
            } else {
                format!("(double)({})", c_expr)
            },
            post: Vec::new(),
            length_expr: None,
        }),
        CType::Pointer { .. }
        | CType::IncompleteArray { .. }
        | CType::Array { size: None, .. } => Ok(PreparedValue {
            pre: Vec::new(),
            expr: format!("lean_box_usize((size_t)({}))", c_expr),
            post: Vec::new(),
            length_expr: None,
        }),
        CType::Enum(_) => prepare_enum_return(lean_ctx, c_ctx, registry, name_gen, c_expr, ty),
        CType::Struct(_) => prepare_struct_return(lean_ctx, c_ctx, registry, name_gen, c_expr, ty),
        CType::Array {
            ref element,
            size: Some(size),
        } => prepare_static_array_return(lean_ctx, c_ctx, registry, name_gen, c_expr, element, size),
        CType::Union(name) => Err(format!("union {} returns are unsupported", name)),
        CType::FunctionPointer { .. } => Err("function pointer returns are unsupported".to_string()),
        CType::Unknown(name) => Err(format!("unsupported return type {}", name)),
        CType::Typedef(_) => Err("unresolved typedef return is unsupported".to_string()),
    }
}

fn prepare_enum_return(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    name_gen: &mut NameGen,
    c_expr: &str,
    ty: &CType,
) -> Result<PreparedValue, String> {
    let enum_name = ensure_enum_decl(lean_ctx, c_ctx, registry, ty)?;
    let int_var = name_gen.next("enum_value");
    let result_var = name_gen.next("lean_result");
    Ok(PreparedValue {
        pre: vec![
            format!(
                "lean_obj_res {} = lean_int64_to_int((int64_t)({}));",
                int_var, c_expr
            ),
            format!("lean_obj_res {} = ffi_to_{}({});", result_var, enum_name, int_var),
        ],
        expr: result_var,
        post: Vec::new(),
        length_expr: None,
    })
}

fn prepare_struct_return(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    name_gen: &mut NameGen,
    c_expr: &str,
    ty: &CType,
) -> Result<PreparedValue, String> {
    let (lean_name, fields) = registry
        .struct_info(ty)
        .ok_or_else(|| "struct metadata is not available".to_string())?;
    ensure_struct_decl(lean_ctx, c_ctx, registry, ty)?;

    let mut pre = Vec::new();
    let mut field_vars = Vec::with_capacity(fields.len());
    for field in fields {
        if field.name.is_empty() {
            return Err(format!("struct {} contains an unnamed field", lean_name));
        }

        let prepared = prepare_default_return_value(
            lean_ctx,
            c_ctx,
            registry,
            name_gen,
            &format!("({}).{}", c_expr, field.name),
            &field.ty,
            !is_lean_float_type(&field.ty, registry),
        )?;
        pre.extend(prepared.pre);
        pre.extend(prepared.post);
        field_vars.push(prepared.expr);
    }

    let result_var = name_gen.next("lean_struct");
    let call = if field_vars.is_empty() {
        format!("ffi_to_{}()", lean_name)
    } else {
        format!("ffi_to_{}({})", lean_name, field_vars.join(", "))
    };
    pre.push(format!("lean_obj_res {} = {};", result_var, call));

    Ok(PreparedValue {
        pre,
        expr: result_var,
        post: Vec::new(),
        length_expr: None,
    })
}

fn prepare_static_array_return(
    lean_ctx: &mut LeanContext,
    c_ctx: &mut CContext,
    registry: &TypeRegistry,
    name_gen: &mut NameGen,
    c_expr: &str,
    element_ty: &CType,
    size: usize,
) -> Result<PreparedValue, String> {
    if size == 0 {
        return Err("zero-sized arrays are not supported".to_string());
    }

    let result_var = name_gen.next("lean_array");
    let index_var = name_gen.next("i");
    let mut pre = vec![format!(
        "lean_obj_res {} = lean_mk_empty_array_with_capacity(lean_box({}));",
        result_var, size
    )];

    for index in 0..size {
        let prepared = prepare_default_return_value(
            lean_ctx,
            c_ctx,
            registry,
            name_gen,
            &format!("({})[{}]", c_expr, index),
            element_ty,
            true,
        )?;
        pre.extend(prepared.pre);
        pre.extend(prepared.post);
        pre.push(format!("{} = lean_array_push({}, {});", result_var, result_var, prepared.expr));
    }

    let _ = index_var;
    Ok(PreparedValue {
        pre,
        expr: result_var,
        post: Vec::new(),
        length_expr: None,
    })
}

fn pointer_cast_type(ty: &CType, registry: &TypeRegistry) -> Result<String, String> {
    match registry.resolve_alias_type(ty) {
        CType::Pointer { .. } => Ok(render_c_type(ty)),
        CType::IncompleteArray { element } => Ok(format!("{}*", render_c_type(&element))),
        CType::Array { element, size: None } => Ok(format!("{}*", render_c_type(&element))),
        _ => Err("type is not pointer-like".to_string()),
    }
}

fn normalize_nested_strategy(
    strategy: Option<&ParameterSpecialConversion>,
) -> Option<&ParameterSpecialConversion> {
    match strategy {
        Some(ParameterSpecialConversion::Out { .. }) => None,
        other => other,
    }
}

fn combine_lean_return_type(base_ty: &str, omit_base: bool, out_types: &[String]) -> String {
    let mut components = Vec::with_capacity(out_types.len() + usize::from(!omit_base));
    if !omit_base {
        components.push(base_ty.to_string());
    }
    components.extend(out_types.iter().cloned());

    match components.as_slice() {
        [] => "Unit".to_string(),
        [only] => only.clone(),
        _ => format!("({})", components.join(" × ")),
    }
}

fn parenthesize_lean_type(ty: &str) -> String {
    if ty.contains(' ') && !(ty.starts_with('(') && ty.ends_with(')')) {
        format!("({})", ty)
    } else {
        ty.to_string()
    }
}

fn ensure_out_stack_type_supported(ty: &CType) -> Result<(), String> {
    match ty {
        CType::Void => Err("out conversion does not support void pointees".to_string()),
        CType::IncompleteArray { .. } => Err("out conversion does not support incomplete-array pointees".to_string()),
        CType::Array { size: None, .. } => Err("out conversion does not support unsized-array pointees".to_string()),
        CType::FunctionPointer { .. } => Err("out conversion does not support function-pointer pointees".to_string()),
        _ => Ok(()),
    }
}

fn enum_variants(variants: &[CEnumVariant]) -> Vec<(String, i64)> {
    let mut next_value = 0i64;
    let mut used_names = HashMap::<String, usize>::new();

    variants
        .iter()
        .map(|variant| {
            let value = variant.value.unwrap_or(next_value);
            next_value = value.saturating_add(1);

            let base = sanitize_lean_ctor_name(&variant.name);
            let count = used_names.entry(base.clone()).or_insert(0);
            let name = if *count == 0 {
                base
            } else {
                format!("{}_{}", base, count)
            };
            *count += 1;

            (name, value)
        })
        .collect()
}

fn struct_getter_name(struct_name: &str, field_name: &str) -> String {
    format!(
        "ffi_get_{}_{}",
        sanitize_c_ident(field_name),
        sanitize_c_ident(struct_name)
    )
}

fn render_c_type(ty: &CType) -> String {
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

fn render_array_declaration(element_ty: &CType, name: &str, size: usize) -> String {
    format!("{} {}[{}]", render_c_type(element_ty), name, size)
}

fn render_zero_initialized_declaration(ty: &CType, name: &str) -> String {
    match ty {
        CType::Array {
            element,
            size: Some(size),
        } => format!("{} = {{0}}", render_array_declaration(element, name, *size)),
        _ => format!("{} {} = {{0}}", render_c_type(ty), name),
    }
}

fn sanitize_c_ident(name: &str) -> String {
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

fn sanitize_lean_type_name(name: &str) -> String {
    let mut sanitized = sanitize_c_ident(name);
    if is_lean_keyword(&sanitized) {
        sanitized.insert_str(0, "Ffi");
    }
    sanitized
}

fn sanitize_lean_field_name(name: &str) -> String {
    let mut sanitized = sanitize_c_ident(name);
    if is_lean_keyword(&sanitized) {
        sanitized.insert_str(0, "ffi_");
    }
    sanitized
}

fn sanitize_lean_ctor_name(name: &str) -> String {
    let mut sanitized = sanitize_lean_field_name(name);
    if sanitized == "other" {
        sanitized = "other_".to_string();
    }
    sanitized
}

fn is_lean_keyword(name: &str) -> bool {
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

fn can_be_no_io(function: &CFunction, registry: &TypeRegistry) -> bool {
    function
        .parameters
        .iter()
        .all(|parameter| is_no_io_primitive(&registry.resolve_alias_type(&parameter.ty)))
        && is_no_io_primitive(&registry.resolve_alias_type(&function.return_type))
}

fn is_no_io_primitive(ty: &CType) -> bool {
    matches!(
        ty,
        CType::Bool
            | CType::Char
            | CType::UChar
            | CType::Short
            | CType::UShort
            | CType::Int
            | CType::UInt
            | CType::Long
            | CType::ULong
            | CType::LongLong
            | CType::ULongLong
            | CType::Float
            | CType::Double
            | CType::LongDouble
            | CType::SizeT
            | CType::PtrdiffT
            | CType::Enum(_)
    )
}
