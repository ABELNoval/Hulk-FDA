// =============================================================================
// Semantic Analyzer (Analizador Semántico)
// =============================================================================
//
// El analizador semántico verifica que el programa tenga sentido más allá de
// la sintaxis. Opera sobre el AST producido por el parser.
//
// Responsabilidades:
// - Type Checking: verificar que los tipos sean correctos
// - Scope Resolution: resolver referencias a variables y funciones
// - Symbol Table: mantener tabla de símbolos con declaraciones
// - Control Flow Analysis: verificar returns, breaks, etc.
// - Detección de errores semánticos
//
// Arquitectura:
// - error: tipos de errores semánticos
// - symbol_table: gestión de símbolos y scopes (Persona 1)
// - type_system: sistema de tipos nominal e herencia (Persona 2)
// - expression_checker: type checking de expresiones (Persona 3)
// - analyzer: orquestación de la fase semántica (Líder)
//
// El resultado es:
// - AST anotado con información de tipos
// - Tabla de símbolos completa
// - Lista de errores semánticos
//
// =============================================================================

pub mod analyzer;
pub mod expression_checker;
pub mod symbol_table;
pub mod type_system;

#[cfg(test)]
mod test_symbol_table;

#[allow(unused_imports)]
pub use analyzer::{SemanticAnalyzer, SemanticContext};
#[allow(unused_imports)]
pub use crate::utils::errors::semantic::SemanticError;
#[allow(unused_imports)]
pub use symbol_table::SymbolTable;
#[allow(unused_imports)]
pub use type_system::TypeEnvironment;
