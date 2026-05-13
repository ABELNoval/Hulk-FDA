// =============================================================================
// Parser (Analizador Sintáctico)
// =============================================================================
//
// El parser es la segunda fase del compilador. Transforma la secuencia de
// tokens producida por el lexer en un Árbol de Sintaxis Abstracta (AST).
//
// Responsabilidades:
// - Verificar que los tokens sigan la gramática del lenguaje
// - Construir el AST que representa la estructura del programa
// - Manejar precedencia y asociatividad de operadores
// - Reportar errores sintácticos con mensajes claros
// - Implementar recuperación de errores para continuar parseando
//
// Técnicas comunes:
// - Recursive Descent Parsing (descendente recursivo)
// - Pratt Parsing para expresiones (precedencia de operadores)
// - LL(k) o LR parsing
//
// El AST resultante contiene nodos como:
// - Declaraciones de funciones
// - Declaraciones de variables
// - Expresiones (binarias, unarias, llamadas, etc.)
// - Statements (if, while, return, etc.)
// - Tipos y anotaciones de tipo
//
// =============================================================================

use crate::lexer::Token;
use crate::lexer::TokenType;

pub mod ast;
pub use ast::{
    AttributeDeclaration, BinaryOperator, Declaration, DeclarationKind, Expr, ExprKind,
    FunctionDeclaration, Literal, Parameter, Program, ProtocolDeclaration, ProtocolMethodSignature,
    TypeDeclaration, TypeMember, TypeReference, TypeReferenceKind, UnaryOperator,
    VariableDeclaration,
};

// =============================================================================
// Parser - API común de alto nivel
// =============================================================================

