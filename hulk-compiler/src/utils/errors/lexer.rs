use super::DisplayError;

/// Enum con todos los tipos de errores que puede producir el Lexer
#[derive(Debug, Clone, PartialEq)]
pub enum LexerError {
    // ==================== CARACTERES INVÁLIDOS ====================
    /// Caracter que no pertenece al lenguaje
    /// Ejemplo: let x = 5 @ 3;  (@ no es válido en HULK)
    UnexpectedCharacter(char),

    /// Caracter inválido encontrado dentro de un número
    /// Ejemplo: 123abc (sin espacio), 45.67.89
    InvalidCharacterInNumber { found: char, partial_number: String },

    /// Caracter inválido dentro de un identificador
    /// Ejemplo: my$var, foo#bar
    InvalidCharacterInIdentifier {
        found: char,
        partial_identifier: String,
    },

    // ==================== LITERALES STRING ====================
    /// String que nunca se cierra (EOF antes de encontrar ")
    /// Ejemplo: let s = "hola mundo
    UnterminatedString {
        start_line: usize,
        start_column: usize,
    },

    /// Secuencia de escape inválida dentro de un string
    /// Ejemplo: "hola\q mundo" (\q no es un escape válido)
    /// Escapes válidos típicos: \n, \t, \r, \\, \", \0
    InvalidEscapeSequence { found: char, in_string: String },

    /// String contiene salto de línea sin escape
    /// Ejemplo: "hola
    ///           mundo"  (sin usar \n)
    UnexpectedNewlineInString { line: usize },

    // ==================== LITERALES NUMÉRICOS ====================
    /// Número con formato inválido
    /// Ejemplo: 3.14.15 (múltiples puntos decimales)
    InvalidNumberFormat { number: String, reason: String },

    /// Número con múltiples puntos decimales
    /// Ejemplo: 3.14.159
    MultipleDecimalPoints { number: String },

    /// Punto decimal al final sin dígitos
    /// Ejemplo: 42.
    TrailingDecimalPoint { number: String },

    /// Punto decimal al inicio sin dígitos
    /// Ejemplo: .42 (si el lenguaje no lo permite)
    LeadingDecimalPoint,

    /// Exponente inválido en notación científica
    /// Ejemplo: 1e, 1e+, 1e-
    InvalidExponent { number: String },

    /// Número demasiado grande (overflow)
    NumberOverflow { number: String },

    // ==================== COMENTARIOS ====================
    /// Comentario de bloque que nunca se cierra
    /// Ejemplo: /* esto nunca termina
    UnterminatedBlockComment {
        start_line: usize,
        start_column: usize,
        nesting_level: usize, // Si soportamos comentarios anidados
    },

    /// Cierre de comentario sin apertura
    /// Ejemplo: */ sin un /* previo
    UnmatchedCommentClose,

    // ==================== CARACTERES/LITERALES CHAR ====================
    /// Literal de caracter vacío
    /// Ejemplo: ''
    EmptyCharLiteral,

    /// Literal de caracter con múltiples caracteres
    /// Ejemplo: 'abc'
    MultiCharacterLiteral { content: String },

    /// Literal de caracter sin cerrar
    /// Ejemplo: 'a
    UnterminatedCharLiteral {
        start_line: usize,
        start_column: usize,
    },

    // ==================== IDENTIFICADORES ====================
    /// Identificador que empieza con número
    /// Ejemplo: 123abc como identificador
    IdentifierStartsWithDigit { identifier: String },

    /// Identificador demasiado largo (si hay límite)
    IdentifierTooLong {
        identifier: String,
        max_length: usize,
    },

    /// Identificador que empieza con guion bajo
    /// Ejemplo: _x, _value
    IdentifierStartsWithUnderscore { identifier: String },

    // ==================== OPERADORES ====================
    /// Operador incompleto o inválido
    /// Ejemplo: & cuando se esperaba &&
    IncompleteOperator { found: String, expected: String },

    // ==================== FIN DE ARCHIVO ====================
    /// Fin de archivo inesperado (en medio de algo)
    UnexpectedEOF {
        expected: String, // Qué se esperaba encontrar
    },

    // ==================== ENCODING ====================
    /// Caracter no UTF-8 válido
    InvalidUtf8 { byte_position: usize },

    /// BOM (Byte Order Mark) inesperado
    UnexpectedBOM,
}

