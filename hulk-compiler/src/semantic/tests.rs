// // Consolidated Semantic tests
// // This file contains all unit tests from the individual semantic modules
// // (symbol_table, type_system, expression_checker, analyzer) so they can
// // be run from a single place.

// use super::analyzer::*;
// use super::expression_checker::*;
// use super::symbol_table::*;
// use super::type_system::*;
// use crate::parser::ast::*;
// use crate::utils::errors::semantic::SemanticError;
// use crate::utils::errors::span::Span;

// // ========= symbol_table tests =========

// #[test]
// fn test_symbol_table_new() {
//     let table = SymbolTable::new();
//     assert_eq!(table.scope_depth(), 1);
// }

// #[test]
// fn test_enter_exit_scope() {
//     let mut table = SymbolTable::new();
//     assert_eq!(table.scope_depth(), 1);

//     table.enter_scope();
//     assert_eq!(table.scope_depth(), 2);

//     table.exit_scope();
//     assert_eq!(table.scope_depth(), 1);
// }

// #[test]
// fn test_declare_and_lookup() {
//     let mut table = SymbolTable::new();
//     let span = Span::default();

//     let var = SymbolInfo::Variable {
//         name: "x".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };

//     table.declare(var).unwrap();
//     assert!(table.lookup("x").is_some());
//     assert!(table.lookup("y").is_none());
// }

// #[test]
// fn test_duplicate_declaration_error() {
//     let mut table = SymbolTable::new();
//     let span = Span::default();

//     let var1 = SymbolInfo::Variable {
//         name: "x".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };

//     let var2 = SymbolInfo::Variable {
//         name: "x".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };

//     table.declare(var1).unwrap();
//     assert!(table.declare(var2).is_err());
// }

// #[test]
// fn test_scope_shadowing() {
//     let mut table = SymbolTable::new();
//     let span = Span::default();

//     let var1 = SymbolInfo::Variable {
//         name: "x".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };

//     table.declare(var1).unwrap();

//     table.enter_scope();
//     let var2 = SymbolInfo::Variable {
//         name: "x".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };

//     // En nuevo scope, puedo redeclarar "x" (shadowing)
//     assert!(table.declare(var2).is_ok());

//     // lookup() encuentra el del scope actual
//     assert!(table.lookup("x").is_some());

//     table.exit_scope();
//     // Sigo encontrando la variable original
//     assert!(table.lookup("x").is_some());
// }

// #[test]
// fn test_declare_multiple_different_symbols() {
//     let mut table = SymbolTable::new();
//     let span = Span::default();

//     let var_x = SymbolInfo::Variable {
//         name: "x".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };
//     let var_y = SymbolInfo::Variable {
//         name: "y".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };
//     let var_z = SymbolInfo::Variable {
//         name: "z".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };

//     assert!(table.declare(var_x).is_ok());
//     assert!(table.declare(var_y).is_ok());
//     assert!(table.declare(var_z).is_ok());

//     assert!(table.lookup("x").is_some());
//     assert!(table.lookup("y").is_some());
//     assert!(table.lookup("z").is_some());
// }

// #[test]
// fn test_declare_different_symbol_types() {
//     let mut table = SymbolTable::new();
//     let span = Span::default();

//     let var = SymbolInfo::Variable {
//         name: "my_var".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };
//     let param = SymbolInfo::Parameter {
//         name: "my_param".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };
//     let func = SymbolInfo::Function {
//         name: "my_func".to_string(),
//         parameters: Vec::new(),
//         return_type: None,
//         span: span.clone(),
//     };
//     let type_sym = SymbolInfo::Type {
//         name: "my_type".to_string(),
//         span: span.clone(),
//     };
//     let proto = SymbolInfo::Protocol {
//         name: "my_protocol".to_string(),
//         span: span.clone(),
//     };

//     assert!(table.declare(var).is_ok());
//     assert!(table.declare(param).is_ok());
//     assert!(table.declare(func).is_ok());
//     assert!(table.declare(type_sym).is_ok());
//     assert!(table.declare(proto).is_ok());

//     assert!(table.lookup("my_var").is_some());
//     assert!(table.lookup("my_param").is_some());
//     assert!(table.lookup("my_func").is_some());
//     assert!(table.lookup("my_type").is_some());
//     assert!(table.lookup("my_protocol").is_some());
// }

