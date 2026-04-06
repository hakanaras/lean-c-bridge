use crossterm::event::{KeyCode, KeyEvent};

use crate::clang::types::{CFunction, CType};
use crate::generator::TypeRegistry;
use crate::generator::c_render::render_c_type;
use crate::options::interface_choices::*;

pub enum View {
    FunctionList,
    FunctionForm,
}

pub struct App {
    pub view: View,
    pub functions: Vec<CFunction>,
    pub registry: TypeRegistry,
    pub choices: InterfaceChoices,
    pub should_quit: bool,

    // Function list
    pub list_selected: usize,
    pub list_search_active: bool,
    pub list_search_buffer: Vec<char>,
    pub list_search_cursor: usize,
    pub list_search_status: Option<String>,

    // Form
    pub form_function_index: usize,
    pub form_choices: FunctionChoices,
    pub form_items: Vec<FormItem>,
    pub form_focus: usize,
    pub form_scroll: usize,
    pub editing_text: bool,
    pub text_buffer: Vec<char>,
    pub text_cursor: usize,
}

pub struct FormItem {
    pub label: String,
    pub kind: FormItemKind,
    pub path: FormPath,
    pub indent: u16,
}

pub enum FormItemKind {
    Header,
    Checkbox {
        checked: bool,
        enabled: bool,
    },
    Selector {
        options: Vec<String>,
        selected: usize,
        enabled: bool,
    },
    TextInput {
        value: String,
        enabled: bool,
    },
}

#[derive(Clone, PartialEq)]
pub enum ReturnEditorTarget {
    Return,
    ReturnElement,
    ParamOut(usize),
    ParamOutElement(usize),
}

#[derive(Clone, PartialEq)]
pub enum FormPath {
    None,
    Omit,
    NoIo,
    ParamConversion(usize),
    ParamReferenceNullable(usize),
    ParamStringNullable(usize),
    ParamStringBufferSize(usize),
    ParamArrayNullable(usize),
    ParamElementConversion(usize),
    ParamLengthOf(usize),
    ParamStaticExpr(usize),
    ParamStaticPreStmt(usize),
    ParamStaticPostStmt(usize),
    ReturnConversionSelector(ReturnEditorTarget),
    ReturnNullable(ReturnEditorTarget),
    ReturnFree(ReturnEditorTarget),
    ReturnFreeFunction(ReturnEditorTarget),
    ReturnLengthOf(ReturnEditorTarget),
    ReturnElementConversion(ReturnEditorTarget),
}

