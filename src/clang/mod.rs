pub mod types;

use clang::{Clang, EntityKind, Index, TypeKind};
use types::{CDeclaration, CEnumVariant, CField, CParameter, CType};

use crate::options::types::Options;

pub fn parse_header(c_header_filepath: &str, options: &Options) -> Vec<CDeclaration> {
    let clang = Clang::new().expect("failed to initialize clang");
    let index = Index::new(&clang, false, true);
    let mut clang_args = options.clang_args.clone();
    clang_args.extend_from_slice(&["-x".to_string(), "c".to_string(), "-std=c11".to_string()]);
    let tu = index
        .parser(c_header_filepath)
        .arguments(&clang_args)
        .parse()
        .expect("failed to parse translation unit");
    let entity = tu.get_entity();
    let mut declarations = Vec::new();
    for child in entity.get_children() {
        if let Some(decl) = convert_entity(&child) {
            declarations.push(decl);
        }
    }
    declarations
}

fn convert_entity(entity: &clang::Entity) -> Option<CDeclaration> {
    match entity.get_kind() {
        EntityKind::FunctionDecl => {
            let name = entity.get_name()?;
            let return_type = entity
                .get_result_type()
                .map(|t| convert_type(&t))
                .unwrap_or(CType::Void);
            let parameters = entity
                .get_arguments()
                .unwrap_or_default()
                .iter()
                .map(|arg| CParameter {
                    name: arg.get_name().filter(|n| !n.is_empty()),
                    ty: arg
                        .get_type()
                        .map(|t| convert_type(&t))
                        .unwrap_or(CType::Void),
                })
                .collect();
            let is_variadic = entity.is_variadic();
            Some(CDeclaration::Function {
                name,
                return_type,
                parameters,
                is_variadic,
            })
        }
        EntityKind::StructDecl => {
            let name = declaration_name(entity);
            let fields = extract_fields(entity);
            Some(CDeclaration::Struct { name, fields })
        }
        EntityKind::UnionDecl => {
            let name = declaration_name(entity);
            let fields = extract_fields(entity);
            Some(CDeclaration::Union { name, fields })
        }
        EntityKind::EnumDecl => {
            let name = declaration_name(entity);
            let variants = entity
                .get_children()
                .iter()
                .filter(|c| c.get_kind() == EntityKind::EnumConstantDecl)
                .filter_map(|c| {
                    Some(CEnumVariant {
                        name: c.get_name()?,
                        value: c.get_enum_constant_value().map(|(signed, _)| signed),
                    })
                })
                .collect();
            Some(CDeclaration::Enum { name, variants })
        }
        EntityKind::TypedefDecl => {
            let name = entity.get_name()?;
            let clang_underlying_type = entity.get_typedef_underlying_type();
            let underlying_type = clang_underlying_type
                .as_ref()
                .map(convert_type)
                .unwrap_or(CType::Unknown("unknown".into()));
            let underlying_declaration = clang_underlying_type
                .as_ref()
                .and_then(|ty| ty.get_declaration())
                .and_then(|declaration| convert_tagged_declaration(&declaration))
                .or_else(|| extract_typedef_underlying_declaration(entity));
            Some(CDeclaration::Typedef {
                name,
                underlying_type,
                underlying_declaration,
            })
        }
        EntityKind::VarDecl => {
            let name = entity.get_name()?;
            let ty = entity
                .get_type()
                .map(|t| convert_type(&t))
                .unwrap_or(CType::Void);
            Some(CDeclaration::Variable { name, ty })
        }
        EntityKind::MacroDefinition => {
            let name = entity.get_name()?;
            Some(CDeclaration::Macro { name })
        }
        _ => None,
    }
}

fn extract_fields(entity: &clang::Entity) -> Vec<CField> {
    entity
        .get_children()
        .iter()
        .filter(|c| c.get_kind() == EntityKind::FieldDecl)
        .filter_map(|c| {
            Some(CField {
                name: c.get_name().unwrap_or_default(),
                ty: c
                    .get_type()
                    .map(|t| convert_type(&t))
                    .unwrap_or(CType::Void),
            })
        })
        .collect()
}

fn declaration_name(entity: &clang::Entity) -> Option<String> {
    if entity.is_anonymous() || !name_is_spelled_as_tag(entity) {
        None
    } else {
        entity.get_name()
    }
}

fn name_is_spelled_as_tag(entity: &clang::Entity) -> bool {
    let keyword = match entity.get_kind() {
        EntityKind::StructDecl => "struct",
        EntityKind::UnionDecl => "union",
        EntityKind::EnumDecl => "enum",
        _ => return true,
    };

    let name_ranges = entity.get_name_ranges();
    let Some(name_range) = name_ranges.first() else {
        return false;
    };
    let location = name_range.get_start().get_spelling_location();
    let Some(file) = location.file else {
        return false;
    };
    let Ok(source) = std::fs::read_to_string(file.get_path()) else {
        return false;
    };
    let offset = location.offset as usize;
    if offset > source.len() {
        return false;
    }

    source[..offset]
        .trim_end_matches(char::is_whitespace)
        .ends_with(keyword)
}

