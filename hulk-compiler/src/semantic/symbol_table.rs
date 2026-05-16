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

// =============================================================================
// Tests (Persona 1)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_table_new() {
        let table = SymbolTable::new();
        assert_eq!(table.scope_depth(), 1);
    }

    #[test]
    fn test_enter_exit_scope() {
        let mut table = SymbolTable::new();
        assert_eq!(table.scope_depth(), 1);

        table.enter_scope();
        assert_eq!(table.scope_depth(), 2);

        table.exit_scope();
        assert_eq!(table.scope_depth(), 1);
    }

    #[test]
    fn test_declare_and_lookup() {
        let mut table = SymbolTable::new();
        let span = Span::default();

        let var = SymbolInfo::Variable {
            name: "x".to_string(),
            type_ref: None,
            span: span.clone(),
        };

        table.declare(var).unwrap();
        assert!(table.lookup("x").is_some());
        assert!(table.lookup("y").is_none());
    }

    #[test]
    fn test_duplicate_declaration_error() {
        let mut table = SymbolTable::new();
        let span = Span::default();

        let var1 = SymbolInfo::Variable {
            name: "x".to_string(),
            type_ref: None,
            span: span.clone(),
        };

        let var2 = SymbolInfo::Variable {
            name: "x".to_string(),
            type_ref: None,
            span: span.clone(),
        };

        table.declare(var1).unwrap();
        assert!(table.declare(var2).is_err());
    }

    #[test]
    fn test_scope_shadowing() {
        let mut table = SymbolTable::new();
        let span = Span::default();

        let var1 = SymbolInfo::Variable {
            name: "x".to_string(),
            type_ref: None,
            span: span.clone(),
        };

        table.declare(var1).unwrap();

        table.enter_scope();
        let var2 = SymbolInfo::Variable {
            name: "x".to_string(),
            type_ref: None,
            span: span.clone(),
        };

        // En nuevo scope, puedo redeclarar "x" (shadowing)
        assert!(table.declare(var2).is_ok());

        // lookup() encuentra el del scope actual
        assert!(table.lookup("x").is_some());

        table.exit_scope();
        // Sigo encontrando la variable original
        assert!(table.lookup("x").is_some());
    }

    // =======================================================================
    // Tests para Subtarea 2: Symbol Declaration & Duplicate Detection
    // =======================================================================

    #[test]
    fn test_declare_multiple_different_symbols() {
        // Verifica que se pueden declarar múltiples símbolos diferentes en el mismo scope
        let mut table = SymbolTable::new();
        let span = Span::default();

        let var_x = SymbolInfo::Variable {
            name: "x".to_string(),
            type_ref: None,
            span: span.clone(),
        };

        let var_y = SymbolInfo::Variable {
            name: "y".to_string(),
            type_ref: None,
            span: span.clone(),
        };

        let var_z = SymbolInfo::Variable {
            name: "z".to_string(),
            type_ref: None,
            span: span.clone(),
        };

        // Declara todas exitosamente
        assert!(table.declare(var_x).is_ok());
        assert!(table.declare(var_y).is_ok());
        assert!(table.declare(var_z).is_ok());

        // Todas están accesibles
        assert!(table.lookup("x").is_some());
        assert!(table.lookup("y").is_some());
        assert!(table.lookup("z").is_some());
    }

    #[test]
    fn test_declare_different_symbol_types() {
        // Verifica que se pueden declarar diferentes tipos de símbolos
        let mut table = SymbolTable::new();
        let span = Span::default();

        // Variable
        let var = SymbolInfo::Variable {
            name: "my_var".to_string(),
            type_ref: None,
            span: span.clone(),
        };

        // Parameter
        let param = SymbolInfo::Parameter {
            name: "my_param".to_string(),
            type_ref: None,
            span: span.clone(),
        };

        // Function
        let func = SymbolInfo::Function {
            name: "my_func".to_string(),
            parameters: Vec::new(),
            return_type: None,
            span: span.clone(),
        };

        // Type
        let type_sym = SymbolInfo::Type {
            name: "my_type".to_string(),
            span: span.clone(),
        };

        // Protocol
        let proto = SymbolInfo::Protocol {
            name: "my_protocol".to_string(),
            span: span.clone(),
        };

        // Todos se declaran exitosamente
        assert!(table.declare(var).is_ok());
        assert!(table.declare(param).is_ok());
        assert!(table.declare(func).is_ok());
        assert!(table.declare(type_sym).is_ok());
        assert!(table.declare(proto).is_ok());

        // Todos están accesibles
        assert!(table.lookup("my_var").is_some());
        assert!(table.lookup("my_param").is_some());
        assert!(table.lookup("my_func").is_some());
        assert!(table.lookup("my_type").is_some());
        assert!(table.lookup("my_protocol").is_some());
    }

    #[test]
    fn test_duplicate_with_different_types() {
        // Verifica que incluso si dos símbolos tienen diferente tipo (Variable vs Parameter),
        // no se puede duplicar el nombre en el mismo scope
        let mut table = SymbolTable::new();
        let span = Span::default();

        let var = SymbolInfo::Variable {
            name: "x".to_string(),
            type_ref: None,
            span: span.clone(),
        };

        let param = SymbolInfo::Parameter {
            name: "x".to_string(), // Mismo nombre
            type_ref: None,
            span: span.clone(),
        };

        // Declara la variable exitosamente
        assert!(table.declare(var).is_ok());

        // Intenta declarar un parámetro con el mismo nombre → debe fallar
        assert!(table.declare(param).is_err());
    }

    #[test]
    fn test_declare_same_name_different_scopes() {
        // Verifica que se puede usar el mismo nombre en diferentes scopes (shadowing permitido)
        let mut table = SymbolTable::new();
        let span = Span::default();

        // Scope global
        let var1 = SymbolInfo::Variable {
            name: "value".to_string(),
            type_ref: None,
            span: span.clone(),
        };
        assert!(table.declare(var1).is_ok());

        // Primer scope local
        table.enter_scope();
        let var2 = SymbolInfo::Variable {
            name: "value".to_string(), // Mismo nombre
            type_ref: None,
            span: span.clone(),
        };
        assert!(table.declare(var2).is_ok()); // ✓ OK (different scope)

        // Segundo scope local
        table.enter_scope();
        let var3 = SymbolInfo::Variable {
            name: "value".to_string(), // Mismo nombre otra vez
            type_ref: None,
            span: span.clone(),
        };
        assert!(table.declare(var3.clone()).is_ok()); // ✓ OK (different scope again)

        // Intenta declarar de nuevo en el mismo scope → debe fallar
        assert!(table.declare(var3.clone()).is_err());

        table.exit_scope();
        table.exit_scope();

        // Después de salir, el global sigue existiendo
        assert!(table.lookup("value").is_some());
    }

    #[test]
    fn test_declare_error_contains_symbol_name() {
        // Verifica que el mensaje de error contiene el nombre del símbolo
        let mut table = SymbolTable::new();
        let span = Span::default();

        let var1 = SymbolInfo::Variable {
            name: "duplicate_name".to_string(),
            type_ref: None,
            span: span.clone(),
        };

        let var2 = SymbolInfo::Variable {
            name: "duplicate_name".to_string(),
            type_ref: None,
            span: span.clone(),
        };

        assert!(table.declare(var1).is_ok());
        
        let result = table.declare(var2);
        assert!(result.is_err());
        
        // Verifica que el error message contiene el nombre
        if let Err(msg) = result {
            assert!(msg.contains("duplicate_name"));
        }
    }

    #[test]
    fn test_declare_lookup_consistency() {
        // Verifica que una vez que se declara algo, siempre se puede buscar
        // y que tiene la información correcta
        let mut table = SymbolTable::new();
        let span = Span::default();

        let var = SymbolInfo::Variable {
            name: "test_var".to_string(),
            type_ref: None,
            span: span.clone(),
        };

        assert!(table.declare(var.clone()).is_ok());

        // Busca la variable
        let found = table.lookup("test_var");
        assert!(found.is_some());

        // Verifica que el símbolo encontrado es el correcto
        if let Some(SymbolInfo::Variable { name, .. }) = found {
            assert_eq!(name, "test_var");
        } else {
            panic!("Expected Variable, got something else");
        }
    }

    #[test]
    fn test_declare_in_multiple_scopes_independent() {
        // Verifica que cada scope tiene su propia tabla de símbolos
        // y no interfieren entre sí
        let mut table = SymbolTable::new();
        let span = Span::default();

        // Global scope
        let global_x = SymbolInfo::Variable {
            name: "x".to_string(),
            type_ref: None,
            span: span.clone(),
        };
        table.declare(global_x).unwrap();

        // Local scope 1
        table.enter_scope();
        let local1_y = SymbolInfo::Variable {
            name: "y".to_string(),
            type_ref: None,
            span: span.clone(),
        };
        table.declare(local1_y).unwrap();

        // x no debe estar en local scope 1 (lookup_local)
        assert!(table.lookup_local("x").is_none());
        // y sí debe estar
        assert!(table.lookup_local("y").is_some());
        // pero lookup vuelve al global y encuentra x
        assert!(table.lookup("x").is_some());

        // Local scope 2
        table.enter_scope();
        let local2_z = SymbolInfo::Variable {
            name: "z".to_string(),
            type_ref: None,
            span: span.clone(),
        };
        table.declare(local2_z).unwrap();

        // En local scope 2 solo está z
        assert!(table.lookup_local("z").is_some());
        assert!(table.lookup_local("y").is_none());
        assert!(table.lookup_local("x").is_none());

        // Pero lookup encuentra todo (z, y, x desde abajo hacia arriba)
        assert!(table.lookup("z").is_some());
        assert!(table.lookup("y").is_some());
        assert!(table.lookup("x").is_some());

        table.exit_scope();
        table.exit_scope();
    }
}