impl App {
    pub fn new(
        choices: InterfaceChoices,
        functions: Vec<CFunction>,
        registry: TypeRegistry,
    ) -> Self {
        Self {
            view: View::FunctionList,
            functions,
            registry,
            choices,
            should_quit: false,
            list_selected: 0,
            list_search_active: false,
            list_search_buffer: Vec::new(),
            list_search_cursor: 0,
            list_search_status: None,
            form_function_index: 0,
            form_choices: FunctionChoices {
                name: String::new(),
                omit: false,
                no_io: false,
                parameters: vec![],
                return_value: None,
            },
            form_items: vec![],
            form_focus: 0,
            form_scroll: 0,
            editing_text: false,
            text_buffer: Vec::new(),
            text_cursor: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.view {
            View::FunctionList => self.handle_list_key(key),
            View::FunctionForm => {
                if self.editing_text {
                    self.handle_text_edit_key(key);
                } else {
                    self.handle_form_key(key);
                }
            }
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) {
        if self.list_search_active {
            self.handle_list_search_key(key);
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Up => {
                if self.list_selected > 0 {
                    self.list_selected -= 1;
                }
            }
            KeyCode::Down => {
                if self.list_selected + 1 < self.functions.len() {
                    self.list_selected += 1;
                }
            }
            KeyCode::PageUp => {
                self.list_selected = self.list_selected.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.list_selected =
                    (self.list_selected + 10).min(self.functions.len().saturating_sub(1));
            }
            KeyCode::Home => {
                self.list_selected = 0;
            }
            KeyCode::End => {
                self.list_selected = self.functions.len().saturating_sub(1);
            }
            KeyCode::Enter => {
                if !self.functions.is_empty() {
                    self.enter_form();
                }
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.start_list_search();
            }
            _ => {}
        }
    }

    fn handle_list_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.clear_list_search();
            }
            KeyCode::Enter => {
                if self.list_search_buffer.is_empty() || self.apply_list_search_selection() {
                    self.clear_list_search();
                } else {
                    self.list_search_status = Some(format!(
                        "No function matches \"{}\"",
                        self.list_search_query()
                    ));
                }
            }
            KeyCode::Backspace => {
                if self.list_search_cursor > 0 {
                    self.list_search_cursor -= 1;
                    self.list_search_buffer.remove(self.list_search_cursor);
                    self.refresh_list_search();
                }
            }
            KeyCode::Delete => {
                if self.list_search_cursor < self.list_search_buffer.len() {
                    self.list_search_buffer.remove(self.list_search_cursor);
                    self.refresh_list_search();
                }
            }
            KeyCode::Left => {
                if self.list_search_cursor > 0 {
                    self.list_search_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.list_search_cursor < self.list_search_buffer.len() {
                    self.list_search_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.list_search_cursor = 0;
            }
            KeyCode::End => {
                self.list_search_cursor = self.list_search_buffer.len();
            }
            KeyCode::Char(c) => {
                self.list_search_buffer.insert(self.list_search_cursor, c);
                self.list_search_cursor += 1;
                self.refresh_list_search();
            }
            _ => {}
        }
    }

    fn start_list_search(&mut self) {
        self.list_search_active = true;
        self.list_search_buffer.clear();
        self.list_search_cursor = 0;
        self.list_search_status = None;
    }

    fn clear_list_search(&mut self) {
        self.list_search_active = false;
        self.list_search_buffer.clear();
        self.list_search_cursor = 0;
        self.list_search_status = None;
    }

    fn refresh_list_search(&mut self) {
        if self.list_search_buffer.is_empty() {
            self.list_search_status = None;
            return;
        }

        if self.apply_list_search_selection() {
            self.list_search_status = None;
        } else {
            self.list_search_status = Some(format!(
                "No function matches \"{}\"",
                self.list_search_query()
            ));
        }
    }

    fn apply_list_search_selection(&mut self) -> bool {
        let query = self.list_search_query();
        let query = query.trim();
        if query.is_empty() {
            return false;
        }

        let needle = query.to_ascii_lowercase();
        if let Some(index) = self
            .functions
            .iter()
            .position(|function| function.name.to_ascii_lowercase().contains(&needle))
        {
            self.list_selected = index;
            true
        } else {
            false
        }
    }

    pub fn list_search_query(&self) -> String {
        self.list_search_buffer.iter().collect()
    }

    fn enter_form(&mut self) {
        self.form_function_index = self.list_selected;
        let func = &self.functions[self.form_function_index];

        self.form_choices = self
            .choices
            .functions
            .iter()
            .find(|c| c.name == func.name)
            .cloned()
            .unwrap_or_else(|| FunctionChoices {
                name: func.name.clone(),
                omit: false,
                no_io: false,
                parameters: func
                    .parameters
                    .iter()
                    .map(|_| ParameterChoices {
                        conversion_strategy: None,
                    })
                    .collect(),
                return_value: None,
            });

        self.form_focus = 0;
        self.form_scroll = 0;
        self.rebuild_form();
        self.view = View::FunctionForm;
    }

    fn save_form_and_go_back(&mut self) {
        let name = self.form_choices.name.clone();
        self.choices.functions.retain(|c| c.name != name);
        self.choices.functions.push(self.form_choices.clone());
        self.view = View::FunctionList;
    }

    pub fn rebuild_form(&mut self) {
        let func = self.functions[self.form_function_index].clone();
        let choices = &self.form_choices;
        let omit = choices.omit;

        let mut items = Vec::new();

        // Omit checkbox
        items.push(FormItem {
            label: "Omit".to_string(),
            kind: FormItemKind::Checkbox {
                checked: choices.omit,
                enabled: true,
            },
            path: FormPath::Omit,
            indent: 0,
        });

        // No IO checkbox
        let can_no_io = can_be_no_io(&func);
        items.push(FormItem {
            label: if can_no_io {
                "No IO".to_string()
            } else {
                "No IO (not applicable)".to_string()
            },
            kind: FormItemKind::Checkbox {
                checked: choices.no_io,
                enabled: can_no_io && !omit,
            },
            path: FormPath::NoIo,
            indent: 0,
        });

        // Parameters
        for (i, param) in func.parameters.iter().enumerate() {
            let param_name = param.name.as_deref().unwrap_or("?");
            let type_str = render_c_type(&param.ty);

            items.push(FormItem {
                label: format!("Parameter {}: {} ({})", i, param_name, type_str),
                kind: FormItemKind::Header,
                path: FormPath::None,
                indent: 0,
            });

            let conversion_options = param_conversion_options(&param.ty, i, &choices.parameters);
            let param_choices = choices.parameters.get(i);
            let selected = param_choices
                .and_then(|pc| pc.conversion_strategy.as_ref())
                .map(|cs| conversion_to_index(cs, &conversion_options))
                .unwrap_or(0);

            items.push(FormItem {
                label: "Conversion".to_string(),
                kind: FormItemKind::Selector {
                    options: conversion_options,
                    selected,
                    enabled: !omit,
                },
                path: FormPath::ParamConversion(i),
                indent: 1,
            });

            // Additional fields based on selected conversion
            if let Some(pc) = param_choices {
                if let Some(ref cs) = pc.conversion_strategy {
                    match cs {
                        ParameterSpecialConversion::Reference {
                            nullable,
                            element_conversion,
                        } => {
                            items.push(FormItem {
                                label: "Nullable".to_string(),
                                kind: FormItemKind::Checkbox {
                                    checked: *nullable,
                                    enabled: !omit,
                                },
                                path: FormPath::ParamReferenceNullable(i),
                                indent: 2,
                            });

                            let elem_options =
                                reference_element_conversion_options(get_pointee(&param.ty));
                            let elem_selected = element_conversion
                                .as_ref()
                                .map(|ec| elem_conversion_to_index(ec, &elem_options))
                                .unwrap_or(0);

                            items.push(FormItem {
                                label: "Pointed value conversion".to_string(),
                                kind: FormItemKind::Selector {
                                    options: elem_options,
                                    selected: elem_selected,
                                    enabled: !omit,
                                },
                                path: FormPath::ParamElementConversion(i),
                                indent: 2,
                            });
                        }
                        ParameterSpecialConversion::String { nullable } => {
                            items.push(FormItem {
                                label: "Nullable".to_string(),
                                kind: FormItemKind::Checkbox {
                                    checked: *nullable,
                                    enabled: !omit,
                                },
                                path: FormPath::ParamStringNullable(i),
                                indent: 2,
                            });
                        }
                        ParameterSpecialConversion::StringBuffer { buffer_size } => {
                            items.push(FormItem {
                                label: "Buffer size".to_string(),
                                kind: FormItemKind::TextInput {
                                    value: buffer_size.to_string(),
                                    enabled: !omit,
                                },
                                path: FormPath::ParamStringBufferSize(i),
                                indent: 2,
                            });
                        }
                        ParameterSpecialConversion::Array {
                            nullable,
                            element_conversion,
                        } => {
                            items.push(FormItem {
                                label: "Nullable".to_string(),
                                kind: FormItemKind::Checkbox {
                                    checked: *nullable,
                                    enabled: !omit,
                                },
                                path: FormPath::ParamArrayNullable(i),
                                indent: 2,
                            });

                            let elem_options = element_conversion_options(get_pointee(&param.ty));
                            let elem_selected = element_conversion
                                .as_ref()
                                .map(|ec| elem_conversion_to_index(ec, &elem_options))
                                .unwrap_or(0);

                            items.push(FormItem {
                                label: "Element conversion".to_string(),
                                kind: FormItemKind::Selector {
                                    options: elem_options,
                                    selected: elem_selected,
                                    enabled: !omit,
                                },
                                path: FormPath::ParamElementConversion(i),
                                indent: 2,
                            });
                        }
                        ParameterSpecialConversion::Out { element_conversion } => {
                            let out_target = ReturnEditorTarget::ParamOut(i);
                            let out_ty = get_pointee(&param.ty);
                            let out_options =
                                return_conversion_options(out_ty, choices, &out_target, true);
                            let out_selected = element_conversion
                                .as_deref()
                                .map(|conversion| {
                                    return_conversion_to_index(conversion, &out_options)
                                })
                                .unwrap_or(0);

                            items.push(FormItem {
                                label: "Pointed value conversion".to_string(),
                                kind: FormItemKind::Selector {
                                    options: out_options,
                                    selected: out_selected,
                                    enabled: !omit,
                                },
                                path: FormPath::ReturnConversionSelector(out_target.clone()),
                                indent: 2,
                            });

                            if let Some(conversion) = element_conversion.as_deref() {
                                push_return_conversion_items(
                                    &mut items, choices, conversion, out_ty, out_target, !omit, 3,
                                    true,
                                );
                            }
                        }
                        ParameterSpecialConversion::Length { of_param_index } => {
                            let eligible: Vec<usize> =
                                eligible_length_targets(i, &choices.parameters);
                            let display_options: Vec<String> =
                                eligible.iter().map(|j| j.to_string()).collect();
                            let selected_idx = eligible
                                .iter()
                                .position(|j| *j == *of_param_index)
                                .unwrap_or(0);
                            items.push(FormItem {
                                label: "Length of parameter".to_string(),
                                kind: FormItemKind::Selector {
                                    options: display_options,
                                    selected: selected_idx,
                                    enabled: !omit,
                                },
                                path: FormPath::ParamLengthOf(i),
                                indent: 2,
                            });
                        }
                        ParameterSpecialConversion::StaticExpr {
                            pre_statements,
                            expr,
                            post_statements,
                        } => {
                            items.push(FormItem {
                                label: "Expression".to_string(),
                                kind: FormItemKind::TextInput {
                                    value: expr.clone(),
                                    enabled: !omit,
                                },
                                path: FormPath::ParamStaticExpr(i),
                                indent: 2,
                            });
                            items.push(FormItem {
                                label: "Pre-statements".to_string(),
                                kind: FormItemKind::TextInput {
                                    value: pre_statements.join("; "),
                                    enabled: !omit,
                                },
                                path: FormPath::ParamStaticPreStmt(i),
                                indent: 2,
                            });
                            items.push(FormItem {
                                label: "Post-statements".to_string(),
                                kind: FormItemKind::TextInput {
                                    value: post_statements.join("; "),
                                    enabled: !omit,
                                },
                                path: FormPath::ParamStaticPostStmt(i),
                                indent: 2,
                            });
                        }
                    }
                }
            }
        }

        // Return type
        let ret_type_str = render_c_type(&func.return_type);
        items.push(FormItem {
            label: format!("Return ({})", ret_type_str),
            kind: FormItemKind::Header,
            path: FormPath::None,
            indent: 0,
        });

        let return_target = ReturnEditorTarget::Return;
        let ret_options =
            return_conversion_options(Some(&func.return_type), choices, &return_target, true);
        if ret_options.len() > 1 {
            let ret_selected = choices
                .return_value
                .as_ref()
                .map(|conversion| return_conversion_to_index(conversion, &ret_options))
                .unwrap_or(0);

            items.push(FormItem {
                label: "Conversion".to_string(),
                kind: FormItemKind::Selector {
                    options: ret_options,
                    selected: ret_selected,
                    enabled: !omit,
                },
                path: FormPath::ReturnConversionSelector(return_target.clone()),
                indent: 1,
            });

            if let Some(conversion) = &choices.return_value {
                push_return_conversion_items(
                    &mut items,
                    choices,
                    conversion,
                    Some(&func.return_type),
                    return_target,
                    !omit,
                    2,
                    true,
                );
            }
        } else {
            items.push(FormItem {
                label: "(no special conversion available)".to_string(),
                kind: FormItemKind::Header,
                path: FormPath::None,
                indent: 1,
            });
        }

        self.form_items = items;

        // Clamp focus
        if self.form_focus >= self.form_items.len() {
            self.form_focus = self.form_items.len().saturating_sub(1);
        }
        // Skip headers
        self.skip_to_interactive(true);
    }

    fn skip_to_interactive(&mut self, forward: bool) {
        let len = self.form_items.len();
        if len == 0 {
            return;
        }
        let mut attempts = 0;
        while attempts < len {
            if self.form_items[self.form_focus].is_interactive() {
                return;
            }
            if forward {
                if self.form_focus + 1 < len {
                    self.form_focus += 1;
                } else {
                    return;
                }
            } else if self.form_focus > 0 {
                self.form_focus -= 1;
            } else {
                return;
            }
            attempts += 1;
        }
    }

    fn handle_form_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.save_form_and_go_back();
            }
            KeyCode::Up => {
                if self.form_focus > 0 {
                    self.form_focus -= 1;
                    self.skip_to_interactive(false);
                }
            }
            KeyCode::Down => {
                if self.form_focus + 1 < self.form_items.len() {
                    self.form_focus += 1;
                    self.skip_to_interactive(true);
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.activate_form_item();
            }
            KeyCode::Left => {
                self.cycle_selector(false);
            }
            KeyCode::Right => {
                self.cycle_selector(true);
            }
            _ => {}
        }
    }

    fn activate_form_item(&mut self) {
        let item = &self.form_items[self.form_focus];
        match &item.kind {
            FormItemKind::Checkbox {
                enabled, checked, ..
            } => {
                if !enabled {
                    return;
                }
                let new_checked = !checked;
                let path = item.path.clone();
                self.apply_checkbox_change(&path, new_checked);
                self.rebuild_form();
            }
            FormItemKind::Selector {
                options,
                selected,
                enabled,
            } => {
                if !enabled || options.is_empty() {
                    return;
                }
                let new_selected = (selected + 1) % options.len();
                let path = item.path.clone();
                let option = options[new_selected].clone();
                self.apply_selector_change(&path, new_selected, &option);
                self.rebuild_form();
            }
            FormItemKind::TextInput { value, enabled } => {
                if !enabled {
                    return;
                }
                self.editing_text = true;
                self.text_buffer = value.chars().collect();
                self.text_cursor = self.text_buffer.len();
            }
            _ => {}
        }
    }

    fn cycle_selector(&mut self, forward: bool) {
        let item = &self.form_items[self.form_focus];
        if let FormItemKind::Selector {
            options,
            selected,
            enabled,
        } = &item.kind
        {
            if !enabled || options.is_empty() {
                return;
            }
            let new_selected = if forward {
                (selected + 1) % options.len()
            } else if *selected == 0 {
                options.len() - 1
            } else {
                selected - 1
            };
            let path = item.path.clone();
            let option = options[new_selected].clone();
            self.apply_selector_change(&path, new_selected, &option);
            self.rebuild_form();
        }
    }

    fn apply_checkbox_change(&mut self, path: &FormPath, checked: bool) {
        match path {
            FormPath::Omit => {
                self.form_choices.omit = checked;
            }
            FormPath::NoIo => {
                self.form_choices.no_io = checked;
            }
            FormPath::ParamStringNullable(i) => {
                if let Some(ParameterSpecialConversion::String { nullable }) =
                    self.form_choices.parameters[*i]
                        .conversion_strategy
                        .as_mut()
                {
                    *nullable = checked;
                }
            }
            FormPath::ParamReferenceNullable(i) => {
                if let Some(ParameterSpecialConversion::Reference { nullable, .. }) =
                    self.form_choices.parameters[*i]
                        .conversion_strategy
                        .as_mut()
                {
                    *nullable = checked;
                }
            }
            FormPath::ParamArrayNullable(i) => {
                if let Some(ParameterSpecialConversion::Array { nullable, .. }) =
                    self.form_choices.parameters[*i]
                        .conversion_strategy
                        .as_mut()
                {
                    *nullable = checked;
                }
            }
            FormPath::ReturnNullable(target) => {
                if let Some(conversion) = get_return_conversion_mut(&mut self.form_choices, target)
                {
                    match conversion {
                        ReturnValueSpecialConversion::String { nullable, .. }
                        | ReturnValueSpecialConversion::ArrayWithLength { nullable, .. }
                        | ReturnValueSpecialConversion::NullTerminatedArray { nullable, .. }
                        | ReturnValueSpecialConversion::Dereference { nullable, .. } => {
                            *nullable = checked;
                        }
                        ReturnValueSpecialConversion::Length { .. } => {}
                    }
                }
            }
            FormPath::ReturnFree(target) => {
                if let Some(conversion) = get_return_conversion_mut(&mut self.form_choices, target)
                {
                    match conversion {
                        ReturnValueSpecialConversion::String { free, .. }
                        | ReturnValueSpecialConversion::Dereference { free, .. } => {
                            *free = checked;
                        }
                        ReturnValueSpecialConversion::ArrayWithLength {
                            free_array_after_conversion,
                            ..
                        }
                        | ReturnValueSpecialConversion::NullTerminatedArray {
                            free_array_after_conversion,
                            ..
                        } => {
                            *free_array_after_conversion = checked;
                        }
                        ReturnValueSpecialConversion::Length { .. } => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn apply_selector_change(&mut self, path: &FormPath, _index: usize, option: &str) {
        match path {
            FormPath::ParamConversion(i) => {
                let i = *i;
                if i < self.form_choices.parameters.len() {
                    self.form_choices.parameters[i].conversion_strategy = match option {
                        "Reference" => Some(ParameterSpecialConversion::Reference {
                            nullable: false,
                            element_conversion: None,
                        }),
                        "String" => Some(ParameterSpecialConversion::String { nullable: false }),
                        "StringBuffer" => {
                            Some(ParameterSpecialConversion::StringBuffer { buffer_size: 1024 })
                        }
                        "Array" => Some(ParameterSpecialConversion::Array {
                            nullable: false,
                            element_conversion: None,
                        }),
                        "Out" => Some(ParameterSpecialConversion::Out {
                            element_conversion: None,
                        }),
                        "Length" => Some(ParameterSpecialConversion::Length {
                            of_param_index: eligible_length_targets(
                                i,
                                &self.form_choices.parameters,
                            )
                            .into_iter()
                            .next()
                            .unwrap_or(0),
                        }),
                        "StaticExpr" => Some(ParameterSpecialConversion::StaticExpr {
                            pre_statements: vec![],
                            expr: String::new(),
                            post_statements: vec![],
                        }),
                        _ => None,
                    };
                }
            }
            FormPath::ParamElementConversion(i) => {
                let i = *i;
                if i < self.form_choices.parameters.len() {
                    if let Some(ref mut cs) = self.form_choices.parameters[i].conversion_strategy {
                        let new_elem = match option {
                            "Reference" => Some(Box::new(ParameterSpecialConversion::Reference {
                                nullable: false,
                                element_conversion: None,
                            })),
                            "String" => Some(Box::new(ParameterSpecialConversion::String {
                                nullable: false,
                            })),
                            "Array" => Some(Box::new(ParameterSpecialConversion::Array {
                                nullable: false,
                                element_conversion: None,
                            })),
                            _ => None,
                        };
                        match cs {
                            ParameterSpecialConversion::Reference {
                                element_conversion, ..
                            }
                            | ParameterSpecialConversion::Array {
                                element_conversion, ..
                            } => {
                                *element_conversion = new_elem;
                            }
                            _ => {}
                        }
                    }
                }
            }
            FormPath::ParamLengthOf(i) => {
                let i = *i;
                if i < self.form_choices.parameters.len() {
                    if let Some(ParameterSpecialConversion::Length {
                        ref mut of_param_index,
                    }) = self.form_choices.parameters[i].conversion_strategy
                    {
                        *of_param_index = option.parse().unwrap_or(0);
                    }
                }
            }
            FormPath::ReturnConversionSelector(target) => {
                let default_length_target =
                    eligible_return_length_targets(target, &self.form_choices)
                        .first()
                        .copied();
                set_return_conversion(
                    &mut self.form_choices,
                    target,
                    return_conversion_from_option(option, default_length_target),
                );
            }
            FormPath::ReturnLengthOf(target) => {
                if let Some(ReturnValueSpecialConversion::Length { of_param_index }) =
                    get_return_conversion_mut(&mut self.form_choices, target)
                {
                    *of_param_index = if option == "return" {
                        None
                    } else {
                        Some(option.parse().unwrap_or(0))
                    };
                }
            }
            FormPath::ReturnElementConversion(target) => {
                if let Some(conversion) = get_return_conversion_mut(&mut self.form_choices, target)
                {
                    match conversion {
                        ReturnValueSpecialConversion::ArrayWithLength {
                            element_conversion,
                            ..
                        }
                        | ReturnValueSpecialConversion::NullTerminatedArray {
                            element_conversion,
                            ..
                        }
                        | ReturnValueSpecialConversion::Dereference {
                            element_conversion, ..
                        } => {
                            *element_conversion =
                                return_conversion_from_option(option, None).map(Box::new);
                        }
                        ReturnValueSpecialConversion::String { .. }
                        | ReturnValueSpecialConversion::Length { .. } => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_text_edit_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.save_text_edit();
                self.editing_text = false;
            }
            KeyCode::Backspace => {
                if self.text_cursor > 0 {
                    self.text_cursor -= 1;
                    self.text_buffer.remove(self.text_cursor);
                }
            }
            KeyCode::Delete => {
                if self.text_cursor < self.text_buffer.len() {
                    self.text_buffer.remove(self.text_cursor);
                }
            }
            KeyCode::Left => {
                if self.text_cursor > 0 {
                    self.text_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.text_cursor < self.text_buffer.len() {
                    self.text_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.text_cursor = 0;
            }
            KeyCode::End => {
                self.text_cursor = self.text_buffer.len();
            }
            KeyCode::Char(c) => {
                self.text_buffer.insert(self.text_cursor, c);
                self.text_cursor += 1;
            }
            _ => {}
        }
    }

    fn save_text_edit(&mut self) {
        let path = self.form_items[self.form_focus].path.clone();
        let value: String = self.text_buffer.iter().collect();

        match path {
            FormPath::ParamStringBufferSize(i) => {
                if let Some(ParameterSpecialConversion::StringBuffer {
                    ref mut buffer_size,
                }) = self.form_choices.parameters[i].conversion_strategy
                {
                    if let Ok(parsed) = value.trim().parse::<usize>() {
                        if parsed > 0 {
                            *buffer_size = parsed;
                        }
                    }
                }
            }
            FormPath::ParamStaticExpr(i) => {
                if let Some(ParameterSpecialConversion::StaticExpr { ref mut expr, .. }) =
                    self.form_choices.parameters[i].conversion_strategy
                {
                    *expr = value;
                }
            }
            FormPath::ParamStaticPreStmt(i) => {
                if let Some(ParameterSpecialConversion::StaticExpr {
                    ref mut pre_statements,
                    ..
                }) = self.form_choices.parameters[i].conversion_strategy
                {
                    *pre_statements = if value.is_empty() {
                        vec![]
                    } else {
                        value.split("; ").map(String::from).collect()
                    };
                }
            }
            FormPath::ParamStaticPostStmt(i) => {
                if let Some(ParameterSpecialConversion::StaticExpr {
                    ref mut post_statements,
                    ..
                }) = self.form_choices.parameters[i].conversion_strategy
                {
                    *post_statements = if value.is_empty() {
                        vec![]
                    } else {
                        value.split("; ").map(String::from).collect()
                    };
                }
            }
            FormPath::ReturnFreeFunction(target) => {
                if let Some(conversion) = get_return_conversion_mut(&mut self.form_choices, &target)
                {
                    match conversion {
                        ReturnValueSpecialConversion::String { free_function, .. }
                        | ReturnValueSpecialConversion::ArrayWithLength { free_function, .. }
                        | ReturnValueSpecialConversion::NullTerminatedArray {
                            free_function, ..
                        }
                        | ReturnValueSpecialConversion::Dereference { free_function, .. } => {
                            *free_function = optional_function_name(&value);
                        }
                        ReturnValueSpecialConversion::Length { .. } => {}
                    }
                }
            }
            _ => {}
        }

        self.rebuild_form();
    }

    pub fn function_has_choices(&self, name: &str) -> bool {
        self.choices.functions.iter().any(|c| c.name == name)
    }

    pub fn preview_target(&self) -> Option<(&CFunction, FunctionChoices)> {
        match self.view {
            View::FunctionList => {
                let function = self.functions.get(self.list_selected)?;
                Some((function, self.saved_or_default_choices(function)))
            }
            View::FunctionForm => self
                .functions
                .get(self.form_function_index)
                .map(|function| (function, self.form_choices.clone())),
        }
    }

    fn saved_or_default_choices(&self, function: &CFunction) -> FunctionChoices {
        self.choices
            .functions
            .iter()
            .find(|choices| choices.name == function.name)
            .cloned()
            .unwrap_or_else(|| FunctionChoices {
                name: function.name.clone(),
                omit: false,
                no_io: false,
                parameters: function
                    .parameters
                    .iter()
                    .map(|_| ParameterChoices {
                        conversion_strategy: None,
                    })
                    .collect(),
                return_value: None,
            })
    }
}

impl FormItem {
    pub fn is_interactive(&self) -> bool {
        !matches!(self.kind, FormItemKind::Header)
    }
}

// --- Helper functions ---

fn is_char_pointer(ty: &CType) -> bool {
    matches!(ty, CType::Pointer { pointee, .. } if matches!(**pointee, CType::Char))
}

fn is_pointer(ty: &CType) -> bool {
    matches!(ty, CType::Pointer { .. })
}

fn get_pointee(ty: &CType) -> Option<&CType> {
    match ty {
        CType::Pointer { pointee, .. } => Some(pointee),
        _ => None,
    }
}

fn is_primitive(ty: &CType) -> bool {
    matches!(
        ty,
        CType::Void
            | CType::Bool
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

fn can_be_no_io(func: &CFunction) -> bool {
    func.parameters.iter().all(|p| is_primitive(&p.ty))
        && (is_primitive(&func.return_type) || matches!(func.return_type, CType::Void))
}

fn is_integer(ty: &CType) -> bool {
    matches!(
        ty,
        CType::Char
            | CType::UChar
            | CType::Short
            | CType::UShort
            | CType::Int
            | CType::UInt
            | CType::Long
            | CType::ULong
            | CType::LongLong
            | CType::ULongLong
            | CType::SizeT
            | CType::PtrdiffT
    )
}

fn has_collection_conversion(param_index: usize, params: &[ParameterChoices]) -> bool {
    params.iter().enumerate().any(|(j, pc)| {
        j != param_index && strategy_provides_length(pc.conversion_strategy.as_ref())
    })
}

fn eligible_length_targets(param_index: usize, params: &[ParameterChoices]) -> Vec<usize> {
    params
        .iter()
        .enumerate()
        .filter(|(j, pc)| {
            *j != param_index && strategy_provides_length(pc.conversion_strategy.as_ref())
        })
        .map(|(j, _)| j)
        .collect()
}

fn return_strategy_consumes_length(strategy: Option<&ReturnValueSpecialConversion>) -> bool {
    matches!(
        strategy,
        Some(ReturnValueSpecialConversion::String { .. })
            | Some(ReturnValueSpecialConversion::ArrayWithLength { .. })
    )
}

fn parameter_provides_return_length_target(parameter: &ParameterChoices) -> bool {
    match parameter.conversion_strategy.as_ref() {
        Some(ParameterSpecialConversion::StringBuffer { .. }) => true,
        Some(ParameterSpecialConversion::Out { element_conversion }) => {
            return_strategy_consumes_length(element_conversion.as_deref())
        }
        _ => false,
    }
}

fn eligible_return_length_targets(
    target: &ReturnEditorTarget,
    choices: &FunctionChoices,
) -> Vec<Option<usize>> {
    let mut targets = Vec::new();

    if *target != ReturnEditorTarget::Return
        && return_strategy_consumes_length(choices.return_value.as_ref())
    {
        targets.push(None);
    }

    targets.extend(
        choices
            .parameters
            .iter()
            .enumerate()
            .filter(|(index, parameter)| {
                *target != ReturnEditorTarget::ParamOut(*index)
                    && parameter_provides_return_length_target(parameter)
            })
            .map(|(index, _)| Some(index)),
    );

    targets
}

fn return_length_target_label(target: Option<usize>) -> String {
    match target {
        Some(index) => index.to_string(),
        None => "return".to_string(),
    }
}

fn return_length_target_selector_state(
    target: &ReturnEditorTarget,
    choices: &FunctionChoices,
    selected_target: Option<usize>,
) -> (Vec<String>, usize) {
    let eligible = eligible_return_length_targets(target, choices);
    if eligible.is_empty() {
        return (vec![return_length_target_label(selected_target)], 0);
    }

    let options = eligible
        .iter()
        .copied()
        .map(return_length_target_label)
        .collect::<Vec<_>>();
    let selected = eligible
        .iter()
        .position(|candidate| *candidate == selected_target)
        .unwrap_or(0);
    (options, selected)
}

fn param_conversion_options(
    ty: &CType,
    param_index: usize,
    params: &[ParameterChoices],
) -> Vec<String> {
    let mut options = vec!["None".to_string()];
    if is_char_pointer(ty) {
        options.push("String".to_string());
        options.push("StringBuffer".to_string());
    }
    if is_pointer(ty) {
        options.push("Reference".to_string());
        options.push("Array".to_string());
        options.push("Out".to_string());
    }
    if is_integer(ty) && !is_pointer(ty) && has_collection_conversion(param_index, params) {
        options.push("Length".to_string());
    }
    options.push("StaticExpr".to_string());
    options
}

fn element_conversion_options(pointee: Option<&CType>) -> Vec<String> {
    let mut options = vec!["None".to_string()];
    if let Some(ty) = pointee {
        if is_char_pointer(ty) {
            options.push("String".to_string());
        }
    }
    options
}

fn reference_element_conversion_options(pointee: Option<&CType>) -> Vec<String> {
    let mut options = vec!["None".to_string()];
    if let Some(ty) = pointee {
        if is_char_pointer(ty) {
            options.push("String".to_string());
        }
        if is_pointer(ty) {
            options.push("Reference".to_string());
            options.push("Array".to_string());
        }
    }
    options
}

fn strategy_provides_length(strategy: Option<&ParameterSpecialConversion>) -> bool {
    match strategy {
        Some(ParameterSpecialConversion::String { .. })
        | Some(ParameterSpecialConversion::StringBuffer { .. })
        | Some(ParameterSpecialConversion::Array { .. }) => true,
        Some(ParameterSpecialConversion::Reference {
            element_conversion, ..
        }) => strategy_provides_length(element_conversion.as_deref()),
        Some(ParameterSpecialConversion::Out { .. })
        | Some(ParameterSpecialConversion::Length { .. })
        | Some(ParameterSpecialConversion::StaticExpr { .. })
        | None => false,
    }
}

fn return_conversion_options(
    ty: Option<&CType>,
    choices: &FunctionChoices,
    target: &ReturnEditorTarget,
    allow_length_metadata: bool,
) -> Vec<String> {
    let mut options = vec!["None".to_string()];
    if ty.is_some_and(is_char_pointer) {
        options.push("String".to_string());
    }
    if ty.is_some_and(is_pointer) {
        options.push("Dereference".to_string());
        options.push("ArrayWithLength".to_string());
    }
    if ty.is_some_and(is_pointer_to_pointer) {
        options.push("NullTerminatedArray".to_string());
    }
    if allow_length_metadata
        && ty.is_some_and(|value_ty| is_integer(value_ty) && !is_pointer(value_ty))
        && !eligible_return_length_targets(target, choices).is_empty()
    {
        options.push("Length".to_string());
    }
    options
}

fn conversion_to_index(cs: &ParameterSpecialConversion, options: &[String]) -> usize {
    let name = match cs {
        ParameterSpecialConversion::Reference { .. } => "Reference",
        ParameterSpecialConversion::String { .. } => "String",
        ParameterSpecialConversion::StringBuffer { .. } => "StringBuffer",
        ParameterSpecialConversion::Array { .. } => "Array",
        ParameterSpecialConversion::Out { .. } => "Out",
        ParameterSpecialConversion::Length { .. } => "Length",
        ParameterSpecialConversion::StaticExpr { .. } => "StaticExpr",
    };
    options.iter().position(|o| o == name).unwrap_or(0)
}

fn elem_conversion_to_index(cs: &ParameterSpecialConversion, options: &[String]) -> usize {
    let name = match cs {
        ParameterSpecialConversion::Reference { .. } => "Reference",
        ParameterSpecialConversion::String { .. } => "String",
        ParameterSpecialConversion::Array { .. } => "Array",
        _ => "None",
    };
    options.iter().position(|o| o == name).unwrap_or(0)
}

fn return_conversion_to_index(cs: &ReturnValueSpecialConversion, options: &[String]) -> usize {
    let name = match cs {
        ReturnValueSpecialConversion::String { .. } => "String",
        ReturnValueSpecialConversion::Dereference { .. } => "Dereference",
        ReturnValueSpecialConversion::Length { .. } => "Length",
        ReturnValueSpecialConversion::ArrayWithLength { .. } => "ArrayWithLength",
        ReturnValueSpecialConversion::NullTerminatedArray { .. } => "NullTerminatedArray",
    };
    options.iter().position(|o| o == name).unwrap_or(0)
}

fn return_conversion_from_option(
    option: &str,
    default_length_target: Option<Option<usize>>,
) -> Option<ReturnValueSpecialConversion> {
    match option {
        "String" => Some(ReturnValueSpecialConversion::String {
            nullable: false,
            free: false,
            free_function: None,
        }),
        "Dereference" => Some(ReturnValueSpecialConversion::Dereference {
            nullable: false,
            element_conversion: None,
            free: false,
            free_function: None,
        }),
        "Length" => Some(ReturnValueSpecialConversion::Length {
            of_param_index: default_length_target.unwrap_or(None),
        }),
        "ArrayWithLength" => Some(ReturnValueSpecialConversion::ArrayWithLength {
            nullable: false,
            element_conversion: None,
            free_array_after_conversion: false,
            free_function: None,
        }),
        "NullTerminatedArray" => Some(ReturnValueSpecialConversion::NullTerminatedArray {
            nullable: false,
            element_conversion: None,
            free_array_after_conversion: false,
            free_function: None,
        }),
        _ => None,
    }
}

fn get_return_conversion_mut<'a>(
    choices: &'a mut FunctionChoices,
    target: &ReturnEditorTarget,
) -> Option<&'a mut ReturnValueSpecialConversion> {
    match target {
        ReturnEditorTarget::Return => choices.return_value.as_mut(),
        ReturnEditorTarget::ReturnElement => match choices.return_value.as_mut()? {
            ReturnValueSpecialConversion::ArrayWithLength {
                element_conversion, ..
            }
            | ReturnValueSpecialConversion::NullTerminatedArray {
                element_conversion, ..
            }
            | ReturnValueSpecialConversion::Dereference {
                element_conversion, ..
            } => element_conversion.as_deref_mut(),
            ReturnValueSpecialConversion::String { .. }
            | ReturnValueSpecialConversion::Length { .. } => None,
        },
        ReturnEditorTarget::ParamOut(i) => {
            match choices
                .parameters
                .get_mut(*i)?
                .conversion_strategy
                .as_mut()?
            {
                ParameterSpecialConversion::Out { element_conversion } => {
                    element_conversion.as_deref_mut()
                }
                _ => None,
            }
        }
        ReturnEditorTarget::ParamOutElement(i) => {
            match choices
                .parameters
                .get_mut(*i)?
                .conversion_strategy
                .as_mut()?
            {
                ParameterSpecialConversion::Out { element_conversion } => {
                    match element_conversion.as_deref_mut()? {
                        ReturnValueSpecialConversion::ArrayWithLength {
                            element_conversion,
                            ..
                        }
                        | ReturnValueSpecialConversion::NullTerminatedArray {
                            element_conversion,
                            ..
                        }
                        | ReturnValueSpecialConversion::Dereference {
                            element_conversion, ..
                        } => element_conversion.as_deref_mut(),
                        ReturnValueSpecialConversion::String { .. }
                        | ReturnValueSpecialConversion::Length { .. } => None,
                    }
                }
                _ => None,
            }
        }
    }
}

fn set_return_conversion(
    choices: &mut FunctionChoices,
    target: &ReturnEditorTarget,
    value: Option<ReturnValueSpecialConversion>,
) {
    match target {
        ReturnEditorTarget::Return => {
            choices.return_value = value;
        }
        ReturnEditorTarget::ReturnElement => {
            if let Some(conversion) = choices.return_value.as_mut() {
                match conversion {
                    ReturnValueSpecialConversion::ArrayWithLength {
                        element_conversion, ..
                    }
                    | ReturnValueSpecialConversion::NullTerminatedArray {
                        element_conversion,
                        ..
                    }
                    | ReturnValueSpecialConversion::Dereference {
                        element_conversion, ..
                    } => {
                        *element_conversion = value.map(Box::new);
                    }
                    ReturnValueSpecialConversion::String { .. }
                    | ReturnValueSpecialConversion::Length { .. } => {}
                }
            }
        }
        ReturnEditorTarget::ParamOut(i) => {
            if let Some(ParameterSpecialConversion::Out { element_conversion }) = choices
                .parameters
                .get_mut(*i)
                .and_then(|parameter| parameter.conversion_strategy.as_mut())
            {
                *element_conversion = value.map(Box::new);
            }
        }
        ReturnEditorTarget::ParamOutElement(i) => {
            if let Some(ParameterSpecialConversion::Out { element_conversion }) = choices
                .parameters
                .get_mut(*i)
                .and_then(|parameter| parameter.conversion_strategy.as_mut())
            {
                if let Some(conversion) = element_conversion.as_deref_mut() {
                    match conversion {
                        ReturnValueSpecialConversion::ArrayWithLength {
                            element_conversion,
                            ..
                        }
                        | ReturnValueSpecialConversion::NullTerminatedArray {
                            element_conversion,
                            ..
                        }
                        | ReturnValueSpecialConversion::Dereference {
                            element_conversion, ..
                        } => {
                            *element_conversion = value.map(Box::new);
                        }
                        ReturnValueSpecialConversion::String { .. }
                        | ReturnValueSpecialConversion::Length { .. } => {}
                    }
                }
            }
        }
    }
}

fn nested_return_editor_target(target: &ReturnEditorTarget) -> Option<ReturnEditorTarget> {
    match target {
        ReturnEditorTarget::Return => Some(ReturnEditorTarget::ReturnElement),
        ReturnEditorTarget::ParamOut(i) => Some(ReturnEditorTarget::ParamOutElement(*i)),
        ReturnEditorTarget::ReturnElement | ReturnEditorTarget::ParamOutElement(_) => None,
    }
}

fn push_return_conversion_items(
    items: &mut Vec<FormItem>,
    choices: &FunctionChoices,
    conversion: &ReturnValueSpecialConversion,
    ty: Option<&CType>,
    target: ReturnEditorTarget,
    enabled: bool,
    indent: u16,
    allow_nested_element_editor: bool,
) {
    if let ReturnValueSpecialConversion::Length { of_param_index } = conversion {
        let (options, selected) =
            return_length_target_selector_state(&target, choices, *of_param_index);
        items.push(FormItem {
            label: "Length of output".to_string(),
            kind: FormItemKind::Selector {
                options,
                selected,
                enabled,
            },
            path: FormPath::ReturnLengthOf(target),
            indent,
        });
        return;
    }

    let free_label = match conversion {
        ReturnValueSpecialConversion::ArrayWithLength { .. }
        | ReturnValueSpecialConversion::NullTerminatedArray { .. } => "Free array after conversion",
        _ => "Free after conversion",
    };

    let (nullable, free, free_function, element_conversion, element_label) = match conversion {
        ReturnValueSpecialConversion::String {
            nullable,
            free,
            free_function,
        } => (
            Some(nullable),
            Some(free),
            free_function.as_ref(),
            None,
            None,
        ),
        ReturnValueSpecialConversion::Dereference {
            nullable,
            element_conversion,
            free,
            free_function,
        } => (
            Some(nullable),
            Some(free),
            free_function.as_ref(),
            Some(element_conversion),
            Some("Pointed value conversion"),
        ),
        ReturnValueSpecialConversion::ArrayWithLength {
            nullable,
            element_conversion,
            free_array_after_conversion,
            free_function,
        } => (
            Some(nullable),
            Some(free_array_after_conversion),
            free_function.as_ref(),
            Some(element_conversion),
            Some("Element conversion"),
        ),
        ReturnValueSpecialConversion::NullTerminatedArray {
            nullable,
            element_conversion,
            free_array_after_conversion,
            free_function,
        } => (
            Some(nullable),
            Some(free_array_after_conversion),
            free_function.as_ref(),
            Some(element_conversion),
            Some("Element conversion"),
        ),
        ReturnValueSpecialConversion::Length { .. } => unreachable!(),
    };

    if let Some(nullable) = nullable {
        items.push(FormItem {
            label: "Nullable".to_string(),
            kind: FormItemKind::Checkbox {
                checked: *nullable,
                enabled,
            },
            path: FormPath::ReturnNullable(target.clone()),
            indent,
        });
    }

    if let Some(free) = free {
        items.push(FormItem {
            label: free_label.to_string(),
            kind: FormItemKind::Checkbox {
                checked: *free,
                enabled,
            },
            path: FormPath::ReturnFree(target.clone()),
            indent,
        });

        if *free {
            items.push(FormItem {
                label: "Free function".to_string(),
                kind: FormItemKind::TextInput {
                    value: free_function.cloned().unwrap_or_default(),
                    enabled,
                },
                path: FormPath::ReturnFreeFunction(target.clone()),
                indent: indent + 1,
            });
        }
    }

    if allow_nested_element_editor {
        if let (Some(element_conversion), Some(element_label), Some(nested_target), Some(ty)) = (
            element_conversion,
            element_label,
            nested_return_editor_target(&target),
            ty,
        ) {
            let element_ty = get_pointee(ty);
            let element_options =
                return_conversion_options(element_ty, choices, &nested_target, false);
            let element_selected = element_conversion
                .as_ref()
                .map(|nested| return_conversion_to_index(nested, &element_options))
                .unwrap_or(0);

            items.push(FormItem {
                label: element_label.to_string(),
                kind: FormItemKind::Selector {
                    options: element_options,
                    selected: element_selected,
                    enabled,
                },
                path: FormPath::ReturnElementConversion(target.clone()),
                indent,
            });

            if let Some(nested_conversion) = get_return_conversion_from_box(element_conversion) {
                push_return_conversion_items(
                    items,
                    choices,
                    nested_conversion,
                    element_ty,
                    nested_target,
                    enabled,
                    indent + 1,
                    false,
                );
            }
        }
    }
}

fn get_return_conversion_from_box(
    conversion: &Option<Box<ReturnValueSpecialConversion>>,
) -> Option<&ReturnValueSpecialConversion> {
    conversion.as_deref()
}

fn is_pointer_to_pointer(ty: &CType) -> bool {
    matches!(get_pointee(ty), Some(inner) if is_pointer_like(inner))
}

fn is_pointer_like(ty: &CType) -> bool {
    matches!(
        ty,
        CType::Pointer { .. } | CType::IncompleteArray { .. } | CType::Array { size: None, .. }
    )
}

fn optional_function_name(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