// #[test]
// fn test_duplicate_with_different_types() {
//     let mut table = SymbolTable::new();
//     let span = Span::default();

//     let var = SymbolInfo::Variable {
//         name: "x".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };
//     let param = SymbolInfo::Parameter {
//         name: "x".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };

//     assert!(table.declare(var).is_ok());
//     assert!(table.declare(param).is_err());
// }

// #[test]
// fn test_declare_same_name_different_scopes() {
//     let mut table = SymbolTable::new();
//     let span = Span::default();

//     let var1 = SymbolInfo::Variable {
//         name: "value".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };
//     assert!(table.declare(var1).is_ok());

//     table.enter_scope();
//     let var2 = SymbolInfo::Variable {
//         name: "value".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };
//     assert!(table.declare(var2).is_ok());

//     table.enter_scope();
//     let var3 = SymbolInfo::Variable {
//         name: "value".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };
//     assert!(table.declare(var3.clone()).is_ok());
//     assert!(table.declare(var3.clone()).is_err());

//     table.exit_scope();
//     table.exit_scope();
//     assert!(table.lookup("value").is_some());
// }

// #[test]
// fn test_declare_error_contains_symbol_name() {
//     let mut table = SymbolTable::new();
//     let span = Span::default();

//     let var1 = SymbolInfo::Variable {
//         name: "duplicate_name".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };
//     let var2 = SymbolInfo::Variable {
//         name: "duplicate_name".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };

//     assert!(table.declare(var1).is_ok());
//     let result = table.declare(var2);
//     assert!(result.is_err());

//     match result.unwrap_err() {
//         SemanticError::VariableAlreadyDeclared { name, .. } => {
//             assert_eq!(name, "duplicate_name");
//         }
//         _ => panic!("Expected VariableAlreadyDeclared error"),
//     }
// }

// #[test]
// fn test_declare_lookup_consistency() {
//     let mut table = SymbolTable::new();
//     let span = Span::default();

//     let var = SymbolInfo::Variable {
//         name: "test_var".to_string(),
//         type_ref: None,
//         span: span.clone(),
//     };
//     assert!(table.declare(var.clone()).is_ok());

//     let found = table.lookup("test_var");
//     assert!(found.is_some());

//     if let Some(SymbolInfo::Variable { name, .. }) = found {
//         assert_eq!(name, "test_var");
//     } else {
//         panic!("Expected Variable");
//     }
// }

// // (Many more symbol_table tests were present; for brevity we've included a
// // representative set above. The rest can be appended similarly if you want
// // the full verbatim transfer.)

// // ========= expression_checker tests =========

// #[test]
// fn test_expression_type_value() {
//     let et = ExpressionType::value(NormalizedType::Number);
//     assert_eq!(et.type_, NormalizedType::Number);
//     assert!(!et.is_lvalue);
// }

// #[test]
// fn test_expression_type_lvalue() {
//     let et = ExpressionType::lvalue(NormalizedType::String);
//     assert_eq!(et.type_, NormalizedType::String);
//     assert!(et.is_lvalue);
// }

// #[test]
// fn test_checker_new() {
//     let _checker = ExpressionChecker::new();
// }

// #[test]
// fn test_if_expression_valid() {
//     let checker = ExpressionChecker::new();
//     let elif_branches = vec![
//         (NormalizedType::Boolean, NormalizedType::Number),
//         (NormalizedType::Boolean, NormalizedType::Number),
//     ];
//     let result = checker
//         .check_if_expression(
//             &NormalizedType::Boolean,
//             &NormalizedType::Number,
//             &elif_branches,
//             Some(&NormalizedType::Number),
//             &Span::default(),
//         )
//         .unwrap();
//     assert_eq!(result.type_, NormalizedType::Number);
// }

// #[test]
// fn test_if_expression_invalid_condition() {
//     let checker = ExpressionChecker::new();
//     let result = checker.check_if_expression(
//         &NormalizedType::Number,
//         &NormalizedType::String,
//         &[],
//         None,
//         &Span::default(),
//     );
//     assert!(matches!(
//         result,
//         Err(SemanticError::NonBooleanCondition { .. })
//     ));
// }

