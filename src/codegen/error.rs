use std::fmt;

pub type CodegenResult<T> = Result<T, CodegenError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    InvalidModule {
        module: String,
        message: String,
    },
    UnsupportedInstruction {
        function: String,
        block: String,
        message: String,
    },
    MissingType {
        value: String,
    },
    BackendFailure {
        message: String,
    },
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::InvalidModule { module, message } => {
                write!(f, "invalid module '{}': {}", module, message)
            }
            CodegenError::UnsupportedInstruction {
                function,
                block,
                message,
            } => write!(
                f,
                "unsupported instruction in function '{}' block '{}': {}",
                function, block, message
            ),
            CodegenError::MissingType { value } => {
                write!(f, "missing type information for value '{}'", value)
            }
            CodegenError::BackendFailure { message } => write!(f, "backend failure: {}", message),
        }
    }
}

impl std::error::Error for CodegenError {}
