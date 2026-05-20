// =============================================================================
// Tests for Symbol Table & Scope Management
// =============================================================================

use super::symbol_table::*;
use crate::parser::ast::TypeReference;
use crate::utils::errors::span::Span;
use crate::utils::errors::semantic::SemanticError;

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
    // Verifica que el error SemanticError contiene el nombre del símbolo
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
    
    // Verifica que el error contiene el nombre
    match result.unwrap_err() {
        SemanticError::VariableAlreadyDeclared { name, .. } => {
            assert_eq!(name, "duplicate_name");
        }
        _ => panic!("Expected VariableAlreadyDeclared error"),
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

// =======================================================================
// Tests para Subtarea 3: Symbol Lookup with Proper Shadowing
// =======================================================================

#[test]
fn test_lookup_finds_closest_symbol_shadowing() {
    // Verifica que lookup retorna el símbolo más CERCANO, no uno lejano
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Global scope: x es Variable
    let global_var = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(global_var).unwrap();

    // Local scope: x es Function (diferente tipo)
    table.enter_scope();
    let local_func = SymbolInfo::Function {
        name: "x".to_string(),
        parameters: Vec::new(),
        return_type: None,
        span: span.clone(),
    };
    table.declare(local_func).unwrap();

    // lookup("x") debe retornar la FUNCIÓN, no la variable
    let found = table.lookup("x");
    assert!(found.is_some());
    
    // Verifica que es la Function, no la Variable
    match found.unwrap() {
        SymbolInfo::Function { name, .. } => {
            assert_eq!(name, "x");
        }
        SymbolInfo::Variable { .. } => {
            panic!("ERROR: lookup returned Variable instead of Function (shadowing failed)");
        }
        _ => panic!("Unexpected symbol type"),
    }

    table.exit_scope();

    // Después de salir, lookup("x") debe retornar la Variable
    let found = table.lookup("x");
    assert!(found.is_some());
    
    match found.unwrap() {
        SymbolInfo::Variable { name, .. } => {
            assert_eq!(name, "x");
        }
        SymbolInfo::Function { .. } => {
            panic!("ERROR: lookup returned Function instead of Variable");
        }
        _ => panic!("Unexpected symbol type"),
    }
}

#[test]
fn test_lookup_returns_none_when_not_found() {
    // Verifica que lookup devuelve None si el símbolo no existe en ningún scope
    let mut table = SymbolTable::new();

    // Tabla vacía
    assert!(table.lookup("nonexistent").is_none());

    // Declara algo
    let span = Span::default();
    let var = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var).unwrap();

    // Busca algo diferente
    assert!(table.lookup("y").is_none());

    // Entra a un scope local
    table.enter_scope();
    
    // x todavía existe (en global)
    assert!(table.lookup("x").is_some());
    
    // pero z no existe en ningún lado
    assert!(table.lookup("z").is_none());

    table.exit_scope();
}

#[test]
fn test_lookup_local_only_current_scope() {
    // Verifica que lookup_local SOLO busca en el scope actual
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Global: x
    let var_x = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_x).unwrap();

    // Local 1: y
    table.enter_scope();
    let var_y = SymbolInfo::Variable {
        name: "y".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_y).unwrap();

    // En local scope 1:
    // - lookup_local("y") → encontrado (en scope actual)
    assert!(table.lookup_local("y").is_some());
    
    // - lookup_local("x") → NO encontrado (x está en parent, no en actual)
    assert!(table.lookup_local("x").is_none());
    
    // - lookup("x") → ENCONTRADO (busca en todos los scopes)
    assert!(table.lookup("x").is_some());
    
    // - lookup("y") → ENCONTRADO (busca en todos, encuentra en actual)
    assert!(table.lookup("y").is_some());

    table.exit_scope();
}