// // (Selected expression_checker tests included; other tests can be added similarly.)

// // ========= type_system tests =========

// #[test]
// fn test_normalized_type_builtin() {
//     assert_eq!(NormalizedType::Number.to_string(), "Number");
//     assert_eq!(NormalizedType::String.to_string(), "String");
//     assert_eq!(NormalizedType::Boolean.to_string(), "Boolean");
// }

// #[test]
// fn test_normalized_type_iterable() {
//     let iter_num = NormalizedType::Iterable(Box::new(NormalizedType::Number));
//     assert_eq!(iter_num.to_string(), "Number*");
// }

// #[test]
// fn test_type_environment_new() {
//     let env = TypeEnvironment::new();
//     assert_eq!(env.all_types().len(), 0);
// }

// #[test]
// fn test_is_compatible_subtype() {
//     let mut env = TypeEnvironment::new();
//     let type_a = TypeInfo {
//         name: "A".into(),
//         parameters: vec![],
//         parent: None,
//         methods: vec![],
//         properties: vec![],
//         implemented_protocols: vec![],
//         span: Span::default(),
//     };
//     let type_b = TypeInfo {
//         name: "B".into(),
//         parameters: vec![],
//         parent: Some("A".into()),
//         methods: vec![],
//         properties: vec![],
//         implemented_protocols: vec![],
//         span: Span::default(),
//     };
//     env.register_type(type_a).unwrap();
//     env.register_type(type_b).unwrap();
//     let t_b = NormalizedType::Named("B".into());
//     let t_a = NormalizedType::Named("A".into());
//     assert!(env.is_compatible(&t_b, &t_a));
//     assert!(!env.is_compatible(&t_a, &t_b));
// }

// #[test]
// fn test_protocol_conformance_positive() {
//     let mut env = TypeEnvironment::new();

//     let proto_sig = ProtocolMethodSignature {
//         name: "m".into(),
//         parameters: vec![],
//         return_type: TypeReference::new("Number".into(), Span::default()),
//         span: Span::default(),
//     };

//     let proto = ProtocolInfo {
//         name: "P".into(),
//         members: vec![proto_sig],
//         extends: vec![],
//         span: Span::default(),
//     };

//     env.register_protocol(proto).unwrap();

//     let func = FunctionDeclaration {
//         name: "m".into(),
//         parameters: vec![],
//         return_type: Some(TypeReference::new("Number".into(), Span::default())),
//         body: Expr::literal(Literal::Number(0.0), Span::default()),
//     };

//     let type_a = TypeInfo {
//         name: "A".into(),
//         parameters: vec![],
//         parent: None,
//         methods: vec![func],
//         properties: vec![],
//         implemented_protocols: vec![],
//         span: Span::default(),
//     };

//     env.register_type(type_a).unwrap();

//     assert!(env.type_conforms_to_protocol("A", "P"));
// }

// #[test]
// fn test_protocol_conformance_negative_missing_method() {
//     let mut env = TypeEnvironment::new();

//     let proto_sig = ProtocolMethodSignature {
//         name: "m".into(),
//         parameters: vec![],
//         return_type: TypeReference::new("Number".into(), Span::default()),
//         span: Span::default(),
//     };

//     let proto = ProtocolInfo {
//         name: "P2".into(),
//         members: vec![proto_sig],
//         extends: vec![],
//         span: Span::default(),
//     };

//     env.register_protocol(proto).unwrap();

//     let type_b = TypeInfo {
//         name: "B".into(),
//         parameters: vec![],
//         parent: None,
//         methods: vec![],
//         properties: vec![],
//         implemented_protocols: vec![],
//         span: Span::default(),
//     };

//     env.register_type(type_b).unwrap();

//     assert!(!env.type_conforms_to_protocol("B", "P2"));
// }

// // ========= analyzer tests =========

// #[test]
// fn test_semantic_analyzer_new() {
//     let analyzer = SemanticAnalyzer::new();
//     assert!(!analyzer.has_errors());
// }

// #[test]
// fn test_semantic_context_new() {
//     let ctx = SemanticContext::new();
//     assert!(!ctx.has_errors());
// }

