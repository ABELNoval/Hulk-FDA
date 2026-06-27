use std::fmt;

// Submódulos
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod span;

// Re-exportar tipos principales
pub use lexer::LexerError;
pub use parser::ParserError;
pub use semantic::SemanticError;
pub use span::Span;

// =============================================================================
// TRAIT DisplayError
// =============================================================================
/// Trait que todos los errores del compilador deben implementar.
/// Provee una interfaz uniforme para obtener información del error.
pub trait DisplayError {
    /// Retorna el código único del error (ej: "E0001", "E1010", "E2003")
    fn code(&self) -> &'static str;

    /// Retorna el mensaje descriptivo del error en español
    fn message(&self) -> String;

    /// Retorna una sugerencia de cómo arreglar el error (opcional)
    fn help(&self) -> Option<String>;
}

// =============================================================================
// ENUM CompilationError
// =============================================================================
/// Enum que unifica todos los tipos de errores del compilador.
/// Permite manejar cualquier error de cualquier fase de manera uniforme.
#[derive(Debug, Clone, PartialEq)]
pub enum CompilationError {
    /// Errores del analizador léxico (tokenización)
    Lexer { error: LexerError, span: Span },

    /// Errores del analizador sintáctico (parsing)
    Parser { error: ParserError, span: Span },

    /// Errores del análisis semántico (tipos, scopes, etc.)
    Semantic(SemanticError),

    /// Errores internos del compilador (bugs)
    Internal {
        message: String,
        location: Option<String>, // archivo:línea donde ocurrió el bug
    },

    /// Errores de entrada/salida (archivos, etc.)
    IO {
        operation: String, // "leer", "escribir", "abrir"
        path: String,
        reason: String,
    },
}

// =============================================================================
// IMPLEMENTACIÓN DE DisplayError PARA CompilationError
// =============================================================================
impl DisplayError for CompilationError {
    fn code(&self) -> &'static str {
        match self {
            CompilationError::Lexer { error, .. } => error.code(),
            CompilationError::Parser { error, .. } => error.code(),
            CompilationError::Semantic(error) => error.code(),
            CompilationError::Internal { .. } => "E9001",
            CompilationError::IO { .. } => "E9002",
        }
    }

    fn message(&self) -> String {
        match self {
            CompilationError::Lexer { error, .. } => error.message(),
            CompilationError::Parser { error, .. } => error.message(),
            CompilationError::Semantic(error) => error.message(),
            CompilationError::Internal { message, location } => match location {
                Some(loc) => format!("error interno del compilador en {}: {}", loc, message),
                None => format!("error interno del compilador: {}", message),
            },
            CompilationError::IO {
                operation,
                path,
                reason,
            } => {
                format!("error al {} '{}': {}", operation, path, reason)
            }
        }
    }

    fn help(&self) -> Option<String> {
        match self {
            CompilationError::Lexer { error, .. } => error.help(),
            CompilationError::Parser { error, .. } => error.help(),
            CompilationError::Semantic(error) => error.help(),
            CompilationError::Internal { .. } => {
                Some("esto es un bug del compilador, por favor repórtalo".to_string())
            }
            CompilationError::IO { operation, .. } => match operation.as_str() {
                "leer" => {
                    Some("verifica que el archivo exista y tengas permisos de lectura".to_string())
                }
                "escribir" => {
                    Some("verifica que tengas permisos de escritura en el directorio".to_string())
                }
                "abrir" => Some("verifica la ruta del archivo".to_string()),
                _ => None,
            },
        }
    }
}

// =============================================================================
// IMPLEMENTACIÓN DE fmt::Display
// =============================================================================
/// Formato de salida del error.
/// Produce algo como: "error[E0001]: caracter inesperado '@'"
impl fmt::Display for CompilationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Determinar el prefijo según el tipo de error
        let prefix = match self {
            CompilationError::Internal { .. } => "error interno",
            CompilationError::IO { .. } => "error de E/S",
            _ => "error",
        };

        // Formato: prefijo[código]: mensaje
        write!(f, "{}[{}]: {}", prefix, self.code(), self.message())?;

        // Si hay ayuda, agregarla en una nueva línea
        if let Some(help_text) = self.help() {
            write!(f, "\n  = ayuda: {}", help_text)?;
        }

        Ok(())
    }
}

// =============================================================================
// IMPLEMENTACIÓN DE std::error::Error
// =============================================================================
impl std::error::Error for CompilationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

