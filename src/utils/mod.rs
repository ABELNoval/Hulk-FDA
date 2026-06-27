// =============================================================================
// UTILS - Módulo de utilidades compartidas
// =============================================================================

pub mod errors;

// Re-exportar tipos comunes para acceso fácil
pub use errors::{CompilationError, CompileResult, DisplayError, LexerError, ParserError, SemanticError};
pub use errors::span::Span;