fn convert_tagged_declaration(entity: &clang::Entity) -> Option<Box<CDeclaration>> {
    match entity.get_kind() {
        EntityKind::StructDecl => Some(Box::new(CDeclaration::Struct {
            name: declaration_name(entity),
            fields: extract_fields(entity),
        })),
        EntityKind::UnionDecl => Some(Box::new(CDeclaration::Union {
            name: declaration_name(entity),
            fields: extract_fields(entity),
        })),
        EntityKind::EnumDecl => {
            let variants = entity
                .get_children()
                .iter()
                .filter(|c| c.get_kind() == EntityKind::EnumConstantDecl)
                .filter_map(|c| {
                    Some(CEnumVariant {
                        name: c.get_name()?,
                        value: c.get_enum_constant_value().map(|(signed, _)| signed),
                    })
                })
                .collect();
            Some(Box::new(CDeclaration::Enum {
                name: declaration_name(entity),
                variants,
            }))
        }
        _ => None,
    }
}

fn extract_typedef_underlying_declaration(entity: &clang::Entity) -> Option<Box<CDeclaration>> {
    entity
        .get_children()
        .iter()
        .find_map(|child| convert_tagged_declaration(child))
}

fn convert_type(ty: &clang::Type) -> CType {
    if ty.get_display_name() == "size_t" {
        return CType::SizeT;
    } else if ty.get_display_name() == "ptrdiff_t" {
        return CType::PtrdiffT;
    }
    match ty.get_kind() {
        TypeKind::Void => CType::Void,
        TypeKind::Bool => CType::Bool,
        TypeKind::CharS | TypeKind::CharU | TypeKind::SChar => CType::Char,
        TypeKind::UChar => CType::UChar,
        TypeKind::Short => CType::Short,
        TypeKind::UShort => CType::UShort,
        TypeKind::Int => CType::Int,
        TypeKind::UInt => CType::UInt,
        TypeKind::Long => CType::Long,
        TypeKind::ULong => CType::ULong,
        TypeKind::LongLong => CType::LongLong,
        TypeKind::ULongLong => CType::ULongLong,
        TypeKind::Float => CType::Float,
        TypeKind::Double => CType::Double,
        TypeKind::LongDouble => CType::LongDouble,
        TypeKind::Pointer => {
            let pointee = ty
                .get_pointee_type()
                .map(|p| convert_type(&p))
                .unwrap_or(CType::Void);
            let is_const = ty
                .get_pointee_type()
                .map(|p| p.is_const_qualified())
                .unwrap_or(false);
            CType::Pointer {
                is_const,
                pointee: Box::new(pointee),
            }
        }
        TypeKind::ConstantArray => {
            let element = ty
                .get_element_type()
                .map(|e| convert_type(&e))
                .unwrap_or(CType::Void);
            let size = ty.get_size();
            CType::Array {
                element: Box::new(element),
                size,
            }
        }
        TypeKind::IncompleteArray => {
            let element = ty
                .get_element_type()
                .map(|e| convert_type(&e))
                .unwrap_or(CType::Void);
            CType::IncompleteArray {
                element: Box::new(element),
            }
        }
        TypeKind::Typedef => {
            let name = ty.get_display_name();
            CType::Typedef(name)
        }
        TypeKind::Record => {
            let mut display_name = ty.get_display_name();
            remove_prefix(&mut display_name, "const ");
            let is_union = display_name.starts_with("union ");
            let declaration = ty.get_declaration();
            let mut name = if declaration.as_ref().is_some_and(clang::Entity::is_anonymous) {
                String::new()
            } else if let Some(name) = declaration.as_ref().and_then(declaration_name) {
                name
            } else {
                display_name
            };
            remove_prefix(&mut name, "union ");
            remove_prefix(&mut name, "struct ");
            if is_union {
                CType::Union(name)
            } else {
                CType::Struct(name)
            }
        }
        TypeKind::Enum => {
            let declaration = ty.get_declaration();
            let mut name = if declaration.as_ref().is_some_and(clang::Entity::is_anonymous) {
                String::new()
            } else if let Some(name) = declaration.as_ref().and_then(declaration_name) {
                name
            } else {
                ty.get_display_name()
            };
            remove_prefix(&mut name, "enum ");
            CType::Enum(name)
        }
        TypeKind::FunctionPrototype => {
            let return_type = ty
                .get_result_type()
                .map(|r| convert_type(&r))
                .unwrap_or(CType::Void);
            let parameters = ty
                .get_argument_types()
                .unwrap_or_default()
                .iter()
                .map(|a| convert_type(a))
                .collect();
            CType::FunctionPointer {
                return_type: Box::new(return_type),
                parameters,
            }
        }
        TypeKind::Elaborated => convert_type(&ty.get_elaborated_type().unwrap()),
        _ => CType::Unknown(ty.get_display_name()),
    }
}

fn remove_prefix(s: &mut String, prefix: &str) {
    if s.starts_with(prefix) {
        s.drain(..prefix.len());
    }
}