// =============================================================================
// CONVERSIONES From<T> PARA FACILITAR EL USO
// =============================================================================
impl From<LexerError> for CompilationError {
    fn from(error: LexerError) -> Self {
        CompilationError::Lexer {
            error,
            span: Span::default(),
        }
    }
}

impl From<ParserError> for CompilationError {
    fn from(error: ParserError) -> Self {
        CompilationError::Parser {
            error,
            span: Span::default(),
        }
    }
}

impl From<SemanticError> for CompilationError {
    fn from(error: SemanticError) -> Self {
        CompilationError::Semantic(error)
    }
}

impl From<std::io::Error> for CompilationError {
    fn from(error: std::io::Error) -> Self {
        CompilationError::IO {
            operation: "acceder".to_string(),
            path: "<desconocido>".to_string(),
            reason: error.to_string(),
        }
    }
}

// =============================================================================
// MÉTODOS DE CONVENIENCIA
// =============================================================================
impl CompilationError {
    /// Crea un error léxico con ubicación.
    pub fn lexer(error: LexerError, span: Span) -> Self {
        CompilationError::Lexer { error, span }
    }

    /// Crea un error sintáctico con ubicación.
    pub fn parser(error: ParserError, span: Span) -> Self {
        CompilationError::Parser { error, span }
    }

    /// Crea un error semántico.
    pub fn semantic(error: SemanticError) -> Self {
        CompilationError::Semantic(error)
    }

    /// Crea un error interno del compilador
    pub fn internal(message: impl Into<String>) -> Self {
        CompilationError::Internal {
            message: message.into(),
            location: None,
        }
    }

    /// Crea un error interno con ubicación
    pub fn internal_at(message: impl Into<String>, location: impl Into<String>) -> Self {
        CompilationError::Internal {
            message: message.into(),
            location: Some(location.into()),
        }
    }

    /// Crea un error de E/S
    pub fn io(
        operation: impl Into<String>,
        path: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        CompilationError::IO {
            operation: operation.into(),
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Retorna true si es un error interno (bug del compilador)
    pub fn is_internal(&self) -> bool {
        matches!(self, CompilationError::Internal { .. })
    }

    /// Retorna la fase del compilador donde ocurrió el error
    pub fn phase(&self) -> &'static str {
        match self {
            CompilationError::Lexer { .. } => "lexer",
            CompilationError::Parser { .. } => "parser",
            CompilationError::Semantic(_) => "semantic",
            CompilationError::Internal { .. } => "internal",
            CompilationError::IO { .. } => "io",
        }
    }

    /// Retorna la ubicación adecuada para reportes de interfaz.
    pub fn interface_position(&self) -> (usize, usize) {
        match self {
            CompilationError::Lexer { span, .. } | CompilationError::Parser { span, .. } => {
                (span.start_line, span.start_column)
            }
            CompilationError::Semantic(error) => error.location().unwrap_or((0, 0)),
            CompilationError::Internal { .. } | CompilationError::IO { .. } => (0, 0),
        }
    }

    /// Retorna el rótulo exigido por la interfaz externa.
    pub fn interface_type(&self) -> &'static str {
        match self {
            CompilationError::Lexer { .. } => "LEXICAL",
            CompilationError::Parser { .. } => "SYNTACTIC",
            CompilationError::Semantic(_) => "SEMANTIC",
            CompilationError::Internal { .. } => "INTERNAL",
            CompilationError::IO { .. } => "IO",
        }
    }

    /// Formatea el error para stderr según el contrato de entrega.
    pub fn render_interface(&self) -> String {
        let (line, column) = self.interface_position();
        format!(
            "({},{}) {}: {}",
            line,
            column,
            self.interface_type(),
            self.message()
        )
    }

    /// Código de salida esperado para la interfaz de entrega.
    pub fn exit_code(&self) -> i32 {
        match self {
            CompilationError::Lexer { .. } => 1,
            CompilationError::Parser { .. } => 2,
            CompilationError::Semantic(_) => 3,
            CompilationError::Internal { .. } | CompilationError::IO { .. } => 1,
        }
    }
}

// =============================================================================
// TIPO Result ALIAS
// =============================================================================
/// Alias para Result con CompilationError como tipo de error
pub type CompileResult<T> = Result<T, CompilationError>;

/// Alias para Result con múltiples errores
pub type CompileResultMulti<T> = Result<T, Vec<CompilationError>>;
