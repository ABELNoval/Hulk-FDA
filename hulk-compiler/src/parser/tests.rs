// =============================================================================
// Tests Unitarios - Parser
// =============================================================================
//
// Tests para el módulo Parser. Aquí se prueban:
//
// - Parsing correcto de cada construcción del lenguaje
// - Precedencia y asociatividad de operadores
// - Errores sintácticos bien formados
// - Recuperación de errores
// - AST generado correctamente
// - Casos límite (expresiones anidadas, empty blocks)
//
// =============================================================================

#[cfg(test)]
mod tests_parser {
    use crate::lexer::Token;
    use crate::lexer::TokenType;
    use crate::parser::TokenCursor;
    use crate::parser::ast::{
        BinaryOperator, Declaration, DeclarationKind, Expr, ExprKind, FunctionDeclaration, Literal,
        Program, ProtocolDeclaration, ProtocolMethodSignature, TypeDeclaration, TypeMember,
        TypeReference, TypeReferenceKind,
    };
    use crate::utils::errors::span::Span;

    // =========================================================================
    // Utilidades para crear tokens de prueba
    // =========================================================================

    fn create_token(lexeme: &str, token_type: TokenType) -> Token {
        Token::new(
            lexeme.to_string(),
            token_type,
            Span::new("test".to_string(), 1, 1, 1, 1),
        )
    }

    // =========================================================================
    // Tests para TokenCursor
    // =========================================================================

    #[test]
    fn test_cursor_peek() {
        let tokens = vec![
            create_token("+", TokenType::Plus),
            create_token("5", TokenType::Number(5.0)),
            create_token("*", TokenType::Star),
            Token::eof("test".to_string(), 1, 1),
        ];

        let cursor = TokenCursor::new(tokens);

        // peek() debe retornar el primer token sin avanzar
        assert_eq!(cursor.peek().token_type, TokenType::Plus);
        assert_eq!(cursor.current_position(), 0);
    }

    #[test]
    fn test_cursor_advance() {
        let tokens = vec![
            create_token("+", TokenType::Plus),
            create_token("5", TokenType::Number(5.0)),
            create_token("*", TokenType::Star),
            Token::eof("test".to_string(), 1, 1),
        ];

        let mut cursor = TokenCursor::new(tokens);

        // advance() debe retornar el token actual y mover a la siguiente posición
        assert_eq!(cursor.peek().token_type, TokenType::Plus);
        cursor.advance();
        assert_eq!(cursor.current_position(), 1);
        assert_eq!(cursor.peek().token_type, TokenType::Number(5.0));
    }

    #[test]
    fn test_cursor_consume_match() {
        let tokens = vec![
            create_token("let", TokenType::Let),
            create_token("x", TokenType::Identifier("x".to_string())),
            create_token("=", TokenType::Equal),
            Token::eof("test".to_string(), 1, 1),
        ];

        let mut cursor = TokenCursor::new(tokens);

        // consume() debe retornar el token si coincide el tipo
        assert!(cursor.consume(&TokenType::Let).is_some());
        assert_eq!(cursor.current_position(), 1);

        // consume() debe retornar None si no coincide el tipo
        assert!(cursor.consume(&TokenType::Plus).is_none());
        assert_eq!(cursor.current_position(), 1);
    }

    #[test]
    fn test_cursor_check() {
        let tokens = vec![
            create_token("if", TokenType::If),
            create_token("(", TokenType::LeftParen),
            create_token("x", TokenType::Identifier("x".to_string())),
            Token::eof("test".to_string(), 1, 1),
        ];

        let cursor = TokenCursor::new(tokens);

        // check() debe verificar sin avanzar
        assert!(cursor.check(&TokenType::If));
        assert!(!cursor.check(&TokenType::LeftParen));
        assert_eq!(cursor.current_position(), 0);
    }

    #[test]
    fn test_cursor_check_any() {
        let tokens = vec![
            create_token("if", TokenType::If),
            create_token("(", TokenType::LeftParen),
            Token::eof("test".to_string(), 1, 1),
        ];

        let cursor = TokenCursor::new(tokens);

        // check_any() debe verificar si coincide con alguno de los tipos
        assert!(cursor.check_any(&[TokenType::While, TokenType::If]));
        assert!(!cursor.check_any(&[TokenType::While, TokenType::For]));
    }

