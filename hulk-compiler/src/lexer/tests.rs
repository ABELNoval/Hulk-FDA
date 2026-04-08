// =============================================================================
// Tests Unitarios - Lexer
// =============================================================================
//
// Tests para el módulo Lexer. Aquí se prueban:
//
// - Tokenización correcta de cada tipo de token
// - Manejo de espacios en blanco y comentarios
// - Posiciones correctas (línea, columna)
// - Errores léxicos (caracteres inválidos, strings sin cerrar)
// - Casos límite (archivos vacíos, tokens muy largos)
// - Números en diferentes formatos
// - Escape sequences en strings
//
// =============================================================================

use crate::lexer::Lexer;
use crate::lexer::token::TokenType;

// =========================================================
// Helper para debug
// =========================================================
fn print_tokens(input: &str) {
    let mut lexer = Lexer::new(input.to_string(), "test.hulk".into());
    let tokens = lexer.tokenize();

    println!("\n================ TOKENS ================");
    println!("Input: {}\n", input);

    for token in tokens {
        println!(
            "Token => type: {:?}, lexeme: '{}', span: [L{}:C{} -> L{}:C{}]",
            token.token_type,
            token.lexeme,
            token.span.start_line,
            token.span.start_column,
            token.span.end_line,
            token.span.end_column
        );
    }

    println!("=======================================\n");
}

// =========================================================
// TEST BÁSICO
// =========================================================
#[test]
fn test_simple_expression() {
    let input = "1 + 2 * 3";
    let mut lexer = Lexer::new(input.into(), "test.hulk".into());
    let tokens = lexer.tokenize();

    assert!(matches!(tokens[0].token_type, TokenType::Number(_)));
    assert_eq!(tokens[1].token_type, TokenType::Plus);
    assert!(matches!(tokens[2].token_type, TokenType::Number(_)));
    assert_eq!(tokens[3].token_type, TokenType::Star);
    assert!(matches!(tokens[4].token_type, TokenType::Number(_)));
}

// =========================================================
// KEYWORDS vs IDENTIFIERS vs BOOLEANS
// =========================================================
#[test]
fn test_keywords_and_identifiers() {
    let input = "let x = if else true false variable";
    print_tokens(input);

    let mut lexer = Lexer::new(input.into(), "test.hulk".into());
    let tokens = lexer.tokenize();

    assert_eq!(tokens[0].token_type, TokenType::Let);
    assert!(matches!(tokens[1].token_type, TokenType::Identifier(_)));
    assert_eq!(tokens[2].token_type, TokenType::Equal);
    assert_eq!(tokens[3].token_type, TokenType::If);
    assert_eq!(tokens[4].token_type, TokenType::Else);
    
    // Verificación de Booleanos
    assert_eq!(tokens[5].token_type, TokenType::True);
    assert_eq!(tokens[6].token_type, TokenType::False);
    
    assert!(matches!(tokens[7].token_type, TokenType::Identifier(_)));
}

// =========================================================
// VALIDACIÓN DE SPANS (Líneas y Columnas)
// =========================================================
#[test]
fn test_spans_and_positions() {
    let input = "let\n  x = /* multi\nlinea */ 10";
    print_tokens(input);

    let mut lexer = Lexer::new(input.into(), "test.hulk".into());
    let tokens = lexer.tokenize();

    // "let" en L1:C1 -> L1:C4
    assert_eq!(tokens[0].span.start_line, 1);
    assert_eq!(tokens[0].span.start_column, 1);

    // "x" en L2:C3 -> L2:C4 (después de \n y 2 espacios)
    assert_eq!(tokens[1].span.start_line, 2);
    assert_eq!(tokens[1].span.start_column, 3);

    // "=" en L2:C5
    assert_eq!(tokens[2].span.start_line, 2);
    assert_eq!(tokens[2].span.start_column, 5);

    // "10" debe estar después del comentario multilínea
    // El comentario termina en la L3, por lo que "10" está en L3
    assert_eq!(tokens[3].span.start_line, 3);
    assert!(tokens[3].span.start_column > 1);
}

// =========================================================
// NÚMEROS (incluye científicos)
// =========================================================
#[test]
fn test_numbers() {
    let input = "10 3.14 1.5e10 2E-3";
    let mut lexer = Lexer::new(input.into(), "test.hulk".into());
    let tokens = lexer.tokenize();

    for token in tokens.iter().take(4) {
        assert!(matches!(token.token_type, TokenType::Number(_)));
    }
}

// =========================================================
// STRINGS + ESCAPES
// =========================================================
#[test]
fn test_strings_with_escapes() {
    let input = r#""hola\nmundo" "test\"ok""#;
    let mut lexer = Lexer::new(input.into(), "test.hulk".into());
    let tokens = lexer.tokenize();

    assert!(matches!(tokens[0].token_type, TokenType::String(_)));
    assert!(matches!(tokens[1].token_type, TokenType::String(_)));
}

// =========================================================
// COMENTARIOS
// =========================================================
#[test]
fn test_comments() {
    let input = r#"
        // comentario
        10 + 20
        /* multi
           linea */
        30
    "#;

    let mut lexer = Lexer::new(input.into(), "test.hulk".into());
    let tokens = lexer.tokenize();

    assert!(tokens.iter().any(|t| matches!(t.token_type, TokenType::Number(_))));
    assert!(tokens.iter().any(|t| t.token_type == TokenType::Plus));
}

// =========================================================
// OPERADORES COMPLEJOS
// =========================================================
#[test]
fn test_complex_operators() {
    let input = "== != <= >= => := @@";
    let mut lexer = Lexer::new(input.into(), "test.hulk".into());
    let tokens = lexer.tokenize();

    assert_eq!(tokens[0].token_type, TokenType::EqualEqual);
    assert_eq!(tokens[1].token_type, TokenType::BangEqual);
    assert_eq!(tokens[2].token_type, TokenType::LessEqual);
    assert_eq!(tokens[3].token_type, TokenType::GreaterEqual);
    assert_eq!(tokens[4].token_type, TokenType::Arrow);
    assert_eq!(tokens[5].token_type, TokenType::ColonEqual);
    assert_eq!(tokens[6].token_type, TokenType::AtAt);
}

// =========================================================
// ERRORES
// =========================================================
#[test]
fn test_invalid_character() {
    let input = "@#";
    let mut lexer = Lexer::new(input.into(), "test.hulk".into());
    let tokens = lexer.tokenize();

    assert_eq!(tokens[0].token_type, TokenType::At);
    assert_eq!(tokens[1].token_type, TokenType::Invalid);
}

#[test]
fn test_unterminated_string() {
    let input = "\"hola";
    let mut lexer = Lexer::new(input.into(), "test.hulk".into());
    let tokens = lexer.tokenize();

    assert_eq!(tokens[0].token_type, TokenType::Invalid);
}

#[test]
fn test_invalid_scientific_notation() {
    let input = "1.5e";
    let mut lexer = Lexer::new(input.into(), "test.hulk".into());
    let tokens = lexer.tokenize();

    assert_eq!(tokens[0].token_type, TokenType::Invalid);
}

// =========================================================
// CASO LÍMITE
// =========================================================
#[test]
fn test_empty_input() {
    let input = "";
    let mut lexer = Lexer::new(input.into(), "test.hulk".into());
    let tokens = lexer.tokenize();

    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_type, TokenType::Eof);
}