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
use crate::semantic::type_system::NormalizedType;
use crate::utils::errors::semantic::SemanticError;
use crate::utils::errors::span::Span;
use std::collections::HashMap;

/// Información sobre un símbolo en la tabla
#[derive(Debug, Clone)]
pub enum SymbolInfo {
    /// Declaración de variable con su tipo
    Variable {
        name: String,
        type_ref: NormalizedType,
        span: Span,
    },
    /// Parámetro de función con su tipo
    Parameter {
        name: String,
        type_ref: NormalizedType,
        span: Span,
    },
    /// Declaración de función
    Function {
        name: String,
        parameters: Vec<Parameter>,
        return_type: NormalizedType,
        span: Span,
    },
    /// Declaración de tipo (type)
    Type { name: String, span: Span },
    /// Declaración de protocolo
    Protocol { name: String, span: Span },
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
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }

    pub fn declare(&mut self, symbol: SymbolInfo) -> Result<(), SemanticError> {
        let scope = self
            .scopes
            .last_mut()
            .expect("At least global scope exists");

        if let Some(existing) = scope.get(symbol.name()) {
            // Retornar error específico basado en el tipo de símbolo
            match &symbol {
                SymbolInfo::Function { name, .. } => {
                    return Err(SemanticError::FunctionAlreadyDeclared {
                        name: name.clone(),
                        first_line: existing.span().start_line,
                        first_column: existing.span().start_column,
                    });
                }
                SymbolInfo::Type { name, .. } => {
                    return Err(SemanticError::TypeAlreadyDeclared {
                        name: name.clone(),
                        first_line: existing.span().start_line,
                        first_column: existing.span().start_column,
                    });
                }
                SymbolInfo::Variable { name, .. } | SymbolInfo::Parameter { name, .. } => {
                    return Err(SemanticError::VariableAlreadyDeclared {
                        name: name.clone(),
                        first_line: existing.span().start_line,
                        first_column: existing.span().start_column,
                    });
                }
                SymbolInfo::Protocol { name, .. } => {
                    return Err(SemanticError::ProtocolAlreadyDeclared {
                        name: name.clone(),
                        first_line: existing.span().start_line,
                        first_column: existing.span().start_column,
                    });
                }
            }
        }

        scope.insert(symbol.name().to_string(), symbol);
        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<SymbolInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol.clone());
            }
        }
        None
    }

    pub fn lookup_local(&self, name: &str) -> Option<SymbolInfo> {
        self.scopes
            .last()
            .and_then(|scope| scope.get(name).cloned())
    }

    pub fn update_function_return_type(
        &mut self,
        name: &str,
        return_type: NormalizedType,
    ) -> Result<(), SemanticError> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(symbol) = scope.get_mut(name) {
                match symbol {
                    SymbolInfo::Function {
                        return_type: existing_return,
                        ..
                    } => {
                        *existing_return = return_type;
                        return Ok(());
                    }
                    _ => {
                        return Err(SemanticError::UndefinedFunction {
                            name: name.to_string(),
                        });
                    }
                }
            }
        }
        Err(SemanticError::UndefinedFunction {
            name: name.to_string(),
        })
    }

    /// Lista todos los símbolos en el scope actual
    pub fn symbols_in_current_scope(&self) -> Vec<SymbolInfo> {
        self.scopes
            .last()
            .map(|scope| scope.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Lista todos los símbolos desde el scope actual hacia el global (para debugging)
    pub fn all_symbols_in_chain(&self) -> Vec<(usize, String, SymbolInfo)> {
        let mut result = Vec::new();

        for (scope_idx, scope) in self.scopes.iter().enumerate().rev() {
            for (name, symbol) in scope {
                result.push((scope_idx, name.clone(), symbol.clone()));
            }
        }

        result
    }

    /// Lista solo los símbolos en el scope global
    pub fn global_symbols(&self) -> Vec<SymbolInfo> {
        self.scopes
            .first()
            .map(|scope| scope.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Declara los símbolos builtin globales (print, sqrt, sin, cos, log, exp, rand)
    pub fn declare_builtins(&mut self) {
        // Builtin functions
        let span = Span::default();

        // print(value: ?) -> PrintResult (handled specially in checker)
        let print_param = Parameter::new("value".into(), None, span.clone());
        let print_fn = SymbolInfo::Function {
            name: "print".into(),
            parameters: vec![print_param],
            return_type: NormalizedType::Unknown,
            span: span.clone(),
        };
        self.scopes
            .first_mut()
            .expect("global scope")
            .insert("print".into(), print_fn);

        // sqrt/sin/cos/exp: (Number) -> Number
        let num_param = Parameter::new(
            "x".into(),
            Some(TypeReference::new("Number".into(), span.clone())),
            span.clone(),
        );
        for &name in &["sqrt", "sin", "cos", "exp"] {
            let f = SymbolInfo::Function {
                name: name.into(),
                parameters: vec![num_param.clone()],
                return_type: NormalizedType::Number,
                span: span.clone(),
            };
            self.scopes
                .first_mut()
                .expect("global scope")
                .insert(name.into(), f);
        }

        // log(base: Number, value: Number) -> Number
        let log_f = SymbolInfo::Function {
            name: "log".into(),
            parameters: vec![
                Parameter::new(
                    "base".into(),
                    Some(TypeReference::new("Number".into(), span.clone())),
                    span.clone(),
                ),
                Parameter::new(
                    "value".into(),
                    Some(TypeReference::new("Number".into(), span.clone())),
                    span.clone(),
                ),
            ],
            return_type: NormalizedType::Number,
            span: span.clone(),
        };
        self.scopes
            .first_mut()
            .expect("global scope")
            .insert("log".into(), log_f);

        // rand() -> Number
        let rand_f = SymbolInfo::Function {
            name: "rand".into(),
            parameters: vec![],
            return_type: NormalizedType::Number,
            span: span.clone(),
        };
        self.scopes
            .first_mut()
            .expect("global scope")
            .insert("rand".into(), rand_f);
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
