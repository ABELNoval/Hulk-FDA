// =============================================================================
// Semantic Analyzer (Analizador Semántico Principal)
// =============================================================================
//
// Responsabilidad del Líder:
// - Orquestar la ejecución de los análisis semánticos
// - Coordinar Symbol Table, Type System, Expression Checker
// - Verificar declaraciones (funciones, tipos, protocolos)
// - Verificar expresión de entrada
// - Recopilar y reportar errores
// - Mantener contexto semántico global
//
// Esta es la interfaz pública del módulo semántico.
//
// =============================================================================

use crate::parser::ast::Program;
use crate::semantic::expression_checker::ExpressionChecker;
use crate::semantic::symbol_table::SymbolTable;
use crate::semantic::type_system::TypeEnvironment;
use crate::utils::errors::semantic::SemanticError;

type SemanticResult<T> = Result<T, SemanticError>;

/// Contexto semántico global
///
/// Contiene toda la información acumulada durante el análisis semántico:
/// - Tabla de símbolos con scopes
/// - Entorno de tipos (tipos, protocolos)
/// - Errores encontrados
pub struct SemanticContext {
    /// Tabla de símbolos con gestión de scopes
    pub symbols: SymbolTable,
    /// Entorno de tipos (tipos definidos, protocolos)
    pub types: TypeEnvironment,
    /// Checker de tipos de expresiones
    pub expression_checker: ExpressionChecker,
    /// Errores semánticos encontrados
    pub errors: Vec<SemanticError>,
}

impl SemanticContext {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            types: TypeEnvironment::new(),
            expression_checker: ExpressionChecker::new(),
            errors: Vec::new(),
        }
    }

    /// Registra un error sin detener el análisis
    pub fn push_error(&mut self, error: SemanticError) {
        self.errors.push(error);
    }

    /// Retorna true si hay errores
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Retorna todos los errores encontrados
    pub fn get_errors(&self) -> &[SemanticError] {
        &self.errors
    }

    /// Limpia la lista de errores
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }
}

impl Default for SemanticContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Analizador semántico principal
///
/// Orquesta toda la fase semántica:
/// 1. Verificación de declaraciones (funciones, tipos, protocolos)
/// 2. Verificación de expresión de entrada
/// 3. Recopilación de errores
pub struct SemanticAnalyzer {
    context: SemanticContext,
}

impl SemanticAnalyzer {
    /// Crea un nuevo analizador semántico
    pub fn new() -> Self {
        let mut context = SemanticContext::new();
        // TODO: Declarar builtins (print, sqrt, sin, cos, log, exp, rand)
        context.symbols.declare_builtins();
        Self { context }
    }

    /// Retorna una referencia al contexto
    pub fn context(&self) -> &SemanticContext {
        &self.context
    }

    /// Retorna una referencia mutable al contexto
    pub fn context_mut(&mut self) -> &mut SemanticContext {
        &mut self.context
    }

    /// Analiza un programa completo
    ///
    /// Pasos:
    /// 1. Procesar todas las declaraciones (functions, types, protocols)
    /// 2. Procesar expresión de entrada (si existe)
    /// 3. Retornar lista de errores (si las hay)
    pub fn analyze(&mut self, program: &Program) -> SemanticResult<()> {
        self.check_declarations(program)?;
        self.check_entry_expression(program)?;

        if self.context.has_errors() {
            return Err(self.context.errors[0].clone());
        }

        Ok(())
    }

    /// Verifica las declaraciones del programa
    ///
    /// Procesa en orden:
    /// 1. Todas las declaraciones de tipos
    /// 2. Todas las declaraciones de protocolos
    /// 3. Todas las declaraciones de funciones
    fn check_declarations(&mut self, _program: &Program) -> SemanticResult<()> {
        // TODO: Implementar verificación de declaraciones
        Ok(())
    }

    /// Verifica la expresión de entrada del programa
    fn check_entry_expression(&mut self, _program: &Program) -> SemanticResult<()> {
        // TODO: Implementar verificación de expresión de entrada
        Ok(())
    }

    /// Retorna todos los errores encontrados
    pub fn errors(&self) -> &[SemanticError] {
        self.context.get_errors()
    }

    /// Retorna true si hay errores
    pub fn has_errors(&self) -> bool {
        self.context.has_errors()
    }

    /// Reporta un error
    pub fn report_error(&mut self, error: SemanticError) {
        self.context.push_error(error);
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
// tests moved to consolidated `tests.rs`
