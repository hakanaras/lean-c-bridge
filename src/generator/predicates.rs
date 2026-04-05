use super::TypeRegistry;
use crate::{
    clang::types::{CFunction, CType},
    options::interface_choices::ReturnValueSpecialConversion,
};

pub(crate) fn can_be_no_io(function: &CFunction, registry: &TypeRegistry) -> bool {
    function
        .parameters
        .iter()
        .all(|parameter| is_no_io_primitive(&registry.resolve_alias_type(&parameter.ty)))
        && is_no_io_primitive(&registry.resolve_alias_type(&function.return_type))
}

pub(crate) fn is_no_io_primitive(ty: &CType) -> bool {
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

pub(crate) fn is_lean_float_type(ty: &CType, registry: &TypeRegistry) -> bool {
    matches!(
        registry.resolve_alias_type(ty),
        CType::Float | CType::Double | CType::LongDouble
    )
}

pub(crate) fn is_lean_float_return(
    ty: &CType,
    strategy: Option<&ReturnValueSpecialConversion>,
    registry: &TypeRegistry,
) -> bool {
    strategy.is_none() && is_lean_float_type(ty, registry)
}