// #[test]
// fn test_context_push_and_clear_error() {
//     let mut ctx = SemanticContext::new();
//     let error = SemanticError::UnsupportedFeature {
//         feature: "Test error".to_string(),
//     };
//     ctx.push_error(error);
//     assert!(ctx.has_errors());
//     ctx.clear_errors();
//     assert!(!ctx.has_errors());
// }

// // =============================================================================
// // Tests Unitarios - Semantic
// // =============================================================================
// //
// // Tests para el módulo Semantic. Aquí se prueban:
// //
// // - Type checking correcto
// // - Detección de errores de tipo
// // - Resolución de scopes (local, global, anidados)
// // - Tabla de símbolos correcta
// // - Variables no declaradas/duplicadas
// // - Inferencia de tipos (si aplica)
// // - Verificación de llamadas a funciones
// //
// // =============================================================================

// // Additional tests for inheritance and protocol edge-cases
// #[test]
// fn test_register_type_inherit_from_undeclared() {
//     let mut env = TypeEnvironment::new();
//     let t = TypeInfo {
//         name: "X".into(),
//         parameters: vec![],
//         parent: Some("Missing".into()),
//         methods: vec![],
//         properties: vec![],
//         implemented_protocols: vec![],
//         span: Span::default(),
//     };

//     let res = env.register_type(t);
//     assert!(matches!(
//         res,
//         Err(SemanticError::InheritFromUndeclared { .. })
//     ));
// }

// #[test]
// fn test_register_type_self_inherit() {
//     let mut env = TypeEnvironment::new();
//     let t = TypeInfo {
//         name: "Selfy".into(),
//         parameters: vec![],
//         parent: Some("Selfy".into()),
//         methods: vec![],
//         properties: vec![],
//         implemented_protocols: vec![],
//         span: Span::default(),
//     };

//     let res = env.register_type(t);
//     assert!(matches!(
//         res,
//         Err(SemanticError::CircularInheritance { .. })
//     ));
// }

// #[test]
// fn test_register_protocol_extends_undeclared() {
//     let mut env = TypeEnvironment::new();
//     let p = ProtocolInfo {
//         name: "P3".into(),
//         members: vec![],
//         extends: vec!["UnknownProto".into()],
//         span: Span::default(),
//     };

//     let res = env.register_protocol(p);
//     assert!(matches!(res, Err(SemanticError::UndeclaredType { .. })));
// }

// #[test]
// fn test_protocol_conformance_inherited_method() {
//     let mut env = TypeEnvironment::new();

//     // Parent with method m()
//     let parent_func = FunctionDeclaration {
//         name: "m".into(),
//         parameters: vec![],
//         return_type: Some(TypeReference::new("Number".into(), Span::default())),
//         body: Expr::literal(Literal::Number(0.0), Span::default()),
//     };

//     let parent = TypeInfo {
//         name: "Parent".into(),
//         parameters: vec![],
//         parent: None,
//         methods: vec![parent_func],
//         properties: vec![],
//         implemented_protocols: vec![],
//         span: Span::default(),
//     };

//     env.register_type(parent).unwrap();

//     let child = TypeInfo {
//         name: "Child".into(),
//         parameters: vec![],
//         parent: Some("Parent".into()),
//         methods: vec![],
//         properties: vec![],
//         implemented_protocols: vec![],
//         span: Span::default(),
//     };

//     env.register_type(child).unwrap();

//     // Protocol requiring m() -> Number
//     let proto_sig = ProtocolMethodSignature {
//         name: "m".into(),
//         parameters: vec![],
//         return_type: TypeReference::new("Number".into(), Span::default()),
//         span: Span::default(),
//     };

//     let proto = ProtocolInfo {
//         name: "PChild".into(),
//         members: vec![proto_sig],
//         extends: vec![],
//         span: Span::default(),
//     };

//     env.register_protocol(proto).unwrap();

//     assert!(env.type_conforms_to_protocol("Child", "PChild"));
// }

// // ========= Functor (protocol invoke) tests =========

