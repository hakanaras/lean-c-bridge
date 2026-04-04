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
    /// Automatically pass a char* from a Lean String, and free it after the call
    String,
    /// Pass an automatically allocated char* buffer that can be used by the original C function to write a string into.
    /// The buffer is subsequently converted back into a Lean String and added to the return value by making it a tuple.
    /// The buffer is automatically freed after the call.
    StringBuffer {
        /// The size of the buffer to allocate.
        buffer_size: usize,
    },
    /// Pass a pointer to a newly allocated array containing the elements of a Lean Array, and free it after the call
    Array {
        /// Optional conversion for the individual elements of the array
        element_conversion: Option<Box<ParameterSpecialConversion>>,
    },
    /// Treat the parameter as an output pointer and add the value it points to to the return value by making it a tuple.
    Out {
        /// Optional conversion for the pointed value, using the same conversion choices as return values.
        element_conversion: Option<Box<ReturnValueSpecialConversion>>,
    },
    /// Pass the length of another parameter (which must have String, StringBuffer or Array conversion)
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
        /// Whether to free the char* after converting it.
        free: bool,
        /// Function that should be used to free the char*.
        /// When left empty, it defaults to `free` from the C standard library.
        /// Only has an effect if `free` is true.
        free_function: Option<String>,
    },
    /// Automatically convert a pointer-to-pointer return value (which is interpreted as a null-terminated array)
    /// into a Lean Array.
    NullTerminatedArray {
        /// Optional conversion for the pointed value, using the same conversion choices as return values.
        element_conversion: Option<Box<ReturnValueSpecialConversion>>,
        /// Whether to free the top-level array after converting it.
        #[serde(default)]
        free_array_after_conversion: bool,
        /// Function that should be used to free the array.
        /// When left empty, it defaults to `free` from the C standard library.
        /// Only has an effect if `free_array_after_conversion` is true.
        free_function: Option<String>
    },
}