#[test]
fn test_lookup_deep_nesting_multiple_levels() {
    // Verifica que lookup funciona correctamente con profundidad > 2
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Level 0 (global): a, b
    let var_a = SymbolInfo::Variable {
        name: "a".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    let var_b = SymbolInfo::Variable {
        name: "b".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_a).unwrap();
    table.declare(var_b).unwrap();

    // Level 1: b (shadows), c
    table.enter_scope();
    let var_b_shadowed = SymbolInfo::Variable {
        name: "b".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    let var_c = SymbolInfo::Variable {
        name: "c".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_b_shadowed).unwrap();
    table.declare(var_c).unwrap();

    // Level 2: c (shadows), d
    table.enter_scope();
    let var_c_shadowed = SymbolInfo::Variable {
        name: "c".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    let var_d = SymbolInfo::Variable {
        name: "d".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_c_shadowed).unwrap();
    table.declare(var_d).unwrap();

    // En level 2, lookup_local solo tiene c y d
    assert!(table.lookup_local("d").is_some());
    assert!(table.lookup_local("c").is_some());
    assert!(table.lookup_local("b").is_none());
    assert!(table.lookup_local("a").is_none());

    // En level 2, lookup encuentra todo (busca hacia arriba)
    assert!(table.lookup("d").is_some());
    assert!(table.lookup("c").is_some()); // La versión del nivel 2 (shadowed)
    assert!(table.lookup("b").is_some()); // La versión del nivel 1 (shadowed)
    assert!(table.lookup("a").is_some()); // La versión del nivel 0 (global)

    // Salir a level 1
    table.exit_scope();

    // En level 1, lookup_local tiene b y c (no d)
    assert!(table.lookup_local("c").is_some());
    assert!(table.lookup_local("b").is_some());
    assert!(table.lookup_local("d").is_none());

    // En level 1, lookup puede encontrar c, b, a (pero no d)
    assert!(table.lookup("c").is_some()); // Versión de nivel 1
    assert!(table.lookup("b").is_some()); // Versión de nivel 1
    assert!(table.lookup("a").is_some()); // Versión de nivel 0
    assert!(table.lookup("d").is_none());  // Nunca fue declarado aquí

    // Salir a level 0 (global)
    table.exit_scope();

    // En global, solo a y b
    assert!(table.lookup_local("a").is_some());
    assert!(table.lookup_local("b").is_some());
    assert!(table.lookup_local("c").is_none());
    assert!(table.lookup_local("d").is_none());

    // lookup encuentra solo a y b
    assert!(table.lookup("a").is_some());
    assert!(table.lookup("b").is_some());
    assert!(table.lookup("c").is_none());
    assert!(table.lookup("d").is_none());
}

#[test]
fn test_lookup_skips_empty_scopes() {
    // Verifica que lookup busca correctamente saltando scopes vacíos
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Global: x
    let var_x = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_x).unwrap();

    // Local 1: vacío
    table.enter_scope();
    assert!(table.lookup_local("x").is_none()); // Vacío localmente
    assert!(table.lookup("x").is_some());        // Pero lookup encuentra x en global

    // Local 2: vacío también
    table.enter_scope();
    assert!(table.lookup_local("x").is_none()); // Vacío localmente
    assert!(table.lookup("x").is_some());        // Pero lookup encuentra x saltando scopes vacíos

    table.exit_scope();
    table.exit_scope();
}

#[test]
fn test_lookup_multiple_symbols_different_scopes() {
    // Verifica que lookup distingue correctamente entre múltiples símbolos
    // cuando hay shadowing
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Global: x=1, y=1
    let x_global = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    let y_global = SymbolInfo::Variable {
        name: "y".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(x_global).unwrap();
    table.declare(y_global).unwrap();

    // Local: x=2 (shadow), z=2
    table.enter_scope();
    let x_local = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    let z_local = SymbolInfo::Variable {
        name: "z".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(x_local).unwrap();
    table.declare(z_local).unwrap();

    // lookup("x") → encuentra x_local (shadowing)
    assert!(table.lookup("x").is_some());
    
    // lookup("y") → encuentra y_global (no está en local, salta al global)
    assert!(table.lookup("y").is_some());
    
    // lookup("z") → encuentra z_local
    assert!(table.lookup("z").is_some());
    
    // lookup_local("x") → encuentra x_local
    assert!(table.lookup_local("x").is_some());
    
    // lookup_local("y") → NO encuentra (no está en local scope)
    assert!(table.lookup_local("y").is_none());
    
    // lookup_local("z") → encuentra z_local
    assert!(table.lookup_local("z").is_some());

    table.exit_scope();

    // En global nuevamente:
    assert!(table.lookup("x").is_some()); // x_global
    assert!(table.lookup("y").is_some()); // y_global
    assert!(table.lookup("z").is_none()); // z no existe en global
}

// =======================================================================
// Tests para Subtarea 4: Variable Names Introduced by Let Expressions
// =======================================================================
//
// Los let expressions introducen variables en un nuevo scope.
// Ejemplos en HULK:
//   let x = 5 in x + 1              # x es resolvible dentro
//   let x = 5 in (let y = x in y)   # y es resolvible en inner scope
//
// La estructura del symbol_table ya soporta esto vía enter_scope/exit_scope.
// Estos tests verifican que la resolución de variables en let expressions
// funciona correctamente.

#[test]
fn test_let_expression_simple_variable_declaration() {
    // Verifica que una variable declarada en un let expression es resolvible
    // Simula: let x = 5 in x + 1
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Simular el análisis semántico de: let x = 5 in x + 1
    // 1. Entrar a scope local para el let
    table.enter_scope();

    // 2. Declarar la variable x del let
    let var_x = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    assert!(table.declare(var_x).is_ok());

    // 3. Dentro del let, x debe ser resolvible
    assert!(table.lookup("x").is_some());
    assert!(table.lookup_local("x").is_some());

    // 4. Salir del let (fin de su scope)
    table.exit_scope();

    // 5. Después del let, x ya no es resolvible
    assert!(table.lookup("x").is_none());
}

#[test]
fn test_let_expression_accesses_outer_scope() {
    // Verifica que dentro de un let, se pueden acceder variables del scope exterior
    // Simula: let y = 10 in (let x = y + 1 in x)
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Global scope: y = 10
    let var_y_global = SymbolInfo::Variable {
        name: "y".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_y_global).unwrap();

    // Outer let scope: x = y + 1
    table.enter_scope();
    let var_x_outer = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_x_outer).unwrap();

    // Dentro del outer let:
    // - x es resolvible (en el mismo scope)
    // - y es resolvible (en global scope)
    assert!(table.lookup("x").is_some());
    assert!(table.lookup("y").is_some());

    // Pero lookup_local no encuentra y (solo en este scope está x)
    assert!(table.lookup_local("x").is_some());
    assert!(table.lookup_local("y").is_none());

    table.exit_scope();

    // Después del let: solo y está resolvible
    assert!(table.lookup("x").is_none());
    assert!(table.lookup("y").is_some());
}

#[test]
fn test_let_expression_nested_shadowing() {
    // Verifica que let expressions anidadas pueden hacer shadowing
    // Simula: let x = 5 in (let x = 10 in x) + x
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Outer let: let x = 5
    table.enter_scope();
    let var_x_outer = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_x_outer).unwrap();

    // lookup("x") encuentra la x outer
    assert!(table.lookup_local("x").is_some());

    // Inner let: let x = 10 (shadow)
    table.enter_scope();
    let var_x_inner = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_x_inner).unwrap();

    // lookup("x") encuentra la x inner (shadowing)
    assert!(table.lookup("x").is_some());
    assert!(table.lookup_local("x").is_some());

    table.exit_scope();

    // Después del inner let, lookup("x") encuentra la x outer nuevamente
    assert!(table.lookup("x").is_some());
    assert!(table.lookup_local("x").is_some());

    table.exit_scope();

    // Después del outer let, x no existe
    assert!(table.lookup("x").is_none());
}

#[test]
fn test_let_expression_deep_nesting() {
    // Verifica que múltiples let expressions anidadas funcionan
    // Simula: let a = 1 in (let b = 2 in (let c = 3 in (a + b + c)))
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // let a = 1
    table.enter_scope();
    let var_a = SymbolInfo::Variable {
        name: "a".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_a).unwrap();
    assert_eq!(table.scope_depth(), 2);

    // let b = 2
    table.enter_scope();
    let var_b = SymbolInfo::Variable {
        name: "b".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_b).unwrap();
    assert_eq!(table.scope_depth(), 3);

    // let c = 3
    table.enter_scope();
    let var_c = SymbolInfo::Variable {
        name: "c".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_c).unwrap();
    assert_eq!(table.scope_depth(), 4);

    // En el innermost let, todas las variables son resolvibles
    assert!(table.lookup("a").is_some());
    assert!(table.lookup("b").is_some());
    assert!(table.lookup("c").is_some());

    // lookup_local solo encuentra c
    assert!(table.lookup_local("a").is_none());
    assert!(table.lookup_local("b").is_none());
    assert!(table.lookup_local("c").is_some());

    // Salir de let c
    table.exit_scope();
    assert_eq!(table.scope_depth(), 3);
    assert!(table.lookup("a").is_some());
    assert!(table.lookup("b").is_some());
    assert!(table.lookup("c").is_none());

    // Salir de let b
    table.exit_scope();
    assert_eq!(table.scope_depth(), 2);
    assert!(table.lookup("a").is_some());
    assert!(table.lookup("b").is_none());
    assert!(table.lookup("c").is_none());

    // Salir de let a
    table.exit_scope();
    assert_eq!(table.scope_depth(), 1);
    assert!(table.lookup("a").is_none());
    assert!(table.lookup("b").is_none());
    assert!(table.lookup("c").is_none());
}

#[test]
fn test_let_expression_multiple_variables_same_scope() {
    // Verifica que en el mismo let scope no se puede declarar dos veces el mismo nombre
    // Simula: let x = 5, let x = 10 in x (error, redeclaración)
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    table.enter_scope();

    let var_x1 = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    assert!(table.declare(var_x1).is_ok());

    // Intenta declarar x otra vez en el mismo scope
    let var_x2 = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    assert!(table.declare(var_x2).is_err());

    // Pero sí se puede declarar una variable diferente
    let var_y = SymbolInfo::Variable {
        name: "y".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    assert!(table.declare(var_y).is_ok());

    table.exit_scope();
}

#[test]
fn test_let_expression_with_different_symbol_types() {
    // Verifica que let expressions pueden contener diferentes tipos de símbolos
    // (variables, parámetros, tipos locales, etc.)
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    table.enter_scope();

    // Variable
    let var = SymbolInfo::Variable {
        name: "my_var".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    assert!(table.declare(var).is_ok());

    // Parámetro (unlikely en un let real, pero válido en la tabla de símbolos)
    let param = SymbolInfo::Parameter {
        name: "my_param".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    assert!(table.declare(param).is_ok());

    // Tipo local (unlikely en un let real, pero válido)
    let type_sym = SymbolInfo::Type {
        name: "LocalType".to_string(),
        span: span.clone(),
    };
    assert!(table.declare(type_sym).is_ok());

    // Todos son resolvibles
    assert!(table.lookup("my_var").is_some());
    assert!(table.lookup("my_param").is_some());
    assert!(table.lookup("LocalType").is_some());

    table.exit_scope();

    // Después de salir del let, ninguno es resolvible
    assert!(table.lookup("my_var").is_none());
    assert!(table.lookup("my_param").is_none());
    assert!(table.lookup("LocalType").is_none());
}

// ===== TAREA 5: FUNCTION PARAMETERS AND LOCAL BINDINGS =====

#[test]
fn test_function_parameter_simple_resolution() {
    // Simula: function foo(x: Number) { x }
    // Parámetro debe ser resolvible dentro del function body
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Paso 1: Declarar función en global scope
    let func_foo = SymbolInfo::Function {
        name: "foo".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    table.declare(func_foo).unwrap();

    // foo es resolvible globalmente
    assert!(table.lookup("foo").is_some());

    // Paso 2: Entrar a scope de la función
    table.enter_scope();

    // Paso 3: Declarar parámetro x
    let param_x = SymbolInfo::Parameter {
        name: "x".to_string(),
        type_ref: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };
    table.declare(param_x).unwrap();

    // Paso 4: x es resolvible dentro de la función
    assert!(table.lookup("x").is_some());
    assert!(table.lookup_local("x").is_some());
    
    // La función también sigue siendo accesible (global scope)
    assert!(table.lookup("foo").is_some());

    // Paso 5: Salir del scope
    table.exit_scope();

    // Paso 6: x no es resolvible fuera de la función
    assert!(table.lookup("x").is_none());
    // Pero la función sigue siendo accesible
    assert!(table.lookup("foo").is_some());
}

#[test]
fn test_function_multiple_parameters() {
    // Simula: function add(x: Number, y: Number) { x + y }
    // Todos los parámetros deben ser resolvibles
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Entrar a scope de la función
    table.enter_scope();

    // Declarar múltiples parámetros
    let param_x = SymbolInfo::Parameter {
        name: "x".to_string(),
        type_ref: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };
    let param_y = SymbolInfo::Parameter {
        name: "y".to_string(),
        type_ref: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };
    let param_z = SymbolInfo::Parameter {
        name: "z".to_string(),
        type_ref: Some(TypeReference::new("String".to_string(), span.clone())),
        span: span.clone(),
    };

    table.declare(param_x).unwrap();
    table.declare(param_y).unwrap();
    table.declare(param_z).unwrap();

    // Todos los parámetros son resolvibles
    assert!(table.lookup("x").is_some());
    assert!(table.lookup("y").is_some());
    assert!(table.lookup("z").is_some());

    // Todos están en el scope local actual
    assert!(table.lookup_local("x").is_some());
    assert!(table.lookup_local("y").is_some());
    assert!(table.lookup_local("z").is_some());

    // Tratamos de declarar un parámetro duplicado - error
    let param_x_dup = SymbolInfo::Parameter {
        name: "x".to_string(),
        type_ref: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };
    assert!(table.declare(param_x_dup).is_err());

    table.exit_scope();

    // Ninguno es resolvible fuera de la función
    assert!(table.lookup("x").is_none());
    assert!(table.lookup("y").is_none());
    assert!(table.lookup("z").is_none());
}

#[test]
fn test_function_parameter_shadowing_with_local_variable() {
    // Simula: function foo(x: Number) { let x = "hello" in x }
    // Variable local puede sombrear parámetro
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Entrar a scope de la función
    table.enter_scope();

    // Declarar parámetro x (Number)
    let param_x = SymbolInfo::Parameter {
        name: "x".to_string(),
        type_ref: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };
    table.declare(param_x).unwrap();

    // x es parámetro
    assert!(table.lookup("x").is_some());
    assert!(table.lookup_local("x").is_some());

    // Entrar a scope de let expression
    table.enter_scope();

    // Declarar variable x (String) que sombrea el parámetro
    let var_x = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: Some(TypeReference::new("String".to_string(), span.clone())),
        span: span.clone(),
    };
    table.declare(var_x).unwrap();

    // lookup("x") encuentra la variable (la más cercana)
    assert!(table.lookup("x").is_some());
    // lookup_local("x") también encuentra la variable
    assert!(table.lookup_local("x").is_some());

    table.exit_scope();

    // Después del let, lookup("x") nuevamente encuentra el parámetro
    assert!(table.lookup("x").is_some());
    assert!(table.lookup_local("x").is_some());

    table.exit_scope();

    // Fuera de la función, x no existe
    assert!(table.lookup("x").is_none());
}

#[test]
fn test_function_parameter_not_accessible_outside_scope() {
    // Verifica que los parámetros están completamente aislados a la función
    // Simula: function foo(x: Number) { x }
    //         foo  <- x no debería estar accesible aquí
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Declarar función en global
    let func = SymbolInfo::Function {
        name: "foo".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    table.declare(func).unwrap();

    // Entrar a scope de la función y declarar parámetro
    table.enter_scope();
    let param_x = SymbolInfo::Parameter {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(param_x).unwrap();

    // Dentro de la función, x está disponible
    assert!(table.lookup("x").is_some());

    table.exit_scope();

    // Después de salir, x no está disponible
    assert!(table.lookup("x").is_none());
    // Pero la función sí
    assert!(table.lookup("foo").is_some());
}

#[test]
fn test_nested_function_parameter_isolation() {
    // Simula:
    // function outer(a: Number) {
    //   function inner(b: Number) { b }
    //   a
    // }
    // Los parámetros de inner no están visibles en outer
    // Las funciones declaradas localmente (inner) solo son visibles en su scope
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Declarar outer en global
    let outer_func = SymbolInfo::Function {
        name: "outer".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    table.declare(outer_func).unwrap();

    // Entrar a scope de outer
    table.enter_scope();

    // Declarar parámetro a de outer
    let param_a = SymbolInfo::Parameter {
        name: "a".to_string(),
        type_ref: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };
    table.declare(param_a).unwrap();

    // a es resolvible
    assert!(table.lookup("a").is_some());

    // Declarar inner dentro de outer (es un símbolo local de outer)
    let inner_func = SymbolInfo::Function {
        name: "inner".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    table.declare(inner_func).unwrap();

    // Entrar a scope de inner
    table.enter_scope();

    // Declarar parámetro b de inner
    let param_b = SymbolInfo::Parameter {
        name: "b".to_string(),
        type_ref: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };
    table.declare(param_b).unwrap();

    // Dentro de inner:
    // b es resolvible (local)
    assert!(table.lookup("b").is_some());
    // a es resolvible (del parent scope de outer)
    assert!(table.lookup("a").is_some());
    // inner es resolvible (local a outer)
    assert!(table.lookup("inner").is_some());
    // outer es resolvible (global)
    assert!(table.lookup("outer").is_some());

    table.exit_scope();  // Salir de inner

    // Dentro de outer (después de inner):
    // b no es resolvible (era parámetro de inner)
    assert!(table.lookup("b").is_none());
    // a es resolvible (local a outer)
    assert!(table.lookup("a").is_some());
    // inner sigue siendo resolvible (local a outer)
    assert!(table.lookup("inner").is_some());
    // outer es resolvible (global)
    assert!(table.lookup("outer").is_some());

    table.exit_scope();  // Salir de outer

    // En global:
    // Ni a ni b están disponibles (eran locales a outer)
    assert!(table.lookup("a").is_none());
    assert!(table.lookup("b").is_none());
    // inner no es resolvible (era local a outer)
    assert!(table.lookup("inner").is_none());
    // Pero outer sí es resolvible (está en global)
    assert!(table.lookup("outer").is_some());
}

// ===== TAREA 6: SUPPORT RESOLUTION OF TOP-LEVEL DECLARATIONS IN GLOBAL SCOPE =====

#[test]
fn test_top_level_function_resolution_from_global() {
    // Verifica que funciones declaradas en global scope son resolvibles desde global
    // Simula: function add(x: Number, y: Number) -> Number { x + y }
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Declarar función en global scope
    let add_func = SymbolInfo::Function {
        name: "add".to_string(),
        parameters: vec![],
        return_type: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };
    table.declare(add_func).unwrap();

    // Verificar que está en global
    assert!(table.is_global_scope());
    assert!(table.lookup("add").is_some());
    assert!(table.lookup_local("add").is_some());

    // Verificar que está en global_symbols()
    let global_syms = table.global_symbols();
    assert_eq!(global_syms.len(), 1);
    assert_eq!(global_syms[0].name(), "add");
}

#[test]
fn test_top_level_function_accessible_from_nested_scope() {
    // Verifica que funciones globales son accesibles desde scopes anidados
    // Simula: 
    // function add(x: Number, y: Number) -> Number { x + y }
    // function multiply(a: Number, b: Number) -> Number { 
    //   let result = add(a, b) in result  <- add es accesible aquí
    // }
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Declarar función top-level
    let add_func = SymbolInfo::Function {
        name: "add".to_string(),
        parameters: vec![],
        return_type: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };
    table.declare(add_func).unwrap();

    // Entrar a scope de otra función
    table.enter_scope();
    
    // Desde aquí, add es resolvible (atraviesa scopes)
    assert!(table.lookup("add").is_some());
    
    // Entrar a un let dentro
    table.enter_scope();
    assert!(table.lookup("add").is_some());
    
    // Entrar a otro nivel
    table.enter_scope();
    assert!(table.lookup("add").is_some());
    
    table.exit_scope();
    table.exit_scope();
    table.exit_scope();

    // En global, sigue siendo resolvible
    assert!(table.lookup("add").is_some());
}

#[test]
fn test_top_level_type_declaration_and_resolution() {
    // Verifica que tipos declarados en global son resolvibles desde cualquier lugar
    // Simula: type Point { x: Number, y: Number }
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Declarar tipo top-level
    let point_type = SymbolInfo::Type {
        name: "Point".to_string(),
        span: span.clone(),
    };
    table.declare(point_type).unwrap();

    // Resolvible en global
    assert!(table.lookup("Point").is_some());

    // Entrar a scope local
    table.enter_scope();
    assert!(table.lookup("Point").is_some());
    
    table.exit_scope();

    // Sigue siendo resolvible
    assert!(table.lookup("Point").is_some());
}

#[test]
fn test_top_level_protocol_declaration_and_resolution() {
    // Verifica que protocolos declarados en global son resolvibles
    // Simula: protocol Printable { print(obj: Printable) }
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Declarar protocolo top-level
    let printable_proto = SymbolInfo::Protocol {
        name: "Printable".to_string(),
        span: span.clone(),
    };
    table.declare(printable_proto).unwrap();

    // Resolvible en global
    assert!(table.lookup("Printable").is_some());

    // Entrar a scope local
    table.enter_scope();
    assert!(table.lookup("Printable").is_some());
    
    // En local también
    let var = SymbolInfo::Variable {
        name: "obj".to_string(),
        type_ref: Some(TypeReference::new("Printable".to_string(), span.clone())),
        span: span.clone(),
    };
    table.declare(var).unwrap();
    assert!(table.lookup("Printable").is_some());
    
    table.exit_scope();

    // Sigue siendo resolvible
    assert!(table.lookup("Printable").is_some());
}

#[test]
fn test_multiple_top_level_declarations_coexist() {
    // Verifica que múltiples declaraciones top-level coexisten sin conflicto
    // Simula:
    // function add(x: Number, y: Number) -> Number { x + y }
    // function subtract(x: Number, y: Number) -> Number { x - y }
    // type Point { x: Number, y: Number }
    // protocol Shape { area() -> Number }
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Declarar múltiples funciones
    let add_func = SymbolInfo::Function {
        name: "add".to_string(),
        parameters: vec![],
        return_type: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };
    table.declare(add_func).unwrap();

    let subtract_func = SymbolInfo::Function {
        name: "subtract".to_string(),
        parameters: vec![],
        return_type: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };
    table.declare(subtract_func).unwrap();

    // Declarar tipo
    let point_type = SymbolInfo::Type {
        name: "Point".to_string(),
        span: span.clone(),
    };
    table.declare(point_type).unwrap();

    // Declarar protocolo
    let shape_proto = SymbolInfo::Protocol {
        name: "Shape".to_string(),
        span: span.clone(),
    };
    table.declare(shape_proto).unwrap();

    // Todos son resolvibles
    assert!(table.lookup("add").is_some());
    assert!(table.lookup("subtract").is_some());
    assert!(table.lookup("Point").is_some());
    assert!(table.lookup("Shape").is_some());

    // global_symbols() contiene todos
    let global_syms = table.global_symbols();
    assert_eq!(global_syms.len(), 4);

    // Dentro de un scope local, todos siguen siendo accesibles
    table.enter_scope();
    assert!(table.lookup("add").is_some());
    assert!(table.lookup("subtract").is_some());
    assert!(table.lookup("Point").is_some());
    assert!(table.lookup("Shape").is_some());
    table.exit_scope();
}

#[test]
fn test_top_level_duplicate_declaration_error() {
    // Verifica que no pueda haber duplicados en global scope
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Declarar función
    let func1 = SymbolInfo::Function {
        name: "myFunc".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    assert!(table.declare(func1).is_ok());

    // Intentar declarar función duplicada en global
    let func2 = SymbolInfo::Function {
        name: "myFunc".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    assert!(table.declare(func2).is_err());

    // Pero en un scope local sí puedo redeclarar (shadowing)
    table.enter_scope();
    let func3 = SymbolInfo::Function {
        name: "myFunc".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    assert!(table.declare(func3).is_ok());
    table.exit_scope();
}

#[test]
fn test_top_level_shadowing_not_allowed_same_scope() {
    // Verifica que el mismo tipo de símbolo no puede ser declarado dos veces
    // incluso en global, pero diferentes tipos SÍ pueden coexistir con el mismo nombre
    // (aunque sea mala práctica)
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Declarar tipo "Point"
    let point_type = SymbolInfo::Type {
        name: "Point".to_string(),
        span: span.clone(),
    };
    table.declare(point_type).unwrap();

    // Intentar declarar función con el mismo nombre - error
    let point_func = SymbolInfo::Function {
        name: "Point".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    assert!(table.declare(point_func).is_err());

    // lookup("Point") encuentra el tipo
    assert!(table.lookup("Point").is_some());
}

#[test]
fn test_top_level_all_symbols_including_from_parent_scopes() {
    // Verifica que all_symbols_in_chain() incluye símbolos globales
    // incluso desde scopes anidados
    
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Declarar función en global
    let global_func = SymbolInfo::Function {
        name: "globalFunc".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    table.declare(global_func).unwrap();

    // Entrar a scope local
    table.enter_scope();

    // Declarar variable local
    let local_var = SymbolInfo::Variable {
        name: "localVar".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(local_var).unwrap();

    // all_symbols_in_chain() debe incluir ambos
    let all_syms = table.all_symbols_in_chain();
    assert_eq!(all_syms.len(), 2);

    // Verificar que incluye tanto global como local
    let names: Vec<&str> = all_syms.iter().map(|(_, n, _)| n.as_str()).collect();
    assert!(names.contains(&"globalFunc"));
    assert!(names.contains(&"localVar"));

    table.exit_scope();

    // En global, solo la función es visible
    let global_syms = table.global_symbols();
    assert_eq!(global_syms.len(), 1);
    assert_eq!(global_syms[0].name(), "globalFunc");
}

// ===== TAREA 7: SEMANTIC ERRORS FOR UNDEFINED SYMBOLS AND INVALID REDEFINITIONS =====

#[test]
fn test_semantic_error_function_already_declared() {
    // Verifica que declare() retorna SemanticError::FunctionAlreadyDeclared
    let mut table = SymbolTable::new();
    let span = Span::default();

    let func1 = SymbolInfo::Function {
        name: "add".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };

    assert!(table.declare(func1).is_ok());

    let func2 = SymbolInfo::Function {
        name: "add".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };

    // Intentar declarar función duplicada
    let err = table.declare(func2);
    assert!(err.is_err());

    // Verificar que es el error correcto
    match err.unwrap_err() {
        SemanticError::FunctionAlreadyDeclared { name, .. } => {
            assert_eq!(name, "add");
        }
        _ => panic!("Expected FunctionAlreadyDeclared error"),
    }
}

#[test]
fn test_semantic_error_variable_already_declared() {
    // Verifica que declare() retorna SemanticError::VariableAlreadyDeclared
    let mut table = SymbolTable::new();
    let span = Span::default();

    let var1 = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };

    assert!(table.declare(var1).is_ok());

    let var2 = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };

    let err = table.declare(var2);
    assert!(err.is_err());

    match err.unwrap_err() {
        SemanticError::VariableAlreadyDeclared { name, first_line, first_column } => {
            assert_eq!(name, "x");
            assert_eq!(first_line, span.start_line);
            assert_eq!(first_column, span.start_column);
        }
        _ => panic!("Expected VariableAlreadyDeclared error"),
    }
}

#[test]
fn test_semantic_error_type_already_declared() {
    // Verifica que declare() retorna SemanticError::TypeAlreadyDeclared
    let mut table = SymbolTable::new();
    let span = Span::default();

    let type1 = SymbolInfo::Type {
        name: "Point".to_string(),
        span: span.clone(),
    };

    assert!(table.declare(type1).is_ok());

    let type2 = SymbolInfo::Type {
        name: "Point".to_string(),
        span: span.clone(),
    };

    let err = table.declare(type2);
    assert!(err.is_err());

    match err.unwrap_err() {
        SemanticError::TypeAlreadyDeclared { name, .. } => {
            assert_eq!(name, "Point");
        }
        _ => panic!("Expected TypeAlreadyDeclared error"),
    }
}

#[test]
fn test_semantic_error_parameter_already_declared() {
    // Verifica que parámetros duplicados retornan VariableAlreadyDeclared
    let mut table = SymbolTable::new();
    let span = Span::default();

    table.enter_scope();  // Simular scope de función

    let param1 = SymbolInfo::Parameter {
        name: "x".to_string(),
        type_ref: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };

    assert!(table.declare(param1).is_ok());

    let param2 = SymbolInfo::Parameter {
        name: "x".to_string(),
        type_ref: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };

    let err = table.declare(param2);
    assert!(err.is_err());

    match err.unwrap_err() {
        SemanticError::VariableAlreadyDeclared { name, .. } => {
            assert_eq!(name, "x");
        }
        _ => panic!("Expected VariableAlreadyDeclared error for parameter"),
    }

    table.exit_scope();
}

#[test]
fn test_semantic_error_undeclared_variable_lookup() {
    // Verifica que lookup de variable inexistente retorna None
    // y que get_symbol_or_error() retorna UndeclaredVariable
    let table = SymbolTable::new();

    // lookup retorna None
    assert!(table.lookup("undefined_var").is_none());

    // get_symbol_or_error retorna UndeclaredVariable
    let err = table.get_symbol_or_error("undefined_var");
    assert!(err.is_err());

    match err.unwrap_err() {
        SemanticError::UndeclaredVariable { name } => {
            assert_eq!(name, "undefined_var");
        }
        _ => panic!("Expected UndeclaredVariable error"),
    }
}

#[test]
fn test_semantic_error_undeclared_type_lookup() {
    // Verifica que lookup de tipo inexistente retorna None
    // y que get_symbol_or_error() retorna UndeclaredType para nombres con mayúscula
    let table = SymbolTable::new();

    // lookup retorna None
    assert!(table.lookup("UndefinedType").is_none());

    // get_symbol_or_error retorna UndeclaredType (heurística: mayúscula inicial)
    let err = table.get_symbol_or_error("UndefinedType");
    assert!(err.is_err());

    match err.unwrap_err() {
        SemanticError::UndeclaredType { name } => {
            assert_eq!(name, "UndefinedType");
        }
        _ => panic!("Expected UndeclaredType error"),
    }
}

#[test]
fn test_semantic_error_redeclaration_in_different_scope_allowed() {
    // Verifica que la redeclaración en DIFERENTE scope es permitida (shadowing)
    // pero en el MISMO scope no
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Declarar variable en global
    let var1 = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    assert!(table.declare(var1).is_ok());

    // Intentar redeclarar en mismo scope - ERROR
    let var2 = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    assert!(table.declare(var2).is_err());

    // Entrar a scope local
    table.enter_scope();

    // Redeclarar en scope diferente - OK (shadowing)
    let var3 = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    assert!(table.declare(var3).is_ok());

    // lookup encuentra la variable del scope actual (shadowing)
    let sym = table.lookup("x").unwrap();
    assert_eq!(sym.name(), "x");

    table.exit_scope();

    // Fuera del scope, lookup encuentra la variable global
    let sym = table.lookup("x").unwrap();
    assert_eq!(sym.name(), "x");
}

#[test]
fn test_semantic_error_mixed_symbol_types_same_name() {
    // Verifica que no se pueden declarar símbolos de diferente tipo con el mismo nombre
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Declarar función "calculate"
    let func = SymbolInfo::Function {
        name: "calculate".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    assert!(table.declare(func).is_ok());

    // Intentar declarar variable "calculate" - ERROR
    let var = SymbolInfo::Variable {
        name: "calculate".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    assert!(table.declare(var).is_err());

    // Intentar declarar tipo "calculate" - ERROR
    let typ = SymbolInfo::Type {
        name: "calculate".to_string(),
        span: span.clone(),
    };
    assert!(table.declare(typ).is_err());
}

#[test]
fn test_semantic_error_tracks_first_declaration_location() {
    // Verifica que el error reporta la ubicación de la primera declaración
    let mut table = SymbolTable::new();
    let span1 = Span {
        start_line: 10,
        start_column: 5,
        ..Default::default()
    };
    let span2 = Span {
        start_line: 15,
        start_column: 8,
        ..Default::default()
    };

    let func1 = SymbolInfo::Function {
        name: "myFunc".to_string(),
        parameters: vec![],
        return_type: None,
        span: span1.clone(),
    };
    table.declare(func1).unwrap();

    let func2 = SymbolInfo::Function {
        name: "myFunc".to_string(),
        parameters: vec![],
        return_type: None,
        span: span2.clone(),
    };

    let err = table.declare(func2);
    match err.unwrap_err() {
        SemanticError::FunctionAlreadyDeclared {
            name,
            first_line,
            first_column,
        } => {
            assert_eq!(name, "myFunc");
            assert_eq!(first_line, 10);  // Ubicación de PRIMERA declaración
            assert_eq!(first_column, 5);
        }
        _ => panic!("Expected FunctionAlreadyDeclared error"),
    }
}

#[test]
fn test_semantic_error_heuristic_lowercase_vs_uppercase() {
    // Verifica la heurística de get_symbol_or_error() para distinguir tipos de símbolos
    let table = SymbolTable::new();

    // Nombre con minúscula -> UndeclaredVariable
    let err1 = table.get_symbol_or_error("myVariable");
    match err1.unwrap_err() {
        SemanticError::UndeclaredVariable { name } => {
            assert_eq!(name, "myVariable");
        }
        _ => panic!("Expected UndeclaredVariable for lowercase name"),
    }

    // Nombre con mayúscula -> UndeclaredType
    let err2 = table.get_symbol_or_error("MyType");
    match err2.unwrap_err() {
        SemanticError::UndeclaredType { name } => {
            assert_eq!(name, "MyType");
        }
        _ => panic!("Expected UndeclaredType for uppercase name"),
    }

    // Número al inicio -> UndeclaredVariable (no es mayúscula)
    let err3 = table.get_symbol_or_error("123invalid");
    match err3.unwrap_err() {
        SemanticError::UndeclaredVariable { name } => {
            assert_eq!(name, "123invalid");
        }
        _ => panic!("Expected UndeclaredVariable for digit-start name"),
    }
}

// ===== TAREA 8: EXPOSE SAFE API FOR OTHER MODULES =====

#[test]
fn test_safe_api_is_function() {
    // Verifica que is_function() retorna true solo para funciones
    let mut table = SymbolTable::new();
    let span = Span::default();

    let func = SymbolInfo::Function {
        name: "add".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    table.declare(func).unwrap();

    let var = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var).unwrap();

    // is_function() retorna true solo para funciones
    assert!(table.is_function("add"));
    assert!(!table.is_function("x"));
    assert!(!table.is_function("undefined"));
}

#[test]
fn test_safe_api_is_variable() {
    // Verifica que is_variable() retorna true solo para variables
    let mut table = SymbolTable::new();
    let span = Span::default();

    let func = SymbolInfo::Function {
        name: "foo".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    table.declare(func).unwrap();

    let var = SymbolInfo::Variable {
        name: "count".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var).unwrap();

    assert!(!table.is_variable("foo"));
    assert!(table.is_variable("count"));
    assert!(!table.is_variable("undefined"));
}

#[test]
fn test_safe_api_is_parameter() {
    // Verifica que is_parameter() retorna true solo para parámetros
    let mut table = SymbolTable::new();
    let span = Span::default();

    table.enter_scope();

    let param = SymbolInfo::Parameter {
        name: "x".to_string(),
        type_ref: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };
    table.declare(param).unwrap();

    let var = SymbolInfo::Variable {
        name: "y".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var).unwrap();

    assert!(table.is_parameter("x"));
    assert!(!table.is_parameter("y"));
    assert!(!table.is_parameter("undefined"));

    table.exit_scope();
}

#[test]
fn test_safe_api_is_type() {
    // Verifica que is_type() retorna true solo para tipos
    let mut table = SymbolTable::new();
    let span = Span::default();

    let typ = SymbolInfo::Type {
        name: "Point".to_string(),
        span: span.clone(),
    };
    table.declare(typ).unwrap();

    let var = SymbolInfo::Variable {
        name: "point".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var).unwrap();

    assert!(table.is_type("Point"));
    assert!(!table.is_type("point"));
    assert!(!table.is_type("undefined"));
}

#[test]
fn test_safe_api_is_protocol() {
    // Verifica que is_protocol() retorna true solo para protocolos
    let mut table = SymbolTable::new();
    let span = Span::default();

    let proto = SymbolInfo::Protocol {
        name: "Drawable".to_string(),
        span: span.clone(),
    };
    table.declare(proto).unwrap();

    let typ = SymbolInfo::Type {
        name: "Circle".to_string(),
        span: span.clone(),
    };
    table.declare(typ).unwrap();

    assert!(table.is_protocol("Drawable"));
    assert!(!table.is_protocol("Circle"));
    assert!(!table.is_protocol("undefined"));
}

#[test]
fn test_safe_api_get_symbol_kind() {
    // Verifica que get_symbol_kind() retorna el tipo correcto
    let mut table = SymbolTable::new();
    let span = Span::default();

    let func = SymbolInfo::Function {
        name: "calculate".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    table.declare(func).unwrap();

    let var = SymbolInfo::Variable {
        name: "result".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var).unwrap();

    let typ = SymbolInfo::Type {
        name: "Value".to_string(),
        span: span.clone(),
    };
    table.declare(typ).unwrap();

    assert_eq!(table.get_symbol_kind("calculate"), Some("function"));
    assert_eq!(table.get_symbol_kind("result"), Some("variable"));
    assert_eq!(table.get_symbol_kind("Value"), Some("type"));
    assert_eq!(table.get_symbol_kind("undefined"), None);
}

#[test]
fn test_safe_api_functions_in_scope() {
    // Verifica que functions_in_scope() retorna solo funciones del scope actual
    let mut table = SymbolTable::new();
    let span = Span::default();

    let func1 = SymbolInfo::Function {
        name: "add".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    let func2 = SymbolInfo::Function {
        name: "multiply".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    let var = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };

    table.declare(func1).unwrap();
    table.declare(func2).unwrap();
    table.declare(var).unwrap();

    let funcs = table.functions_in_scope();
    assert_eq!(funcs.len(), 2);
    let names: Vec<&str> = funcs.iter().map(|f| f.name()).collect();
    assert!(names.contains(&"add"));
    assert!(names.contains(&"multiply"));
}

#[test]
fn test_safe_api_global_functions() {
    // Verifica que global_functions() retorna funciones globales
    let mut table = SymbolTable::new();
    let span = Span::default();

    let func1 = SymbolInfo::Function {
        name: "globalFunc1".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    let func2 = SymbolInfo::Function {
        name: "globalFunc2".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };

    table.declare(func1).unwrap();
    table.declare(func2).unwrap();

    // Entrar a scope local y declarar función local
    table.enter_scope();
    let local_func = SymbolInfo::Function {
        name: "localFunc".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    table.declare(local_func).unwrap();
    table.exit_scope();

    // global_functions() solo retorna funciones globales
    let global_funcs = table.global_functions();
    assert_eq!(global_funcs.len(), 2);
    let names: Vec<&str> = global_funcs.iter().map(|f| f.name()).collect();
    assert!(names.contains(&"globalFunc1"));
    assert!(names.contains(&"globalFunc2"));
    assert!(!names.contains(&"localFunc"));
}

#[test]
fn test_safe_api_symbol_count_in_scope() {
    // Verifica que symbol_count_in_scope() cuenta correctamente
    let mut table = SymbolTable::new();
    let span = Span::default();

    assert_eq!(table.symbol_count_in_scope(), 0);

    let var1 = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var1).unwrap();
    assert_eq!(table.symbol_count_in_scope(), 1);

    let var2 = SymbolInfo::Variable {
        name: "y".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var2).unwrap();
    assert_eq!(table.symbol_count_in_scope(), 2);

    table.enter_scope();
    assert_eq!(table.symbol_count_in_scope(), 0);  // Nuevo scope está vacío

    let var3 = SymbolInfo::Variable {
        name: "z".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var3).unwrap();
    assert_eq!(table.symbol_count_in_scope(), 1);

    table.exit_scope();
    assert_eq!(table.symbol_count_in_scope(), 2);  // De vuelta a 2 en scope global
}

#[test]
fn test_safe_api_total_symbol_count() {
    // Verifica que total_symbol_count() cuenta en todos los scopes
    let mut table = SymbolTable::new();
    let span = Span::default();

    let var1 = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var1).unwrap();
    assert_eq!(table.total_symbol_count(), 1);

    table.enter_scope();
    let var2 = SymbolInfo::Variable {
        name: "y".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var2).unwrap();
    assert_eq!(table.total_symbol_count(), 2);  // x + y

    table.enter_scope();
    let var3 = SymbolInfo::Variable {
        name: "z".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var3).unwrap();
    assert_eq!(table.total_symbol_count(), 3);  // x + y + z

    table.exit_scope();
    table.exit_scope();
}

#[test]
fn test_safe_api_resolve_symbol_info() {
    // Verifica que resolve_symbol_info() retorna info completa
    let mut table = SymbolTable::new();
    let span = Span::default();

    let func = SymbolInfo::Function {
        name: "myFunc".to_string(),
        parameters: vec![],
        return_type: None,
        span: span.clone(),
    };
    table.declare(func).unwrap();

    let info = table.resolve_symbol_info("myFunc");
    assert!(info.is_some());

    let (sym, scope_idx, kind) = info.unwrap();
    assert_eq!(sym.name(), "myFunc");
    assert_eq!(scope_idx, 0);  // Global scope (índice 0)
    assert_eq!(kind, "function");
}

#[test]
fn test_safe_api_symbol_availability() {
    // Verifica que symbol_availability() retorna la información correcta
    let mut table = SymbolTable::new();
    let span = Span::default();

    // Variable en global
    let var_global = SymbolInfo::Variable {
        name: "globalVar".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_global).unwrap();

    // Cuando estamos en global, globalVar es local al scope actual (que es global)
    let (exists, is_local, is_global) = table.symbol_availability("globalVar");
    assert!(exists);
    assert!(is_local);     // Es local porque estamos en el scope global
    assert!(is_global);

    // Variable en scope local
    table.enter_scope();
    let var_local = SymbolInfo::Variable {
        name: "localVar".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(var_local).unwrap();

    let (exists, is_local, is_global) = table.symbol_availability("localVar");
    assert!(exists);
    assert!(is_local);     // En scope actual
    assert!(!is_global);

    // Verificar global variable desde local scope
    let (exists, is_local, is_global) = table.symbol_availability("globalVar");
    assert!(exists);       // Sigue siendo accesible
    assert!(!is_local);    // No es local (está en scope anterior)
    assert!(is_global);    // Es global

    table.exit_scope();
}

#[test]
fn test_safe_api_get_type_reference() {
    // Verifica que get_type_reference() retorna el tipo correcto
    let mut table = SymbolTable::new();
    let span = Span::default();

    let typed_var = SymbolInfo::Variable {
        name: "age".to_string(),
        type_ref: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };
    table.declare(typed_var).unwrap();

    let untyped_var = SymbolInfo::Variable {
        name: "value".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(untyped_var).unwrap();

    let type_ref = table.get_type_reference("age");
    assert!(type_ref.is_some());

    let type_ref = table.get_type_reference("value");
    assert!(type_ref.is_none());

    let type_ref = table.get_type_reference("undefined");
    assert!(type_ref.is_none());
}

#[test]
fn test_safe_api_exists_in_global() {
    // Verifica que exists_in_global() solo retorna true para símbolos globales
    let mut table = SymbolTable::new();
    let span = Span::default();

    let global_var = SymbolInfo::Variable {
        name: "globalVar".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(global_var).unwrap();

    assert!(table.exists_in_global("globalVar"));
    assert!(!table.exists_in_global("undefined"));

    table.enter_scope();
    let local_var = SymbolInfo::Variable {
        name: "localVar".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(local_var).unwrap();

    assert!(!table.exists_in_global("localVar"));  // Existe pero no globalmente
    assert!(table.exists_in_global("globalVar"));  // Sigue existiendo globalmente

    table.exit_scope();
}

#[test]
fn test_safe_api_exists_only_locally() {
    // Verifica que exists_only_locally() retorna true solo para símbolos locales
    let mut table = SymbolTable::new();
    let span = Span::default();

    let global_var = SymbolInfo::Variable {
        name: "globalVar".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(global_var).unwrap();

    assert!(!table.exists_only_locally("globalVar"));  // Es global, no solo local

    table.enter_scope();

    let local_var = SymbolInfo::Variable {
        name: "localVar".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    table.declare(local_var).unwrap();

    assert!(table.exists_only_locally("localVar"));     // Es solo local
    assert!(!table.exists_only_locally("globalVar"));   // Es global, no solo local

    table.exit_scope();

    assert!(!table.exists_only_locally("localVar"));    // Ya no existe
    assert!(!table.exists_only_locally("globalVar"));   // Es global
}

#[test]
fn test_safe_api_global_types_and_protocols() {
    // Verifica que global_types() y global_protocols() funcionan correctamente
    let mut table = SymbolTable::new();
    let span = Span::default();

    let type1 = SymbolInfo::Type {
        name: "Point".to_string(),
        span: span.clone(),
    };
    let type2 = SymbolInfo::Type {
        name: "Vector".to_string(),
        span: span.clone(),
    };
    let proto = SymbolInfo::Protocol {
        name: "Drawable".to_string(),
        span: span.clone(),
    };

    table.declare(type1).unwrap();
    table.declare(type2).unwrap();
    table.declare(proto).unwrap();

    let types = table.global_types();
    assert_eq!(types.len(), 2);
    let type_names: Vec<&str> = types.iter().map(|t| t.name()).collect();
    assert!(type_names.contains(&"Point"));
    assert!(type_names.contains(&"Vector"));

    let protos = table.global_protocols();
    assert_eq!(protos.len(), 1);
    assert_eq!(protos[0].name(), "Drawable");
}

#[test]
fn test_safe_api_variables_and_parameters_in_scope() {
    // Verifica que variables_in_scope() y parameters_in_scope() funcionan
    let mut table = SymbolTable::new();
    let span = Span::default();

    table.enter_scope();

    let var = SymbolInfo::Variable {
        name: "x".to_string(),
        type_ref: None,
        span: span.clone(),
    };
    let param = SymbolInfo::Parameter {
        name: "y".to_string(),
        type_ref: Some(TypeReference::new("Number".to_string(), span.clone())),
        span: span.clone(),
    };

    table.declare(var).unwrap();
    table.declare(param).unwrap();

    let vars = table.variables_in_scope();
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].name(), "x");

    let params = table.parameters_in_scope();
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].name(), "y");

    table.exit_scope();
}