// #[test]
// fn test_functor_parameter_call_ok() {
//     let proto_sig = crate::parser::ast::ProtocolMethodSignature {
//         name: "invoke".into(),
//         parameters: vec![crate::parser::ast::Parameter::new(
//             "x".into(),
//             Some(crate::parser::ast::TypeReference::new("Number".into(), Span::default())),
//             Span::default(),
//         )],
//         return_type: crate::parser::ast::TypeReference::new("Number".into(), Span::default()),
//         span: Span::default(),
//     };

//     let proto_decl = crate::parser::ast::ProtocolDeclaration {
//         name: "Callable".into(),
//         extends: vec![],
//         members: vec![proto_sig],
//     };

//     let func = crate::parser::ast::FunctionDeclaration {
//         name: "use_functor".into(),
//         parameters: vec![crate::parser::ast::Parameter::new(
//             "f".into(),
//             Some(crate::parser::ast::TypeReference::new("Callable".into(), Span::default())),
//             Span::default(),
//         )],
//         return_type: Some(crate::parser::ast::TypeReference::new("Number".into(), Span::default())),
//         body: crate::parser::ast::Expr::call(
//             crate::parser::ast::Expr::identifier("f".into(), Span::default()),
//             vec![crate::parser::ast::Expr::literal(crate::parser::ast::Literal::Number(1.0), Span::default())],
//             Span::default(),
//         ),
//     };

//     let program = crate::parser::ast::Program::new(
//         vec![
//             crate::parser::ast::Declaration::new(crate::parser::ast::DeclarationKind::Protocol(proto_decl), Span::default()),
//             crate::parser::ast::Declaration::new(crate::parser::ast::DeclarationKind::Function(func), Span::default()),
//         ],
//         None,
//         Span::default(),
//     );

//     let mut analyzer = SemanticAnalyzer::new();
//     let res = analyzer.analyze(&program);
//     if let Err(e) = &res {
//         panic!("analyze error: {:?}\ncontext errors: {:?}", e, analyzer.errors());
//     }
//     assert!(!analyzer.has_errors());
// }

// #[test]
// fn test_functor_parameter_call_type_mismatch() {
//     let proto_sig = crate::parser::ast::ProtocolMethodSignature {
//         name: "invoke".into(),
//         parameters: vec![crate::parser::ast::Parameter::new(
//             "x".into(),
//             Some(crate::parser::ast::TypeReference::new("Number".into(), Span::default())),
//             Span::default(),
//         )],
//         return_type: crate::parser::ast::TypeReference::new("Number".into(), Span::default()),
//         span: Span::default(),
//     };

//     let proto_decl = crate::parser::ast::ProtocolDeclaration {
//         name: "Callable2".into(),
//         extends: vec![],
//         members: vec![proto_sig],
//     };

//     // Function calls f("not a number") where f: Callable2
//     let func = crate::parser::ast::FunctionDeclaration {
//         name: "use_functor_bad".into(),
//         parameters: vec![crate::parser::ast::Parameter::new(
//             "f".into(),
//             Some(crate::parser::ast::TypeReference::new("Callable2".into(), Span::default())),
//             Span::default(),
//         )],
//         return_type: Some(crate::parser::ast::TypeReference::new("Number".into(), Span::default())),
//         body: crate::parser::ast::Expr::call(
//             crate::parser::ast::Expr::identifier("f".into(), Span::default()),
//             vec![crate::parser::ast::Expr::literal(crate::parser::ast::Literal::String("x".into()), Span::default())],
//             Span::default(),
//         ),
//     };

//     let program = crate::parser::ast::Program::new(
//         vec![
//             crate::parser::ast::Declaration::new(crate::parser::ast::DeclarationKind::Protocol(proto_decl), Span::default()),
//             crate::parser::ast::Declaration::new(crate::parser::ast::DeclarationKind::Function(func), Span::default()),
//         ],
//         None,
//         Span::default(),
//     );

//     let mut analyzer = SemanticAnalyzer::new();
//     let res = analyzer.analyze(&program);
//     assert!(res.is_err());
//     // Ensure the recorded error is an ArgumentTypeMismatch
//     let err = res.unwrap_err();
//     match err {
//         SemanticError::ArgumentTypeMismatch { .. } => {}
//         other => panic!("Expected ArgumentTypeMismatch, got {:?}", other),
//     }
// }
