use super::ident::sanitize_lean_type_name;
use crate::clang::types::{CDeclaration, CEnumVariant, CField, CType};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
struct StructMetadata {
    c_type_name: String,
    fields: Vec<CField>,
}

#[derive(Debug, Clone, Default)]
pub struct TypeRegistry {
    structs: HashMap<String, StructMetadata>,
    struct_aliases: HashMap<String, String>,
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
                    registry.register_named_struct(name, fields);
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
                    ..
                } => {
                    registry
                        .typedefs
                        .insert(name.clone(), underlying_type.clone());
                }
                _ => {}
            }
        }

        for declaration in declarations {
            if let CDeclaration::Typedef {
                name,
                underlying_type,
                underlying_declaration,
            } = declaration
            {
                registry.register_typedef_struct(
                    name,
                    underlying_type,
                    underlying_declaration.as_deref(),
                );
                registry.register_typedef_enum(name, underlying_declaration.as_deref());
            }
        }

        registry
    }

    fn register_named_struct(&mut self, name: &str, fields: &[CField]) {
        self.register_struct_entry(name, &format!("struct {}", name), fields);
        self.add_struct_alias(name, name);
    }

    fn register_typedef_struct(
        &mut self,
        typedef_name: &str,
        underlying_type: &CType,
        underlying_declaration: Option<&CDeclaration>,
    ) {
        let inline_struct = match underlying_declaration {
            Some(CDeclaration::Struct { name, fields }) => Some((name.as_deref(), fields.as_slice())),
            _ => None,
        };

        let inline_struct_is_anonymous = matches!(inline_struct, Some((None, _)));
        let struct_tag = if inline_struct_is_anonymous {
            None
        } else {
            match underlying_type {
                CType::Struct(name) if !name.is_empty() => Some(name.clone()),
                _ => None,
            }
        };

        let Some(canonical_name) = struct_tag.clone().or_else(|| {
            inline_struct
                .map(|_| typedef_name.to_string())
                .or_else(|| self.struct_canonical_name(underlying_type))
        }) else {
            return;
        };

        let c_type_name = struct_tag
            .as_ref()
            .map(|name| format!("struct {}", name))
            .or_else(|| self.struct_c_type(underlying_type))
            .unwrap_or_else(|| typedef_name.to_string());

        let fields = inline_struct
            .map(|(_, fields)| fields.to_vec())
            .or_else(|| {
                struct_tag
                    .as_ref()
                    .and_then(|name| self.structs.get(name).map(|metadata| metadata.fields.clone()))
            })
            .or_else(|| {
                self.structs
                    .get(&canonical_name)
                    .map(|metadata| metadata.fields.clone())
            })
            .unwrap_or_default();

        self.register_struct_entry(&canonical_name, &c_type_name, &fields);
        self.add_struct_alias(typedef_name, &canonical_name);
        if let Some(struct_tag) = struct_tag {
            self.add_struct_alias(&struct_tag, &canonical_name);
        }
    }

    fn register_typedef_enum(
        &mut self,
        typedef_name: &str,
        underlying_declaration: Option<&CDeclaration>,
    ) {
        let Some(CDeclaration::Enum { variants, .. }) = underlying_declaration else {
            return;
        };

        self.enums
            .entry(typedef_name.to_string())
            .or_insert_with(|| variants.clone());
    }

    fn register_struct_entry(&mut self, canonical_name: &str, c_type_name: &str, fields: &[CField]) {
        let metadata = self
            .structs
            .entry(canonical_name.to_string())
            .or_insert_with(|| StructMetadata {
                c_type_name: c_type_name.to_string(),
                fields: Vec::new(),
            });

        metadata.c_type_name = c_type_name.to_string();
        if metadata.fields.is_empty() || !fields.is_empty() {
            metadata.fields = fields.to_vec();
        }
    }

    fn add_struct_alias(&mut self, alias: &str, canonical_name: &str) {
        if !alias.is_empty() {
            self.struct_aliases
                .insert(alias.to_string(), canonical_name.to_string());
        }
    }

    pub(crate) fn resolve_alias_type(&self, ty: &CType) -> CType {
        self.resolve_alias_type_inner(ty, &mut HashSet::new())
    }

    fn resolve_alias_type_inner(&self, ty: &CType, seen: &mut HashSet<String>) -> CType {
        match ty {
            CType::Typedef(name) => {
                if !seen.insert(name.clone()) {
                    return ty.clone();
                }

                if let Some(canonical_name) = self.struct_aliases.get(name) {
                    seen.remove(name);
                    return CType::Struct(canonical_name.clone());
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

    fn struct_canonical_name_inner(&self, ty: &CType, seen: &mut HashSet<String>) -> Option<String> {
        match ty {
            CType::Struct(name) | CType::Typedef(name) => {
                if let Some(canonical_name) = self.struct_aliases.get(name) {
                    return Some(canonical_name.clone());
                }

                if matches!(ty, CType::Struct(_)) && self.structs.contains_key(name) {
                    return Some(name.clone());
                }

                if !matches!(ty, CType::Typedef(_)) || !seen.insert(name.clone()) {
                    return None;
                }

                let resolved = self
                    .typedefs
                    .get(name)
                    .and_then(|underlying| self.struct_canonical_name_inner(underlying, seen));

                seen.remove(name);
                resolved
            }
            _ => None,
        }
    }

    pub(crate) fn struct_canonical_name(&self, ty: &CType) -> Option<String> {
        self.struct_canonical_name_inner(ty, &mut HashSet::new())
    }

    pub(crate) fn struct_c_type(&self, ty: &CType) -> Option<String> {
        let canonical_name = self.struct_canonical_name(ty)?;
        self.structs
            .get(&canonical_name)
            .map(|metadata| metadata.c_type_name.clone())
            .or_else(|| Some(format!("struct {}", canonical_name)))
    }

    pub(crate) fn struct_lean_name(&self, ty: &CType) -> Option<String> {
        self.struct_canonical_name(ty)
            .map(|name| sanitize_lean_type_name(&name))
    }

    fn named_type_name(&self, ty: &CType) -> Option<String> {
        match ty {
            CType::Struct(name) | CType::Enum(name) | CType::Union(name) | CType::Typedef(name) => {
                Some(name.clone())
            }
            _ => None,
        }
    }

    pub(crate) fn struct_info<'a>(&'a self, ty: &CType) -> Option<(String, &'a [CField])> {
        let canonical_name = self.struct_canonical_name(ty)?;
        self.structs.get(&canonical_name).map(|metadata| {
            (
                sanitize_lean_type_name(&canonical_name),
                metadata.fields.as_slice(),
            )
        })
    }

    pub(crate) fn enum_info<'a>(&'a self, ty: &CType) -> Option<(String, &'a [CEnumVariant])> {
        let lean_name = self.named_type_name(ty)?;
        match self.resolve_alias_type(ty) {
            CType::Enum(name) => self
                .enums
                .get(&name)
                .map(|variants| (sanitize_lean_type_name(&lean_name), variants.as_slice())),
            _ => None,
        }
    }

    pub(crate) fn is_char_pointer_like(&self, ty: &CType) -> bool {
        matches!(
            self.resolve_alias_type(ty),
            CType::Pointer { pointee, .. } if matches!(*pointee, CType::Char)
        )
    }

    pub(crate) fn pointer_element_type(&self, ty: &CType) -> Option<CType> {
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

    pub(crate) fn is_pointer_like(&self, ty: &CType) -> bool {
        matches!(
            self.resolve_alias_type(ty),
            CType::Pointer { .. } | CType::IncompleteArray { .. } | CType::Array { size: None, .. }
        )
    }
}