impl DisplayError for LexerError {
    /// Retorna el código de error (E0xxx)
    fn code(&self) -> &'static str {
        match self {
            LexerError::UnexpectedCharacter(_) => "E0001",
            LexerError::InvalidCharacterInNumber { .. } => "E0002",
            LexerError::InvalidCharacterInIdentifier { .. } => "E0003",
            LexerError::UnterminatedString { .. } => "E0010",
            LexerError::InvalidEscapeSequence { .. } => "E0011",
            LexerError::UnexpectedNewlineInString { .. } => "E0012",
            LexerError::InvalidNumberFormat { .. } => "E0020",
            LexerError::MultipleDecimalPoints { .. } => "E0021",
            LexerError::TrailingDecimalPoint { .. } => "E0022",
            LexerError::LeadingDecimalPoint => "E0023",
            LexerError::InvalidExponent { .. } => "E0024",
            LexerError::NumberOverflow { .. } => "E0025",
            LexerError::UnterminatedBlockComment { .. } => "E0030",
            LexerError::UnmatchedCommentClose => "E0031",
            LexerError::EmptyCharLiteral => "E0040",
            LexerError::MultiCharacterLiteral { .. } => "E0041",
            LexerError::UnterminatedCharLiteral { .. } => "E0042",
            LexerError::IdentifierStartsWithDigit { .. } => "E0050",
            LexerError::IdentifierTooLong { .. } => "E0051",
            LexerError::IdentifierStartsWithUnderscore { .. } => "E0052",
            LexerError::IncompleteOperator { .. } => "E0060",
            LexerError::UnexpectedEOF { .. } => "E0070",
            LexerError::InvalidUtf8 { .. } => "E0080",
            LexerError::UnexpectedBOM => "E0081",
        }
    }

    /// Retorna el mensaje de error descriptivo
    fn message(&self) -> String {
        match self {
            LexerError::UnexpectedCharacter(c) => {
                format!("caracter inesperado '{}'", c)
            }
            LexerError::InvalidCharacterInNumber {
                found,
                partial_number,
            } => {
                format!(
                    "caracter inválido '{}' en número '{}'",
                    found, partial_number
                )
            }
            LexerError::InvalidCharacterInIdentifier {
                found,
                partial_identifier,
            } => {
                format!(
                    "caracter inválido '{}' en identificador '{}'",
                    found, partial_identifier
                )
            }
            LexerError::UnterminatedString {
                start_line,
                start_column,
            } => {
                format!(
                    "string sin cerrar (comenzó en línea {}, columna {})",
                    start_line, start_column
                )
            }
            LexerError::InvalidEscapeSequence { found, .. } => {
                format!("secuencia de escape inválida '\\{}'", found)
            }
            LexerError::UnexpectedNewlineInString { line } => {
                format!(
                    "salto de línea inesperado en string (línea {}). Usa \\n para incluir saltos de línea",
                    line
                )
            }
            LexerError::InvalidNumberFormat { number, reason } => {
                format!("formato de número inválido '{}': {}", number, reason)
            }
            LexerError::MultipleDecimalPoints { number } => {
                format!("múltiples puntos decimales en número '{}'", number)
            }
            LexerError::TrailingDecimalPoint { number } => {
                format!("punto decimal al final sin dígitos en '{}'", number)
            }
            LexerError::LeadingDecimalPoint => "punto decimal al inicio sin dígitos".to_string(),
            LexerError::InvalidExponent { number } => {
                format!("exponente inválido en número '{}'", number)
            }
            LexerError::NumberOverflow { number } => {
                format!("número demasiado grande '{}'", number)
            }
            LexerError::UnterminatedBlockComment {
                start_line,
                start_column,
                ..
            } => {
                format!(
                    "comentario de bloque sin cerrar (comenzó en línea {}, columna {})",
                    start_line, start_column
                )
            }
            LexerError::UnmatchedCommentClose => {
                "cierre de comentario '*/' sin apertura '/*'".to_string()
            }
            LexerError::EmptyCharLiteral => "literal de caracter vacío ''".to_string(),
            LexerError::MultiCharacterLiteral { content } => {
                format!("literal de caracter con múltiples caracteres '{}'", content)
            }
            LexerError::UnterminatedCharLiteral {
                start_line,
                start_column,
            } => {
                format!(
                    "literal de caracter sin cerrar (comenzó en línea {}, columna {})",
                    start_line, start_column
                )
            }
            LexerError::IdentifierStartsWithDigit { identifier } => {
                format!(
                    "identificador '{}' no puede empezar con un dígito",
                    identifier
                )
            }
            LexerError::IdentifierTooLong {
                identifier,
                max_length,
            } => {
                format!(
                    "identificador '{}' excede el límite de {} caracteres",
                    identifier, max_length
                )
            }
            LexerError::IdentifierStartsWithUnderscore { identifier } => {
                format!("identificador '{}' no puede empezar con '_'", identifier)
            }
            LexerError::IncompleteOperator { found, expected } => {
                format!(
                    "operador incompleto '{}', ¿quisiste escribir '{}'?",
                    found, expected
                )
            }
            LexerError::UnexpectedEOF { expected } => {
                format!("fin de archivo inesperado, se esperaba {}", expected)
            }
            LexerError::InvalidUtf8 { byte_position } => {
                format!(
                    "secuencia de bytes UTF-8 inválida en posición {}",
                    byte_position
                )
            }
            LexerError::UnexpectedBOM => {
                "BOM (Byte Order Mark) inesperado en el archivo".to_string()
            }
        }
    }

    /// Retorna una sugerencia de cómo arreglar el error (si aplica)
    fn help(&self) -> Option<String> {
        match self {
            LexerError::InvalidEscapeSequence { found: _, .. } => Some(
                "secuencias de escape válidas: \\n, \\t, \\r, \\\\, \\\", \\0. \
                    Si quieres el caracter '\\' literal, usa '\\\\'."
                    .to_string(),
            ),
            LexerError::UnterminatedString { .. } => {
                Some("asegúrate de cerrar el string con '\"'".to_string())
            }
            LexerError::UnterminatedBlockComment { .. } => {
                Some("asegúrate de cerrar el comentario con '*/'".to_string())
            }
            LexerError::TrailingDecimalPoint { .. } => {
                Some("agrega dígitos después del punto o elimina el punto".to_string())
            }
            LexerError::LeadingDecimalPoint => {
                Some("agrega un 0 antes del punto: 0.42".to_string())
            }
            LexerError::EmptyCharLiteral => {
                Some("un literal de caracter debe contener exactamente un caracter".to_string())
            }
            LexerError::MultiCharacterLiteral { .. } => {
                Some("usa comillas dobles \"...\" para strings de múltiples caracteres".to_string())
            }
            LexerError::IdentifierStartsWithDigit { .. } => {
                Some("los identificadores deben empezar con una letra o '_'".to_string())
            }
            LexerError::IdentifierStartsWithUnderscore { .. } => {
                Some("los identificadores deben empezar con una letra, no con '_'".to_string())
            }
            _ => None,
        }
    }
}
