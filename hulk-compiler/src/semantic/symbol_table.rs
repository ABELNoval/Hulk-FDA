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
use crate::utils::errors::semantic::SemanticError;
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
///

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
        } else {
            panic!("Cannot exit global scope");
        }
    }

   
    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }

    pub fn declare(&mut self, symbol: SymbolInfo) -> Result<(), SemanticError> {
        let scope = self.scopes.last_mut().expect("At least global scope exists");

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
                    return Err(SemanticError::TypeAlreadyDeclared {
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

    /// Lista todos los símbolos en el scope actual
    pub fn symbols_in_current_scope(&self) -> Vec<SymbolInfo> {
        self.scopes
            .last()
            .map(|scope| scope.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Verifica si un símbolo está declarado en algún scope (sin retornarlo)

    pub fn is_symbol_declared(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Verifica si estamos en el scope global
  
    pub fn is_global_scope(&self) -> bool {
        self.scope_depth() == 1
    }

    /// Retorna el símbolo o un error descriptivo (usando String)
    pub fn get_symbol(&self, name: &str) -> Result<SymbolInfo, String> {
        self.lookup(name)
            .ok_or_else(|| format!("Symbol '{}' is not declared", name))
    }

    /// Retorna el símbolo o un SemanticError si no está declarado
    /// Determina automáticamente qué tipo de error basado en convenciones de nombres
    pub fn get_symbol_or_error(&self, name: &str) -> Result<SymbolInfo, SemanticError> {
        self.lookup(name).ok_or_else(|| {
            // Heurística: si empieza con mayúscula, probablemente sea un tipo/protocolo
            if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                SemanticError::UndeclaredType {
                    name: name.to_string(),
                }
            } else {
                // Si empieza con minúscula, probablemente sea una variable/función
                SemanticError::UndeclaredVariable {
                    name: name.to_string(),
                }
            }
        })
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

    // ========== SAFE PUBLIC API FOR OTHER MODULES ==========
    // Estos métodos forman la interfaz segura para que otros módulos
    // (expression_checker, analyzer) consulten la tabla de símbolos

    /// Verifica si un símbolo es una función
    pub fn is_function(&self, name: &str) -> bool {
        matches!(self.lookup(name), Some(SymbolInfo::Function { .. }))
    }

    /// Verifica si un símbolo es una variable
    pub fn is_variable(&self, name: &str) -> bool {
        matches!(self.lookup(name), Some(SymbolInfo::Variable { .. }))
    }

    /// Verifica si un símbolo es un parámetro
    pub fn is_parameter(&self, name: &str) -> bool {
        matches!(self.lookup(name), Some(SymbolInfo::Parameter { .. }))
    }

    /// Verifica si un símbolo es un tipo
    pub fn is_type(&self, name: &str) -> bool {
        matches!(self.lookup(name), Some(SymbolInfo::Type { .. }))
    }

    /// Verifica si un símbolo es un protocolo
    pub fn is_protocol(&self, name: &str) -> bool {
        matches!(self.lookup(name), Some(SymbolInfo::Protocol { .. }))
    }

    /// Retorna el nombre del tipo de símbolo ("function", "variable", "type", etc.)
    pub fn get_symbol_kind(&self, name: &str) -> Option<&'static str> {
        match self.lookup(name) {
            Some(SymbolInfo::Function { .. }) => Some("function"),
            Some(SymbolInfo::Variable { .. }) => Some("variable"),
            Some(SymbolInfo::Parameter { .. }) => Some("parameter"),
            Some(SymbolInfo::Type { .. }) => Some("type"),
            Some(SymbolInfo::Protocol { .. }) => Some("protocol"),
            None => None,
        }
    }

    /// Retorna todas las funciones declaradas en el scope actual
    pub fn functions_in_scope(&self) -> Vec<SymbolInfo> {
        self.symbols_in_current_scope()
            .into_iter()
            .filter(|s| matches!(s, SymbolInfo::Function { .. }))
            .collect()
    }

    /// Retorna todas las variables en el scope actual
    pub fn variables_in_scope(&self) -> Vec<SymbolInfo> {
        self.symbols_in_current_scope()
            .into_iter()
            .filter(|s| matches!(s, SymbolInfo::Variable { .. }))
            .collect()
    }

    /// Retorna todos los parámetros en el scope actual
    pub fn parameters_in_scope(&self) -> Vec<SymbolInfo> {
        self.symbols_in_current_scope()
            .into_iter()
            .filter(|s| matches!(s, SymbolInfo::Parameter { .. }))
            .collect()
    }

    /// Retorna todas las funciones globales (útil para buscar funciones disponibles)
    pub fn global_functions(&self) -> Vec<SymbolInfo> {
        self.global_symbols()
            .into_iter()
            .filter(|s| matches!(s, SymbolInfo::Function { .. }))
            .collect()
    }

    /// Retorna todos los tipos globales
    pub fn global_types(&self) -> Vec<SymbolInfo> {
        self.global_symbols()
            .into_iter()
            .filter(|s| matches!(s, SymbolInfo::Type { .. }))
            .collect()
    }

    /// Retorna todos los protocolos globales
    pub fn global_protocols(&self) -> Vec<SymbolInfo> {
        self.global_symbols()
            .into_iter()
            .filter(|s| matches!(s, SymbolInfo::Protocol { .. }))
            .collect()
    }

    /// Cuenta cuántos símbolos hay en el scope actual
    pub fn symbol_count_in_scope(&self) -> usize {
        self.symbols_in_current_scope().len()
    }

    /// Cuenta cuántos símbolos hay en total (todos los scopes)
    pub fn total_symbol_count(&self) -> usize {
        self.all_symbols_in_chain().len()
    }

    /// Cuenta cuántos símbolos hay en global scope
    pub fn global_symbol_count(&self) -> usize {
        self.global_symbols().len()
    }

    /// Resuelve un símbolo de forma segura, retornando información completa
    /// Útil para el analyzer cuando quiere saber todo sobre un símbolo
    pub fn resolve_symbol_info(&self, name: &str) -> Option<(SymbolInfo, usize, &'static str)> {
        self.lookup(name).map(|sym| {
            let kind = match &sym {
                SymbolInfo::Function { .. } => "function",
                SymbolInfo::Variable { .. } => "variable",
                SymbolInfo::Parameter { .. } => "parameter",
                SymbolInfo::Type { .. } => "type",
                SymbolInfo::Protocol { .. } => "protocol",
            };
            
            // Encontrar en qué scope está
            for (scope_idx, scope) in self.scopes.iter().enumerate().rev() {
                if scope.contains_key(name) {
                    return (sym, scope_idx, kind);
                }
            }
            
            (sym, 0, kind)  // Si no lo encuentra (no debería pasar), retorna scope 0
        })
    }

    /// Verifica si un símbolo está disponible (existe y es accesible)
    /// Retorna (existe, es_local, es_global)
    pub fn symbol_availability(&self, name: &str) -> (bool, bool, bool) {
        let exists = self.is_symbol_declared(name);
        let is_local = self.lookup_local(name).is_some();
        let is_global = self.scopes.first().map_or(false, |g| g.contains_key(name));
        
        (exists, is_local, is_global)
    }

    /// Obtiene la información de tipo de un símbolo (si la tiene)
    pub fn get_type_reference(&self, name: &str) -> Option<TypeReference> {
        self.lookup(name).and_then(|sym| {
            match sym {
                SymbolInfo::Variable { type_ref, .. } => type_ref,
                SymbolInfo::Parameter { type_ref, .. } => type_ref,
                SymbolInfo::Function { return_type, .. } => return_type,
                _ => None,
            }
        })
    }

    /// Verifica si el símbolo existe en el scope global específicamente
    pub fn exists_in_global(&self, name: &str) -> bool {
        self.scopes.first().map_or(false, |g| g.contains_key(name))
    }

    /// Verifica si el símbolo existe SOLO en scopes locales (no en global)
    pub fn exists_only_locally(&self, name: &str) -> bool {
        self.is_symbol_declared(name) && !self.exists_in_global(name)
    }

    /// Declara los símbolos builtin globales (print, sqrt, sin, cos, log, exp, rand)
  
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
