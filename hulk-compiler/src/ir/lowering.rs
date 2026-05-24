// Persona 3 — AST lowering into IR
// Assigned: Persona3
// Responsibilities: implementar la pasarela que convierte `Program` (AST)
// en `IRModule`. Manejar lowering de expresiones, variables, control flow y
// conectar con el IR builder sin exponer detalles internos.

use crate::parser::ast::Program;

use super::module::IRModule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IRLoweringError {
    pub message: String,
    pub location: Option<String>,
}

impl IRLoweringError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: None,
        }
    }

    pub fn at(message: impl Into<String>, location: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: Some(location.into()),
        }
    }
}

impl std::fmt::Display for IRLoweringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.location {
            Some(location) => write!(f, "{} ({})", self.message, location),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for IRLoweringError {}

pub trait IRLoweringContext {
    fn lower_program(&mut self, program: &Program) -> Result<IRModule, IRLoweringError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRLoweringResult {
    pub module: IRModule,
    pub warnings: Vec<String>,
}

impl IRLoweringResult {
    pub fn new(module: IRModule) -> Self {
        Self {
            module,
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}