    #[test]
    fn test_cursor_match_token() {
        let tokens = vec![
            create_token("+", TokenType::Plus),
            create_token("5", TokenType::Number(5.0)),
            Token::eof("test".to_string(), 1, 1),
        ];

        let mut cursor = TokenCursor::new(tokens);

        // match_token() debe avanzar solo si coincide
        assert!(cursor.match_token(&TokenType::Plus));
        assert_eq!(cursor.current_position(), 1);

        assert!(!cursor.match_token(&TokenType::Plus));
        assert_eq!(cursor.current_position(), 1);
    }

    #[test]
    fn test_cursor_match_any() {
        let tokens = vec![
            create_token("&", TokenType::Ampersand),
            create_token("x", TokenType::Identifier("x".to_string())),
            Token::eof("test".to_string(), 1, 1),
        ];

        let mut cursor = TokenCursor::new(tokens);

        // match_any() debe avanzar si coincide con alguno de los tipos
        assert!(cursor.match_any(&[TokenType::Pipe, TokenType::Ampersand]));
        assert_eq!(cursor.current_position(), 1);

        assert!(!cursor.match_any(&[TokenType::Plus, TokenType::Minus]));
        assert_eq!(cursor.current_position(), 1);
    }

    #[test]
    fn test_cursor_backtrack() {
        let tokens = vec![
            create_token("+", TokenType::Plus),
            create_token("5", TokenType::Number(5.0)),
            create_token("*", TokenType::Star),
            Token::eof("test".to_string(), 1, 1),
        ];

        let mut cursor = TokenCursor::new(tokens);

        // Avanzar y luego retroceder
        cursor.advance();
        cursor.advance();
        assert_eq!(cursor.current_position(), 2);

        cursor.backtrack();
        assert_eq!(cursor.current_position(), 1);
    }

    #[test]
    fn test_cursor_is_at_end() {
        let tokens = vec![
            create_token("+", TokenType::Plus),
            Token::eof("test".to_string(), 1, 1),
        ];

        let mut cursor = TokenCursor::new(tokens);

        assert!(!cursor.is_at_end());
        cursor.advance();
        assert!(cursor.is_at_end());
    }

    #[test]
    fn test_cursor_set_position() {
        let tokens = vec![
            create_token("+", TokenType::Plus),
            create_token("5", TokenType::Number(5.0)),
            create_token("*", TokenType::Star),
            Token::eof("test".to_string(), 1, 1),
        ];

        let mut cursor = TokenCursor::new(tokens);

        // set_position() debe establecer la posición
        cursor.set_position(2);
        assert_eq!(cursor.current_position(), 2);
        assert_eq!(cursor.peek().token_type, TokenType::Star);

        // No debe permitir ir más allá del final
        cursor.set_position(100);
        assert!(cursor.current_position() < 100);
    }

    #[test]
    fn test_cursor_token_count() {
        let tokens = vec![
            create_token("+", TokenType::Plus),
            create_token("5", TokenType::Number(5.0)),
            create_token("*", TokenType::Star),
            Token::eof("test".to_string(), 1, 1),
        ];

        let cursor = TokenCursor::new(tokens);

        // token_count() debe retornar el número total de tokens
        assert_eq!(cursor.token_count(), 4);
    }

    #[test]
    fn test_program_ast_root() {
        let span = Span::new("test".to_string(), 1, 1, 1, 10);
        let entry = Some(Expr::literal(Literal::Number(1.0), span.clone()));
        let program = Program::new(Vec::new(), entry, span.clone());

        assert!(program.declarations.is_empty());
        assert!(program.entry_expression.is_some());
        assert_eq!(program.span, span);
    }

    #[test]
    fn test_parse_program_with_declaration_and_entry_expression() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "function main() => 1; 2";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let program = parser.parse_program();

        assert_eq!(program.declarations.len(), 1);
        assert!(program.entry_expression.is_some());

