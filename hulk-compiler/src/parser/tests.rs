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
}
