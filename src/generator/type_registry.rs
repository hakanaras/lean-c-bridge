use super::ident::sanitize_lean_type_name;
use crate::clang::types::{CDeclaration, CEnumVariant, CField, CType};
use std::collections::{HashMap, HashSet};

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

    pub(crate) fn resolve_alias_type(&self, ty: &CType) -> CType {
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
            CType::Struct(name) | CType::Enum(name) | CType::Union(name) | CType::Typedef(name) => {
                Some(name.clone())
            }
            _ => None,
        }
    }

    pub(crate) fn struct_info<'a>(&'a self, ty: &CType) -> Option<(String, &'a [CField])> {
        let lean_name = self.named_type_name(ty)?;
        match self.resolve_alias_type(ty) {
            CType::Struct(name) => self
                .structs
                .get(&name)
                .map(|fields| (sanitize_lean_type_name(&lean_name), fields.as_slice())),
            _ => None,
        }
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
