#[derive(Debug, Clone)]
pub enum CType {
    Void,
    Bool,
    Char,
    UChar,
    Short,
    UShort,
    Int,
    UInt,
    Long,
    ULong,
    LongLong,
    ULongLong,
    Float,
    Double,
    LongDouble,
    SizeT,
    PtrdiffT,
    Pointer {
        is_const: bool,
        pointee: Box<CType>,
    },
    Array {
        element: Box<CType>,
        size: Option<usize>,
    },
    Struct(String),
    Union(String),
    Enum(String),
    Typedef(String),
    FunctionPointer {
        return_type: Box<CType>,
        parameters: Vec<CType>,
    },
    IncompleteArray {
        element: Box<CType>,
    },
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct CField {
    pub name: String,
    pub ty: CType,
}

#[derive(Debug, Clone)]
pub struct CParameter {
    pub name: Option<String>,
    pub ty: CType,
}

#[derive(Debug, Clone)]
pub struct CEnumVariant {
    pub name: String,
    pub value: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum CDeclaration {
    Function {
        name: String,
        return_type: CType,
        parameters: Vec<CParameter>,
        is_variadic: bool,
    },
    Struct {
        name: Option<String>,
        fields: Vec<CField>,
    },
    #[allow(dead_code)]
    Union {
        name: Option<String>,
        fields: Vec<CField>,
    },
    Enum {
        name: Option<String>,
        variants: Vec<CEnumVariant>,
    },
    Typedef {
        name: String,
        underlying_type: CType,
    },
    #[allow(dead_code)]
    Variable {
        name: String,
        ty: CType,
    },
    #[allow(dead_code)]
    Macro {
        name: String,
    },
}

#[derive(Debug, Clone)]
pub struct CFunction {
    pub name: String,
    pub return_type: CType,
    pub parameters: Vec<CParameter>,
}
