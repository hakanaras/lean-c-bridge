use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceChoices {
    pub functions: Vec<FunctionChoices>,
}

impl InterfaceChoices {
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).unwrap();
        std::fs::write(path, json)
    }

    pub fn load(path: &str) -> std::io::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let choices = serde_json::from_str(&json).unwrap();
        Ok(choices)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionChoices {
    pub name: String,
    pub omit: bool,
    pub no_io: bool,
    pub parameters: Vec<ParameterChoices>,
    pub return_value: Option<ReturnValueSpecialConversion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterChoices {
    pub conversion_strategy: Option<ParameterSpecialConversion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterSpecialConversion {
    /// Instead of passing the argument directly to the original C function, a pointer to it will be passed.
    /// The pointer will be pointing to the stack, so the original C function may not keep the pointer after the function returns.
    Reference {
        /// Whether to use `Option` and pass `null` when `none`.
        #[serde(default)]
        nullable: bool,
        /// Optional conversion for the pointed value.
        element_conversion: Option<Box<ParameterSpecialConversion>>,
    },
    /// Automatically pass a char* from a Lean String, and free it after the call
    String {
        /// Whether to use `Option` and pass `null` when `none`.
        #[serde(default)]
        nullable: bool,
        /// Whether to skip the `free` after the call. This is useful if the C function takes ownership of the string.
        #[serde(default)]
        skip_free: bool,
    },
    /// Pass an automatically allocated char* buffer that can be used by the original C function to write a string into.
    /// The buffer is subsequently converted back into a Lean String and added to the return value by making it a tuple.
    /// The buffer is automatically freed after the call.
    StringBuffer {
        /// The size of the buffer to allocate.
        buffer_size: usize,
    },
    /// Pass an automatically allocated buffer for the original C function to write an array into, and convert it back
    /// into a Lean Array and add it to the return value by making it a tuple. The buffer is automatically freed after the call.
    ArrayBuffer {
        /// The size of the buffer to allocate.
        buffer_size: usize,
        /// The C expression that can be used to check if an element is the terminator of the array.
        /// The expression can use `%ELEM%` as a placeholder for the element.
        /// For example `%ELEM% == 0` can be used for zero-terminated arrays.
        terminator_expression: String,
        /// Optional conversion for the individual elements of the array, using the same conversion choices as return values.
        element_conversion: Option<Box<ReturnValueSpecialConversion>>,
        /// Whether this should be marshalled to a `ByteArray` instead of a regular `Array`.
        /// Only available for elements `char`, `signed char`, and `unsigned char`.
        #[serde(default)]
        byte_array: bool,
    },
    /// Pass a pointer to a newly allocated array containing the elements of a Lean Array, and free it after the call
    Array {
        /// Whether to use `Option` and pass `null` when `none`.
        #[serde(default)]
        nullable: bool,
        /// Optional conversion for the individual elements of the array
        element_conversion: Option<Box<ParameterSpecialConversion>>,
        /// Whether this should be marshalled from a `ByteArray` instead of a regular `Array`.
        /// Only available for elements `char`, `signed char`, and `unsigned char`.
        #[serde(default)]
        byte_array: bool,
        /// Whether to skip the `free` after the call. This is useful if the C function takes ownership of the array.
        #[serde(default)]
        skip_free: bool,
    },
    /// Treat the parameter as an output pointer and add the value it points to to the return value by making it a tuple.
    Out {
        /// Optional conversion for the pointed value, using the same conversion choices as return values.
        element_conversion: Option<Box<ReturnValueSpecialConversion>>,
    },
    /// Automatically pass the length of another parameter (which must have `String`, `StringBuffer` or `Array` conversion)
    Length {
        /// Which parameter is this the length of? (0-based index)
        of_param_index: usize,
    },
    /// The parameter is not in the FFI. It is always passed the same expression.
    StaticExpr {
        /// Optional statements to be emitted before the expression, e.g. to set up variables used in the expression
        pre_statements: Vec<String>,
        /// The expression to be passed for this parameter
        expr: String,
        /// Optional statements to be emitted after the expression, e.g. for cleanup
        post_statements: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReturnValueSpecialConversion {
    /// Automatically convert a char* return value into a Lean String.
    String {
        /// Whether to use `Option String` instead of `String` and interpret `null` as `none`.
        #[serde(default)]
        nullable: bool,
        /// Whether to free the char* after converting it.
        free: bool,
        /// Function that should be used to free the char*.
        /// When left empty, it defaults to `free` from the C standard library.
        /// Only has an effect if `free` is true.
        free_function: Option<String>,
    },
    /// Automatically convert a pointer return value into the value it points to.
    Dereference {
        /// Whether to use `Option` and interpret `null` as `none`.
        #[serde(default)]
        nullable: bool,
        /// Optional conversion for the pointed value, using the same conversion choices as return values.
        element_conversion: Option<Box<ReturnValueSpecialConversion>>,
        /// Whether to free the pointer after converting it.
        free: bool,
        /// Function that should be used to free the pointer.
        /// When left empty, it defaults to `free` from the C standard library.
        /// Only has an effect if `free` is true.
        free_function: Option<String>,
    },
    /// Denotes the length of another parameter or return value with `ArrayWithLength` or `String` conversion.
    Length {
        /// Which parameter is this the length of? (0-based index).
        /// `None` means that it applies to the return value.
        of_param_index: Option<usize>,
    },
    /// Automatically convert a pointer return value (which is interpreted as an array) into a
    /// Lean `Array` of the pointed value.
    ArrayWithLength {
        /// Whether to use `Option` and interpret `null` as `none`.
        #[serde(default)]
        nullable: bool,
        /// Optional conversion for the individual elements of the array.
        element_conversion: Option<Box<ReturnValueSpecialConversion>>,
        /// Whether to free the array after converting it.
        free_array_after_conversion: bool,
        /// Function that should be used to free the array.
        /// When left empty, it defaults to `free` from the C standard library.
        /// Only has an effect if `free_array_after_conversion` is true.
        free_function: Option<String>,
        /// Whether this should be marshalled to a `ByteArray` instead of a regular `Array`.
        /// Only available for elements `char`, `signed char`, and `unsigned char`.
        #[serde(default)]
        byte_array: bool,
    },
    /// Automatically convert a pointer-to-pointer return value (which is interpreted as a null-terminated array)
    /// into a Lean Array.
    NullTerminatedArray {
        /// Whether to use `Option` and interpret `null` as `none`.
        #[serde(default)]
        nullable: bool,
        /// Optional conversion for the individual elements of the array.
        element_conversion: Option<Box<ReturnValueSpecialConversion>>,
        /// Whether to free the array after converting it.
        #[serde(default)]
        free_array_after_conversion: bool,
        /// Function that should be used to free the array.
        /// When left empty, it defaults to `free` from the C standard library.
        /// Only has an effect if `free_array_after_conversion` is true.
        free_function: Option<String>,
    },
}
