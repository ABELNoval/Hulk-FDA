// =============================================================================
// Tests for Symbol Table & Scope Management
// =============================================================================

use super::symbol_table::*;
use crate::utils::errors::span::Span;

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
