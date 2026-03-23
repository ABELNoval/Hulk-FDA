// =============================================================================
// Lexer (Analizador Léxico / Tokenizador)
// =============================================================================
//
// El lexer es la primera fase del compilador. Transforma el código fuente
// (una secuencia de caracteres) en una secuencia de tokens.
//
// Responsabilidades:
// - Leer el código fuente carácter por carácter
// - Identificar y clasificar tokens (palabras clave, identificadores,
//   literales, operadores, delimitadores, etc.)
// - Ignorar espacios en blanco y comentarios
// - Rastrear posición (línea, columna) para reportar errores
// - Manejar errores léxicos (caracteres inválidos, strings sin cerrar, etc.)
//
// Tipos de tokens típicos:
// - Keywords: let, if, else, while, function, etc.
// - Identificadores: nombres de variables, funciones
// - Literales: números, strings, booleanos
// - Operadores: +, -, *, /, ==, !=, <, >, etc.
// - Delimitadores: (, ), {, }, [, ], ;, ,
// - EOF: fin de archivo
//
// =============================================================================

pub mod token;

// Re-exportar tipos principales para facilitar el uso
pub use token::{Token, TokenType};