#[derive(Debug, Clone)]
pub struct Parser {
    cursor: TokenCursor,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            cursor: TokenCursor::new(tokens),
        }
    }

    pub fn parse_program(&mut self) -> Program {
        unimplemented!()
    }

    pub fn parse_declaration(&mut self) -> Option<Declaration> {
        unimplemented!()
    }

    pub fn parse_expression(&mut self) -> Expr {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Expr {
        let mut expr = self.parse_logical_and();

        while self.cursor.check(&TokenType::Pipe) {
            let operator = self.cursor.advance();
            let right = self.parse_logical_and();
            let span = expr.span.merge(&right.span);
            let op = BinaryOperator::from_token_type(&operator.token_type).unwrap();
            expr = Expr::binary(expr, op, right, span);
        }

        expr
    }

    fn parse_logical_and(&mut self) -> Expr {
        let mut expr = self.parse_equality();

        while self.cursor.check(&TokenType::Ampersand) {
            let operator = self.cursor.advance();
            let right = self.parse_equality();
            let span = expr.span.merge(&right.span);
            let op = BinaryOperator::from_token_type(&operator.token_type).unwrap();
            expr = Expr::binary(expr, op, right, span);
        }

        expr
    }

    fn parse_equality(&mut self) -> Expr {
        let mut expr = self.parse_comparison();

        while self.cursor.check_any(&[TokenType::EqualEqual, TokenType::BangEqual]) {
            let operator = self.cursor.advance();
            let right = self.parse_comparison();
            let span = expr.span.merge(&right.span);
            let op = BinaryOperator::from_token_type(&operator.token_type).unwrap();
            expr = Expr::binary(expr, op, right, span);
        }

        expr
    }

    fn parse_comparison(&mut self) -> Expr {
        let mut expr = self.parse_term();

        while self.cursor.check_any(&[
            TokenType::Less,
            TokenType::LessEqual,
            TokenType::Greater,
            TokenType::GreaterEqual,
        ]) {
            let operator = self.cursor.advance();
            let right = self.parse_term();
            let span = expr.span.merge(&right.span);
            let op = BinaryOperator::from_token_type(&operator.token_type).unwrap();
            expr = Expr::binary(expr, op, right, span);
        }

        expr
    }

    fn parse_term(&mut self) -> Expr {
        let mut expr = self.parse_factor();

        while self.cursor.check_any(&[TokenType::Plus, TokenType::Minus]) {
            let operator = self.cursor.advance();
            let right = self.parse_factor();
            let span = expr.span.merge(&right.span);
            let op = BinaryOperator::from_token_type(&operator.token_type).unwrap();
            expr = Expr::binary(expr, op, right, span);
        }

        expr
    }

    fn parse_factor(&mut self) -> Expr {
        let mut expr = self.parse_power();

        while self.cursor.check_any(&[TokenType::Star, TokenType::Slash, TokenType::Percent]) {
            let operator = self.cursor.advance();
            let right = self.parse_power();
            let span = expr.span.merge(&right.span);
            let op = BinaryOperator::from_token_type(&operator.token_type).unwrap();
            expr = Expr::binary(expr, op, right, span);
        }

        expr
    }

    fn parse_power(&mut self) -> Expr {
        let expr = self.parse_unary();

        if self.cursor.check(&TokenType::Caret) {
            let operator = self.cursor.advance();
            let right = self.parse_power();
            let span = expr.span.merge(&right.span);
            let op = BinaryOperator::from_token_type(&operator.token_type).unwrap();
            return Expr::binary(expr, op, right, span);
        }

        expr
    }

    fn parse_unary(&mut self) -> Expr {
        if self.cursor.check_any(&[TokenType::Plus, TokenType::Minus, TokenType::Bang]) {
            let operator = self.cursor.advance();
            let operand = self.parse_unary();
            let span = operator.span.merge(&operand.span);
            let op = UnaryOperator::from_token_type(&operator.token_type).unwrap();
            return Expr::unary(op, operand, span);
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Expr {
        if self.cursor.check(&TokenType::LeftParen) {
            self.cursor.advance();
            let expr = self.parse_expression();
            if self.cursor.check(&TokenType::RightParen) {
                self.cursor.advance();
            } else {
                panic!("Expected ')' after expression");
            }
            return expr;
        }

        let token = self.cursor.advance();
        let span = token.span.clone();

        match token.token_type {
            TokenType::Number(value) => Expr::literal(Literal::Number(value), span),
            TokenType::String(value) => Expr::literal(Literal::String(value), span),
            TokenType::True => Expr::literal(Literal::Boolean(true), span),
            TokenType::False => Expr::literal(Literal::Boolean(false), span),
            TokenType::Pi => Expr::literal(Literal::Pi, span),
            TokenType::E => Expr::literal(Literal::E, span),
            TokenType::Identifier(name) => Expr::identifier(name, span),
            _ => unimplemented!("Unexpected token in expression: {:?}", token),
        }
    }

    pub fn parse_type_reference(&mut self) -> TypeReference {
        unimplemented!()
    }

    pub fn cursor(&self) -> &TokenCursor {
        &self.cursor
    }

    pub fn cursor_mut(&mut self) -> &mut TokenCursor {
        &mut self.cursor
    }
}

// =============================================================================
// TokenCursor - Sistema de navegación del flujo de tokens
// =============================================================================
//
// El TokenCursor es responsable de navegar a través de la secuencia de tokens
// producida por el lexer. Proporciona operaciones como:
// - peek: mirar el token actual sin avanzar
// - advance: pasar al siguiente token
// - consume: consumir un token esperado
// - conditional matching: verificaciones condicionales de tokens
//
// =============================================================================

#[derive(Debug, Clone)]
pub struct TokenCursor {
    /// Vector de tokens del lexer
    tokens: Vec<Token>,
    /// Posición actual en el flujo de tokens
    position: usize,
}

impl TokenCursor {
    /// Crea un nuevo cursor de tokens a partir de una lista de tokens.
    ///
    /// # Argumentos
    /// * `tokens` - Vector de tokens del lexer
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    // =========================================================================
    // Operaciones básicas de navegación
    // =========================================================================

    /// Obtiene el token actual sin avanzar la posición (peek).
    ///
    /// # Retorna
    /// Una referencia al token actual, o una referencia a un token EOF si
    /// hemos llegado al final del flujo.
    pub fn peek(&self) -> &Token {
        self.tokens
            .get(self.position)
            .unwrap_or_else(|| &self.tokens[self.tokens.len() - 1])
    }

    /// Avanza a la posición del siguiente token (advance).
    ///
    /// # Retorna
    /// Una copia del token que acaba de ser consumido.
    pub fn advance(&mut self) -> Token {
        let token = self.peek().clone();
        if self.position < self.tokens.len() - 1 {
            self.position += 1;
        }
        token
    }

    /// Consume un token del tipo especificado.
    ///
    /// Si el token actual coincide con el tipo esperado, lo consume y
    /// retorna una copia del token. De lo contrario, retorna None.
    ///
    /// # Argumentos
    /// * `expected_type` - El tipo de token que se espera consumir
    ///
    /// # Retorna
    /// `Some(token)` si el token actual coincide con el tipo esperado,
    /// `None` en caso contrario.
    pub fn consume(&mut self, expected_type: &TokenType) -> Option<Token> {
        if std::mem::discriminant(&self.peek().token_type) == std::mem::discriminant(expected_type)
        {
            let token = self.peek().clone();
            self.advance();
            Some(token)
        } else {
            None
        }
    }

    // =========================================================================
    // Operaciones de coincidencia condicional
    // =========================================================================

    /// Verifica si el token actual es del tipo especificado sin avanzar.
    ///
    /// # Argumentos
    /// * `token_type` - El tipo de token a verificar
    ///
    /// # Retorna
    /// `true` si el token actual coincide con el tipo especificado, `false` en caso contrario.
    pub fn check(&self, token_type: &TokenType) -> bool {
        std::mem::discriminant(&self.peek().token_type) == std::mem::discriminant(token_type)
    }

    /// Verifica si el token actual coincide con alguno de los tipos especificados.
    ///
    /// # Argumentos
    /// * `token_types` - Slice de tipos de token a verificar
    ///
    /// # Retorna
    /// `true` si el token actual coincide con alguno de los tipos, `false` en caso contrario.
    pub fn check_any(&self, token_types: &[TokenType]) -> bool {
        token_types.iter().any(|t| self.check(t))
    }

    /// Verifica si el token actual coincide y lo consume si es así.
    ///
    /// # Argumentos
    /// * `token_type` - El tipo de token a verificar y consumir
    ///
    /// # Retorna
    /// `true` si el token fue consumido, `false` en caso contrario.
    pub fn match_token(&mut self, token_type: &TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Verifica si el token actual coincide con alguno de los tipos y lo consume.
    ///
    /// # Argumentos
    /// * `token_types` - Slice de tipos de token a verificar
    ///
    /// # Retorna
    /// `true` si el token fue consumido, `false` en caso contrario.
    pub fn match_any(&mut self, token_types: &[TokenType]) -> bool {
        if self.check_any(token_types) {
            self.advance();
            true
        } else {
            false
        }
    }

    // =========================================================================
    // Utilidades
    // =========================================================================

    /// Obtiene la posición actual en el flujo de tokens.
    pub fn current_position(&self) -> usize {
        self.position
    }

    /// Establece la posición a un valor específico.
    ///
    /// # Argumentos
    /// * `position` - La nueva posición
    pub fn set_position(&mut self, position: usize) {
        self.position = position.min(self.tokens.len() - 1);
    }

    /// Retrocede una posición (util para recuperación de errores).
    pub fn backtrack(&mut self) {
        if self.position > 0 {
            self.position -= 1;
        }
    }

    /// Verifica si hemos llegado al final del flujo de tokens.
    pub fn is_at_end(&self) -> bool {
        self.peek().token_type == TokenType::Eof
    }

    /// Obtiene el número total de tokens.
    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
