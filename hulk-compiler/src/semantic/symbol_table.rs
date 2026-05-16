// =============================================================================
// Symbol Table & Scope Management
// =============================================================================
//
// Responsabilidad de Persona 1:
// - Mantener tabla de símbolos con declaraciones (variables, funciones, tipos)
// - Gestionar scopes (let expressions, function bodies, etc.)
// - Resolver referencias a símbolos (lookup)
// - Detectar duplicados, undeclared, etc.
//
// Esta es la interfaz compartida. Los métodos son stubs que Persona 1 implementará.
//
// =============================================================================

use crate::parser::ast::{Parameter, TypeReference};
use crate::utils::errors::span::Span;
use std::collections::HashMap;

/// Información sobre un símbolo en la tabla
#[derive(Debug, Clone)]
pub enum SymbolInfo {
    /// Declaración de variable con su tipo
    Variable {
        name: String,
        type_ref: Option<TypeReference>,
        span: Span,
    },
    /// Parámetro de función con su tipo
    Parameter {
        name: String,
        type_ref: Option<TypeReference>,
        span: Span,
    },
    /// Declaración de función
    Function {
        name: String,
        parameters: Vec<Parameter>,
        return_type: Option<TypeReference>,
        span: Span,
    },
    /// Declaración de tipo (type)
    Type {
        name: String,
        span: Span,
    },
    /// Declaración de protocolo
    Protocol {
        name: String,
        span: Span,
    },
}

impl SymbolInfo {
    pub fn name(&self) -> &str {
        match self {
            SymbolInfo::Variable { name, .. } => name,
            SymbolInfo::Parameter { name, .. } => name,
            SymbolInfo::Function { name, .. } => name,
            SymbolInfo::Type { name, .. } => name,
            SymbolInfo::Protocol { name, .. } => name,
        }
    }

    pub fn span(&self) -> &Span {
        match self {
            SymbolInfo::Variable { span, .. } => span,
            SymbolInfo::Parameter { span, .. } => span,
            SymbolInfo::Function { span, .. } => span,
            SymbolInfo::Type { span, .. } => span,
            SymbolInfo::Protocol { span, .. } => span,
        }
    }
}

/// Tabla de símbolos con gestión de scopes
///
/// Mantiene múltiples niveles de scopes (jerarquía).
/// - Scope 0: global (builtins, declaraciones de funciones/tipos/protocolos)
/// - Scope 1+: locales (let expressions, function bodies)
pub struct SymbolTable {
    /// Stack de scopes. El primero es el global, los demás son locales.
    scopes: Vec<HashMap<String, SymbolInfo>>,
}

impl SymbolTable {
    /// Crea una nueva tabla de símbolos vacía (con un scope global)
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    /// Ingresa a un nuevo scope (por ejemplo, al entrar a un let o función)
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Sale del scope actual (por ejemplo, al salir de un let o función)
    ///
    /// # Panics
    /// Si se intenta salir del scope global
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        } else {
            panic!("Cannot exit global scope");
        }
    }

    /// Retorna la profundidad actual de scopes
    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }

    /// Declara un nuevo símbolo en el scope actual
    ///
    /// Retorna error si ya existe un símbolo con el mismo nombre en el scope actual.
    /// (Nota: puede existir con el mismo nombre en otros scopes, eso es shadowing)
    pub fn declare(&mut self, symbol: SymbolInfo) -> Result<(), String> {
        let scope = self.scopes.last_mut().expect("At least global scope exists");

        if scope.contains_key(symbol.name()) {
            return Err(format!("Symbol '{}' already declared in this scope", symbol.name()));
        }

        scope.insert(symbol.name().to_string(), symbol);
        Ok(())
    }

    /// Busca un símbolo en la tabla
    ///
    /// Comienza en el scope actual y sube recursivamente hasta el global.
    /// Retorna None si el símbolo no está declarado en ningún scope.
    pub fn lookup(&self, name: &str) -> Option<SymbolInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol.clone());
            }
        }
        None
    }

    /// Busca un símbolo solo en el scope actual (no en padres)
    pub fn lookup_local(&self, name: &str) -> Option<SymbolInfo> {
        self.scopes
            .last()
            .and_then(|scope| scope.get(name).cloned())
    }

    /// Lista todos los símbolos en el scope actual
    pub fn symbols_in_current_scope(&self) -> Vec<SymbolInfo> {
        self.scopes
            .last()
            .map(|scope| scope.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Declara los símbolos builtin globales (print, sqrt, sin, cos, log, exp, rand)
    ///
    /// Esta función se llama una sola vez al inicializar el analizador semántico.
    /// Los builtins no tienen tipos anotados (se detectan en tiempo de ejecución).
    pub fn declare_builtins(&mut self) {
        // TODO: Implementar declaración de builtins
        // - print(value: ?)
        // - sqrt(x: Number) -> Number
        // - sin(x: Number) -> Number
        // - cos(x: Number) -> Number
        // - log(x: Number) -> Number
        // - exp(x: Number) -> Number
        // - rand() -> Number
        //
        // Y constantes:
        // - PI: Number
        // - E: Number
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