        if let Some(entry) = program.entry_expression {
            assert!(matches!(entry.kind, ExprKind::Literal(Literal::Number(_))));
        }
    }

    #[test]
    fn test_expr_binary_node() {
        let span = Span::new("test".to_string(), 1, 1, 1, 5);
        let left = Expr::literal(Literal::Number(2.0), span.clone());
        let right = Expr::literal(Literal::Number(3.0), span.clone());
        let expr = Expr::binary(left, BinaryOperator::Add, right, span.clone());

        match expr.kind {
            ExprKind::Binary { operator, .. } => assert_eq!(operator, BinaryOperator::Add),
            _ => panic!("se esperaba un nodo binario"),
        }
        assert_eq!(expr.span, span);
    }

    #[test]
    fn test_declaration_function_shape() {
        let span = Span::new("test".to_string(), 1, 1, 1, 15);
        let return_type = TypeReference::new("Number".to_string(), span.clone());
        let body = Expr::literal(Literal::Number(42.0), span.clone());
        let function = Declaration::new(
            DeclarationKind::Function(FunctionDeclaration {
                name: "answer".to_string(),
                parameters: Vec::new(),
                return_type: Some(return_type),
                body,
            }),
            span.clone(),
        );

        match function.kind {
            DeclarationKind::Function(ref declaration) => {
                assert_eq!(declaration.name, "answer");
                assert!(declaration.parameters.is_empty());
            }
            _ => panic!("se esperaba una declaración de función"),
        }
        assert_eq!(function.span, span);
    }

    #[test]
    fn test_if_expr_node() {
        let span = Span::new("test".to_string(), 1, 1, 1, 20);
        let condition = Expr::identifier("x".to_string(), span.clone());
        let then_expr = Expr::literal(Literal::Number(1.0), span.clone());
        let else_expr = Some(Expr::literal(Literal::Number(0.0), span.clone()));

        let if_node = Expr::if_expr(condition, then_expr, vec![], else_expr, span.clone());

        match if_node.kind {
            ExprKind::If {
                ref condition,
                ref then_expr,
                ref else_expr,
                ..
            } => {
                assert!(matches!(condition.kind, ExprKind::Identifier(_)));
                assert!(matches!(
                    then_expr.kind,
                    ExprKind::Literal(Literal::Number(1.0))
                ));
                assert!(else_expr.is_some());
            }
            _ => panic!("se esperaba una expresión if"),
        }
    }

    #[test]
    fn test_while_expr_node() {
        let span = Span::new("test".to_string(), 1, 1, 1, 15);
        let condition = Expr::identifier("keep_going".to_string(), span.clone());
        let body = Expr::literal(Literal::Number(42.0), span.clone());

        let while_node = Expr::while_expr(condition, body, span.clone());

        match while_node.kind {
            ExprKind::While {
                ref condition,
                ref body,
            } => {
                assert!(matches!(condition.kind, ExprKind::Identifier(_)));
                assert!(matches!(body.kind, ExprKind::Literal(_)));
            }
            _ => panic!("se esperaba una expresión while"),
        }
    }

    #[test]
    fn test_for_expr_node() {
        let span = Span::new("test".to_string(), 1, 1, 1, 15);
        let iterable = Expr::identifier("items".to_string(), span.clone());
        let body = Expr::identifier("i".to_string(), span.clone());

        let for_node = Expr::for_expr("i".to_string(), iterable, body, span.clone());

        match for_node.kind {
            ExprKind::For {
                ref variable,
                ref iterable,
                ref body,
            } => {
                assert_eq!(variable, "i");
                assert!(matches!(iterable.kind, ExprKind::Identifier(_)));
                assert!(matches!(body.kind, ExprKind::Identifier(_)));
            }
            _ => panic!("se esperaba una expresión for"),
        }
    }

    #[test]
    fn test_let_expr_node() {
        let span = Span::new("test".to_string(), 1, 1, 1, 15);
        let annotation = Some(TypeReference::new("Number".to_string(), span.clone()));
        let value = Some(Expr::literal(Literal::Number(5.0), span.clone()));

        let let_node = Expr::let_expr("x".to_string(), annotation.clone(), value, span.clone());

        match &let_node.kind {
            ExprKind::Let {
                name,
                annotation: ann,
                value: val,
            } => {
                assert_eq!(name, "x");
                assert!(ann.is_some());
                assert!(val.is_some());
            }
            _ => panic!("se esperaba una expresión let"),
        }
    }

    #[test]
    fn test_member_access_node() {
        let span = Span::new("test".to_string(), 1, 1, 1, 10);
        let object = Expr::identifier("obj".to_string(), span.clone());

        let access = Expr::member_access(object, "property".to_string(), span.clone());

        match &access.kind {
            ExprKind::MemberAccess { object: _, member } => {
                assert_eq!(member, "property");
            }
            _ => panic!("se esperaba member access"),
        }
    }

    #[test]
    fn test_type_check_node() {
        let span = Span::new("test".to_string(), 1, 1, 1, 10);
        let expr = Expr::identifier("x".to_string(), span.clone());
        let type_ref = TypeReference::new("Number".to_string(), span.clone());

        let check = Expr::type_check(expr, type_ref, span.clone());

        match &check.kind {
            ExprKind::TypeCheck {
                expr: _,
                type_ref: tr,
            } => {
                assert_eq!(tr.display_name(), "Number");
            }
            _ => panic!("se esperaba type check"),
        }
    }

    #[test]
    fn test_new_expr_node() {
        let span = Span::new("test".to_string(), 1, 1, 1, 10);
        let type_ref = TypeReference::new("MyClass".to_string(), span.clone());
        let arguments = vec![Expr::literal(Literal::Number(1.0), span.clone())];

        let new_node = Expr::new_expr(type_ref.clone(), arguments, span.clone());

        match &new_node.kind {
            ExprKind::New {
                type_ref: tr,
                arguments: args,
            } => {
                assert_eq!(tr.display_name(), "MyClass");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("se esperaba new expression"),
        }
    }

    #[test]
    fn test_vector_literal_expr_node() {
        let span = Span::new("test".to_string(), 1, 1, 1, 10);
        let elements = vec![
            Expr::literal(Literal::Number(1.0), span.clone()),
            Expr::literal(Literal::Number(2.0), span.clone()),
        ];

        let vector = Expr::vector_literal(elements, span.clone());

        match vector.kind {
            ExprKind::VectorLiteral(items) => assert_eq!(items.len(), 2),
            _ => panic!("se esperaba vector literal"),
        }
    }

    #[test]
    fn test_vector_comprehension_expr_node() {
        let span = Span::new("test".to_string(), 1, 1, 1, 20);
        let element_expr = Expr::binary(
            Expr::identifier("x".to_string(), span.clone()),
            BinaryOperator::Power,
            Expr::literal(Literal::Number(2.0), span.clone()),
            span.clone(),
        );
        let iterable = Expr::identifier("numbers".to_string(), span.clone());

        let vector =
            Expr::vector_comprehension(element_expr, "x".to_string(), iterable, span.clone());

        match vector.kind {
            ExprKind::VectorComprehension { binding, .. } => assert_eq!(binding, "x"),
            _ => panic!("se esperaba vector comprehension"),
        }
    }

    #[test]
    fn test_type_reference_iterable_and_vector() {
        let span = Span::new("test".to_string(), 1, 1, 1, 8);
        let number = TypeReference::new("Number".to_string(), span.clone());
        let iterable = TypeReference::iterable_of(number.clone(), span.clone());
        let vector = TypeReference::vector_of(number, span.clone());

        assert_eq!(iterable.display_name(), "Number*");
        assert_eq!(vector.display_name(), "Number[]");
        assert!(matches!(iterable.kind, TypeReferenceKind::Iterable(_)));
        assert!(matches!(vector.kind, TypeReferenceKind::Vector(_)));
    }

    #[test]
    fn test_parse_type_reference_iterable() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "Number*";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let type_reference = parser.parse_type_reference();

        assert_eq!(type_reference.display_name(), "Number*");
        assert!(matches!(
            type_reference.kind,
            TypeReferenceKind::Iterable(_)
        ));
    }

    #[test]
    fn test_parse_type_reference_vector() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "Number[]";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let type_reference = parser.parse_type_reference();

        assert_eq!(type_reference.display_name(), "Number[]");
        assert!(matches!(type_reference.kind, TypeReferenceKind::Vector(_)));
    }

    #[test]
    fn test_protocol_method_signature_has_no_body() {
        let span = Span::new("test".to_string(), 1, 1, 1, 20);
        let method = ProtocolMethodSignature {
            name: "next".to_string(),
            parameters: Vec::new(),
            return_type: TypeReference::new("Boolean".to_string(), span.clone()),
            span: span.clone(),
        };
        let protocol = ProtocolDeclaration {
            name: "Iterable".to_string(),
            extends: Vec::new(),
            members: vec![method],
        };

        assert_eq!(protocol.members.len(), 1);
        assert_eq!(protocol.members[0].name, "next");
    }

    #[test]
    fn test_type_declaration_supports_parent_arguments() {
        let span = Span::new("test".to_string(), 1, 1, 1, 30);
        let decl = TypeDeclaration {
            name: "PolarPoint".to_string(),
            parameters: Vec::new(),
            inherits: Some(TypeReference::new("Point".to_string(), span.clone())),
            parent_arguments: vec![Expr::literal(Literal::Number(1.0), span.clone())],
            members: Vec::new(),
        };

        assert!(decl.inherits.is_some());
        assert_eq!(decl.parent_arguments.len(), 1);
    }

    #[test]
    fn test_parse_protocol_declaration() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "protocol Iterable extends Seq { next(): Boolean; reset(): Void }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let declaration = parser.parse_declaration();

        assert!(
            declaration.is_some(),
            "Debe parsear una declaración de protocolo"
        );

        let declaration = declaration.unwrap();
        match declaration.kind {
            DeclarationKind::Protocol(protocol) => {
                assert_eq!(protocol.name, "Iterable");
                assert_eq!(protocol.extends.len(), 1);
                assert_eq!(protocol.members.len(), 2);
                assert_eq!(protocol.members[0].name, "next");
            }
            _ => panic!("Se esperaba DeclarationKind::Protocol"),
        }
    }

    #[test]
    fn test_self_expr_node() {
        let span = Span::new("test".to_string(), 1, 1, 1, 5);
        let self_node = Expr::self_expr(span.clone());

        match self_node.kind {
            ExprKind::Self_ => {
                // Verificar que es self
            }
            _ => panic!("se esperaba self expression"),
        }
    }

    #[test]
    fn test_break_expr_node() {
        let span = Span::new("test".to_string(), 1, 1, 1, 5);
        let break_node = Expr::break_expr(span.clone());

        match break_node.kind {
            ExprKind::Break => {
                // Verificar que es break
            }
            _ => panic!("se esperaba break expression"),
        }
    }

    #[test]
    fn test_return_expr_node() {
        let span = Span::new("test".to_string(), 1, 1, 1, 10);
        let value = Some(Expr::literal(Literal::Number(42.0), span.clone()));
        let return_node = Expr::return_expr(value, span.clone());

        match return_node.kind {
            ExprKind::Return(ref val) => {
                assert!(val.is_some());
            }
            _ => panic!("se esperaba return expression"),
        }
    }

    // =========================================================================
    // Tests para parse_block() - Bloques de código
    // =========================================================================

    #[test]
    fn test_parse_block_from_lexer() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "{ 5 }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression();

        match &expr.kind {
            ExprKind::Block(exprs) => {
                assert_eq!(exprs.len(), 1, "Bloque debe tener 1 expresión");
            }
            _ => panic!("Se esperaba Block, obtuvo {:?}", expr.kind),
        }
    }

    #[test]
    fn test_parse_empty_block() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "{}";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression();

        match &expr.kind {
            ExprKind::Block(exprs) => {
                assert_eq!(exprs.len(), 0, "Bloque vacío debe tener 0 expresiones");
            }
            _ => panic!("Se esperaba Block"),
        }
    }

    #[test]
    fn test_parse_multiple_expr_block() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "{ 5; 10; 15 }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression();

        match &expr.kind {
            ExprKind::Block(exprs) => {
                assert_eq!(exprs.len(), 3, "Bloque debe tener 3 expresiones");
            }
            _ => panic!("Se esperaba Block"),
        }
    }

    // =========================================================================
    // Tests para parse_let_binding() - Let expressions
    // =========================================================================

    #[test]
    fn test_parse_simple_let() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "let x = 5";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression();

        match &expr.kind {
            ExprKind::Let {
                name,
                value,
                annotation,
            } => {
                assert_eq!(name, "x", "Variable debe ser 'x'");
                assert!(annotation.is_none(), "No debe haber anotación de tipo");
                assert!(value.is_some(), "Debe haber un valor");
            }
            _ => panic!("Se esperaba Let, obtuvo {:?}", expr.kind),
        }
    }

    #[test]
    fn test_parse_let_with_type_annotation() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        // Note: Este test no prueba la anotación de tipo completamente
        // porque parse_type_reference() no está implementado
        // Solo verificamos que la estructura Let se crea correctamente
        let code = "let x = 5";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression();

        match &expr.kind {
            ExprKind::Let {
                name,
                value,
                annotation: _,
            } => {
                assert_eq!(name, "x");
                assert!(value.is_some());
            }
            _ => panic!("Se esperaba Let con anotación"),
        }
    }

    // =========================================================================
    // Tests para parse_if_expr() - Condicionales
    // =========================================================================

    #[test]
    fn test_parse_simple_if() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "if (true) { 5 }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression();

        match &expr.kind {
            ExprKind::If {
                condition: _,
                then_expr: _,
                elif_parts,
                else_expr,
            } => {
                // La estructura se parsea correctamente
                assert_eq!(elif_parts.len(), 0, "No debe haber elif");
                assert!(else_expr.is_none(), "No debe haber else");
            }
            _ => panic!("Se esperaba If"),
        }
    }

    #[test]
    fn test_parse_if_else() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "if (true) { 1 } else { 2 }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression();

        match &expr.kind {
            ExprKind::If {
                condition: _,
                then_expr: _,
                elif_parts,
                else_expr,
            } => {
                // Verificar estructura
                assert_eq!(elif_parts.len(), 0);
                assert!(else_expr.is_some(), "Debe haber else");
            }
            _ => panic!("Se esperaba If/else"),
        }
    }

    #[test]
    fn test_parse_if_elif_else() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "if (false) { 1 } elif (true) { 2 } else { 3 }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression();

        match &expr.kind {
            ExprKind::If {
                condition: _,
                then_expr: _,
                elif_parts,
                else_expr,
            } => {
                // Verificar estructura
                assert_eq!(elif_parts.len(), 1, "Debe haber 1 elif");
                assert!(else_expr.is_some(), "Debe haber else");
            }
            _ => panic!("Se esperaba If/elif/else"),
        }
    }

    // =========================================================================
    // Tests para parse_while_expr() - While loops
    // =========================================================================

    #[test]
    fn test_parse_simple_while() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "while (true) { 5 }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression();

        match &expr.kind {
            ExprKind::While {
                condition: _,
                body: _,
            } => {
                // La estructura se parsea correctamente
                // Ambos campos siempre existen (Box<Expr>)
            }
            _ => panic!("Se esperaba While"),
        }
    }

    // =========================================================================
    // Tests para parse_for_expr() - For loops
    // =========================================================================

    #[test]
    fn test_parse_simple_for() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "for i in range(1, 10) { 5 }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression();

        match &expr.kind {
            ExprKind::For {
                variable,
                iterable: _,
                body: _,
            } => {
                assert_eq!(variable, "i", "Variable debe ser 'i'");
                // iterable y body son Box<Expr>, siempre existen
            }
            _ => panic!("Se esperaba For"),
        }
    }

    // =========================================================================
    // Tests para parse_assignment_expr() - Asignaciones
    // =========================================================================

    #[test]
    fn test_parse_simple_assignment() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "x := 5";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression();

        match &expr.kind {
            ExprKind::Assignment { target, value } => {
                // target y value son Box<Expr>, siempre existen
                assert!(matches!(target.kind, ExprKind::Identifier(_)));
                assert!(matches!(value.kind, ExprKind::Literal(Literal::Number(_))));
            }
            _ => panic!("Se esperaba Assignment"),
        }
    }

    #[test]
    fn test_parse_assignment_with_expression() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "x := y + 5";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expression();

        match &expr.kind {
            ExprKind::Assignment { target, value: _ } => {
                // Target debe ser un identificador
                if let ExprKind::Identifier(name) = &target.kind {
                    assert_eq!(name, "x");
                } else {
                    panic!("Target debe ser identificador");
                }
            }
            _ => panic!("Se esperaba Assignment"),
        }
    }

    // =========================================================================
    // Tests para parse_declaration() - Declaraciones de función
    // =========================================================================

    #[test]
    fn test_parse_function_declaration_expression_body() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "function sum(x, y) => x";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let declaration = parser.parse_declaration();

        assert!(
            declaration.is_some(),
            "Debe parsear una declaración de función"
        );

        let declaration = declaration.unwrap();
        match declaration.kind {
            DeclarationKind::Function(function) => {
                assert_eq!(function.name, "sum");
                assert_eq!(function.parameters.len(), 2);
                assert!(function.return_type.is_none());
            }
            _ => panic!("Se esperaba DeclarationKind::Function"),
        }
    }

    #[test]
    fn test_parse_function_declaration_block_body() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "function answer() { 42 }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let declaration = parser.parse_declaration();

        assert!(
            declaration.is_some(),
            "Debe parsear una declaración de función"
        );

        let declaration = declaration.unwrap();
        match declaration.kind {
            DeclarationKind::Function(function) => {
                assert_eq!(function.name, "answer");
                assert_eq!(function.parameters.len(), 0);
                assert!(matches!(function.body.kind, ExprKind::Block(_)));
            }
            _ => panic!("Se esperaba DeclarationKind::Function"),
        }
    }

    #[test]
    fn test_parse_type_declaration_with_inherits_and_members() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "type Point(x: Number, y: Number) inherits BasePoint(x, y) { size: Number = 0; function norm() => x }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let declaration = parser.parse_declaration();

        assert!(
            declaration.is_some(),
            "Debe parsear una declaración de tipo"
        );

        let declaration = declaration.unwrap();
        match declaration.kind {
            DeclarationKind::Type(type_decl) => {
                assert_eq!(type_decl.name, "Point");
                assert_eq!(type_decl.parameters.len(), 2);
                assert!(type_decl.inherits.is_some());
                assert_eq!(type_decl.parent_arguments.len(), 2);
                assert_eq!(type_decl.members.len(), 2);

                match &type_decl.members[0] {
                    TypeMember::Attribute(attribute) => {
                        assert_eq!(attribute.name, "size");
                        assert!(attribute.annotation.is_some());
                    }
                    _ => panic!("Se esperaba un atributo como primer miembro"),
                }

                match &type_decl.members[1] {
                    TypeMember::Method(function) => {
                        assert_eq!(function.name, "norm");
                        assert_eq!(function.parameters.len(), 0);
                    }
                    _ => panic!("Se esperaba un método como segundo miembro"),
                }
            }
            _ => panic!("Se esperaba DeclarationKind::Type"),
        }
    }

    // =========================================================================
    // Tests para type attributes (Tarea #3)
    // =========================================================================

    #[test]
    fn test_parse_type_with_multiple_attributes() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "type Person() {
            name: String = \"John\";
            age: Number = 30;
            salary: Number = 5000
        }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let declaration = parser.parse_declaration();

        assert!(declaration.is_some());

        let declaration = declaration.unwrap();
        match declaration.kind {
            DeclarationKind::Type(type_decl) => {
                assert_eq!(type_decl.name, "Person");
                assert_eq!(type_decl.members.len(), 3);

                // Verificar que todos son atributos
                for (i, member) in type_decl.members.iter().enumerate() {
                    match member {
                        TypeMember::Attribute(attr) => {
                            assert!(attr.annotation.is_some());
                            assert!(!attr.name.is_empty());
                        }
                        _ => panic!("Se esperaba atributo en posición {}", i),
                    }
                }
            }
            _ => panic!("Se esperaba Type"),
        }
    }

    #[test]
    fn test_parse_type_attribute_with_complex_initializer() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "type Circle() {
            radius: Number = 5;
            area: Number = 10
        }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let declaration = parser.parse_declaration();

        assert!(declaration.is_some());

        let declaration = declaration.unwrap();
        match declaration.kind {
            DeclarationKind::Type(type_decl) => {
                assert_eq!(type_decl.name, "Circle");
                assert_eq!(type_decl.members.len(), 2);

                // El segundo atributo tiene un inicializador numérico
                match &type_decl.members[1] {
                    TypeMember::Attribute(attr) => {
                        assert_eq!(attr.name, "area");
                        // El inicializador debe ser una expresión (número literal)
                        match &attr.initializer.kind {
                            ExprKind::Literal(Literal::Number(val)) => {
                                assert_eq!(*val, 10.0);
                            }
                            _ => panic!("Se esperaba un literal numérico para el inicializador"),
                        }
                    }
                    _ => panic!("Se esperaba atributo"),
                }
            }
            _ => panic!("Se esperaba Type"),
        }
    }

    #[test]
    fn test_parse_type_attributes_mixed_with_methods() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "type Rectangle() {
            width: Number = 10;
            height: Number = 20;
            function getArea() => width * height;
            color: String = \"red\"
        }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let declaration = parser.parse_declaration();

        assert!(declaration.is_some());

        let declaration = declaration.unwrap();
        match declaration.kind {
            DeclarationKind::Type(type_decl) => {
                assert_eq!(type_decl.name, "Rectangle");
                assert_eq!(type_decl.members.len(), 4);

                // Primer atributo
                match &type_decl.members[0] {
                    TypeMember::Attribute(attr) => assert_eq!(attr.name, "width"),
                    _ => panic!("Se esperaba atributo en posición 0"),
                }

                // Segundo atributo
                match &type_decl.members[1] {
                    TypeMember::Attribute(attr) => assert_eq!(attr.name, "height"),
                    _ => panic!("Se esperaba atributo en posición 1"),
                }

                // Método
                match &type_decl.members[2] {
                    TypeMember::Method(func) => assert_eq!(func.name, "getArea"),
                    _ => panic!("Se esperaba método en posición 2"),
                }

                // Tercer atributo (después del método)
                match &type_decl.members[3] {
                    TypeMember::Attribute(attr) => assert_eq!(attr.name, "color"),
                    _ => panic!("Se esperaba atributo en posición 3"),
                }
            }
            _ => panic!("Se esperaba Type"),
        }
    }

    // =========================================================================
    // Tests para type methods (Tarea #4)
    // =========================================================================

    #[test]
    fn test_parse_type_method_with_parameters() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "type Calculator() { function add(x, y) => x; function multiply(a, b) => a }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let declaration = parser.parse_declaration();

        assert!(
            declaration.is_some(),
            "Debe parsear una declaración de tipo"
        );

        let declaration = declaration.unwrap();
        match declaration.kind {
            DeclarationKind::Type(type_decl) => {
                assert_eq!(type_decl.name, "Calculator");
                assert_eq!(type_decl.members.len(), 2);

                // Validar primer método
                match &type_decl.members[0] {
                    TypeMember::Method(method) => {
                        assert_eq!(method.name, "add");
                        assert_eq!(method.parameters.len(), 2);
                        assert_eq!(method.parameters[0].name, "x");
                        assert_eq!(method.parameters[1].name, "y");
                    }
                    _ => panic!("Se esperaba método en posición 0"),
                }

                // Validar segundo método
                match &type_decl.members[1] {
                    TypeMember::Method(method) => {
                        assert_eq!(method.name, "multiply");
                        assert_eq!(method.parameters.len(), 2);
                    }
                    _ => panic!("Se esperaba método en posición 1"),
                }
            }
            _ => panic!("Se esperaba DeclarationKind::Type"),
        }
    }

    #[test]
    fn test_parse_type_method_with_return_type() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "type Helper() { function getValue() : Number => 42 }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let declaration = parser.parse_declaration();

        assert!(declaration.is_some());

        let declaration = declaration.unwrap();
        match declaration.kind {
            DeclarationKind::Type(type_decl) => {
                assert_eq!(type_decl.name, "Helper");
                assert_eq!(type_decl.members.len(), 1);

                match &type_decl.members[0] {
                    TypeMember::Method(method) => {
                        assert_eq!(method.name, "getValue");
                        assert!(method.return_type.is_some(), "Debe tener tipo de retorno");
                        let return_type = method.return_type.as_ref().unwrap();
                        assert_eq!(return_type.display_name(), "Number");
                    }
                    _ => panic!("Se esperaba método"),
                }
            }
            _ => panic!("Se esperaba Type"),
        }
    }

    // =========================================================================
    // Tests para inheritance declarations (Tarea #5)
    // =========================================================================

    #[test]
    fn test_parse_type_with_single_inheritance() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "type Dog() inherits Animal() { }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let declaration = parser.parse_declaration();

        assert!(declaration.is_some());

        let declaration = declaration.unwrap();
        match declaration.kind {
            DeclarationKind::Type(type_decl) => {
                assert_eq!(type_decl.name, "Dog");
                assert!(type_decl.inherits.is_some(), "Debe tener un tipo base");
                let parent = type_decl.inherits.as_ref().unwrap();
                assert_eq!(parent.display_name(), "Animal");
                assert_eq!(type_decl.parent_arguments.len(), 0);
            }
            _ => panic!("Se esperaba Type"),
        }
    }

    #[test]
    fn test_parse_type_with_inheritance_and_parent_arguments() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "type Circle(r: Number) inherits Shape(r, r) { }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let declaration = parser.parse_declaration();

        assert!(declaration.is_some());

        let declaration = declaration.unwrap();
        match declaration.kind {
            DeclarationKind::Type(type_decl) => {
                assert_eq!(type_decl.name, "Circle");
                assert!(type_decl.inherits.is_some());
                let parent = type_decl.inherits.as_ref().unwrap();
                assert_eq!(parent.display_name(), "Shape");
                assert_eq!(
                    type_decl.parent_arguments.len(),
                    2,
                    "Debe pasar 2 argumentos al padre"
                );
            }
            _ => panic!("Se esperaba Type"),
        }
    }

    #[test]
    fn test_parse_type_without_inheritance() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let code = "type Standalone() { }";
        let mut lexer = Lexer::new(code.to_string(), "test.hulk".to_string());
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let declaration = parser.parse_declaration();

        assert!(declaration.is_some());

        let declaration = declaration.unwrap();
        match declaration.kind {
            DeclarationKind::Type(type_decl) => {
                assert_eq!(type_decl.name, "Standalone");
                assert!(type_decl.inherits.is_none(), "No debe tener tipo base");
                assert_eq!(type_decl.parent_arguments.len(), 0);
            }
            _ => panic!("Se esperaba Type"),
        }
    }
}
