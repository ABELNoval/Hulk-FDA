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
use crate::utils::errors::DisplayError;
use crate::utils::errors::ParserError;

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
        if self.cursor.check(&TokenType::Let) {
            self.parse_let_binding()
        } else if self.cursor.check(&TokenType::If) {
            self.parse_if_expr()
        } else if self.cursor.check(&TokenType::While) {
            self.parse_while_expr()
        } else if self.cursor.check(&TokenType::For) {
            self.parse_for_expr()
        } else if self.cursor.check(&TokenType::LeftBrace) {
            self.parse_block()
        } else {
            self.parse_assignment_expr()
        }
    }

    fn parse_block(&mut self) -> Expr {
        let start_token = self.expect(TokenType::LeftBrace);

        if let Err(_) = start_token {
            return Expr::literal(Literal::Number(0.0), self.cursor.peek().span.clone());
        }

        let start_span = start_token.unwrap().span;
        let mut expressions = Vec::new();

        while !self.cursor.check(&TokenType::RightBrace) && !self.cursor.is_at_end() {
            let expr = self.parse_expression();
            expressions.push(expr);
            self.cursor.match_token(&TokenType::Semicolon);
        }

        if let Err(_) = self.expect(TokenType::RightBrace) {
            self.synchronize();
        }

        let end_span = self.cursor.peek().span.clone();
        let span = start_span.merge(&end_span);
        Expr::block(expressions, span)
    }

    fn parse_let_binding(&mut self) -> Expr {
        let start_token = self.expect(TokenType::Let);

        if let Err(_) = start_token {
            return Expr::literal(Literal::Number(0.0), self.cursor.peek().span.clone());
        }

        let start_span = start_token.unwrap().span;
        let mut let_exprs = Vec::new();

        if let Some(expr) = self.parse_single_let_binding() {
            let_exprs.push(expr);
        }

        while self.cursor.match_token(&TokenType::Semicolon) {
            if let Some(expr) = self.parse_single_let_binding() {
                let_exprs.push(expr);
            } else {
                break;
            }
        }

        let end_span = self.cursor.peek().span.clone();

        if let Some(expr) = let_exprs.first().cloned() {
            if let_exprs.len() == 1 {
                let span = start_span.merge(&expr.span);
                return Expr::new(expr.kind, span);
            }
        }

        if !let_exprs.is_empty() {
            let span = start_span.merge(&end_span);
            return Expr::block(let_exprs, span);
        }

        Expr::literal(Literal::Number(0.0), start_span.merge(&end_span))
    }

    fn parse_single_let_binding(&mut self) -> Option<Expr> {
        let name_token = self.cursor.peek().clone();

        if !matches!(name_token.token_type, TokenType::Identifier(_)) {
            self.error(ParserError::ExpectedVariableName);
            return None;
        }

        let name = match &name_token.token_type {
            TokenType::Identifier(n) => n.clone(),
            _ => return None,
        };

        self.cursor.advance();
        let start_span = name_token.span.clone();

        let annotation = if self.cursor.match_token(&TokenType::Colon) {
            Some(self.parse_type_reference())
        } else {
            None
        };

        if let Err(_) = self.expect(TokenType::Equal) {
            self.synchronize();
            return None;
        }

        let value = self.parse_expression();
        let end_span = value.span.clone();
        let span = start_span.merge(&end_span);

        Some(Expr::let_expr(name, annotation, Some(value), span))
    }

    fn parse_if_expr(&mut self) -> Expr {
        let start_token = self.expect(TokenType::If);

        if let Err(_) = start_token {
            return Expr::literal(Literal::Number(0.0), self.cursor.peek().span.clone());
        }

        let start_span = start_token.unwrap().span;

        if let Err(_) = self.expect(TokenType::LeftParen) {
            self.synchronize();
            return Expr::literal(Literal::Number(0.0), start_span);
        }

        let condition = self.parse_expression();

        if let Err(_) = self.expect(TokenType::RightParen) {
            self.synchronize();
        }

        let then_expr = self.parse_expression();
        let (elif_parts, else_expr) = self.parse_elif_parts();

        let end_span = else_expr
            .as_ref()
            .map(|e| e.span.clone())
            .or_else(|| elif_parts.last().map(|(_, e)| e.span.clone()))
            .unwrap_or_else(|| then_expr.span.clone());

        let span = start_span.merge(&end_span);
        Expr::if_expr(condition, then_expr, elif_parts, else_expr, span)
    }

    fn parse_elif_parts(&mut self) -> (Vec<(Expr, Expr)>, Option<Expr>) {
        let mut elif_parts = Vec::new();

        while self.cursor.match_token(&TokenType::Elif) {
            if let Err(_) = self.expect(TokenType::LeftParen) {
                self.synchronize();
                break;
            }

            let condition = self.parse_expression();

            if let Err(_) = self.expect(TokenType::RightParen) {
                self.synchronize();
            }

            let elif_expr = self.parse_expression();
            elif_parts.push((condition, elif_expr));
        }

        let else_expr = if self.cursor.match_token(&TokenType::Else) {
            Some(self.parse_expression())
        } else {
            None
        };

        (elif_parts, else_expr)
    }

    fn parse_while_expr(&mut self) -> Expr {
        let start_token = self.expect(TokenType::While);

        if let Err(_) = start_token {
            return Expr::literal(Literal::Number(0.0), self.cursor.peek().span.clone());
        }

        let start_span = start_token.unwrap().span;

        if let Err(_) = self.expect(TokenType::LeftParen) {
            self.synchronize();
            return Expr::literal(Literal::Number(0.0), start_span);
        }

        let condition = self.parse_expression();

        if let Err(_) = self.expect(TokenType::RightParen) {
            self.synchronize();
        }

        let body = self.parse_expression();
        let end_span = body.span.clone();
        let span = start_span.merge(&end_span);
        Expr::while_expr(condition, body, span)
    }

    fn parse_for_expr(&mut self) -> Expr {
        let start_token = self.expect(TokenType::For);

        if let Err(_) = start_token {
            return Expr::literal(Literal::Number(0.0), self.cursor.peek().span.clone());
        }

        let start_span = start_token.unwrap().span;
        let var_token = self.cursor.peek().clone();

        if !matches!(var_token.token_type, TokenType::Identifier(_)) {
            self.error(ParserError::ExpectedIdentifier {
                found: var_token.lexeme.clone(),
            });
            self.synchronize();
            return Expr::literal(Literal::Number(0.0), start_span);
        }

        let variable = match &var_token.token_type {
            TokenType::Identifier(v) => v.clone(),
            _ => return Expr::literal(Literal::Number(0.0), start_span),
        };

        self.cursor.advance();

        if let Err(_) = self.expect(TokenType::In) {
            self.synchronize();
            return Expr::literal(Literal::Number(0.0), start_span);
        }

        let iterable = self.parse_expression();
        let body = self.parse_expression();
        let end_span = body.span.clone();
        let span = start_span.merge(&end_span);
        Expr::for_expr(variable, iterable, body, span)
    }

    fn parse_assignment_expr(&mut self) -> Expr {
        let checkpoint = self.cursor.current_position();

        if let TokenType::Identifier(_) = &self.cursor.peek().token_type {
            let identifier_token = self.cursor.advance().clone();

            if self.cursor.check(&TokenType::ColonEqual) {
                self.cursor.advance();

                let target = Expr::identifier(
                    match &identifier_token.token_type {
                        TokenType::Identifier(n) => n.clone(),
                        _ => String::new(),
                    },
                    identifier_token.span.clone(),
                );

                let value = self.parse_expression();
                let span = target.span.merge(&value.span);
                return Expr::assignment(target, value, span);
            }

            self.cursor.set_position(checkpoint);
        }

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
        let mut expr = self.parse_concatenation();

        while self.cursor.check_any(&[
            TokenType::Less,
            TokenType::LessEqual,
            TokenType::Greater,
            TokenType::GreaterEqual,
        ]) {
            let operator = self.cursor.advance();
            let right = self.parse_concatenation();
            let span = expr.span.merge(&right.span);
            let op = BinaryOperator::from_token_type(&operator.token_type).unwrap();
            expr = Expr::binary(expr, op, right, span);
        }

        expr
    }

    fn parse_concatenation(&mut self) -> Expr {
        let mut expr = self.parse_term();

        while self.cursor.check_any(&[TokenType::At, TokenType::AtAt]) {
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

        self.parse_call()
    }

    fn parse_call(&mut self) -> Expr {
        let mut expr = self.parse_primary();

        loop {
            if self.cursor.check(&TokenType::LeftParen) {
                self.cursor.advance();
                let mut arguments = Vec::new();

                if !self.cursor.check(&TokenType::RightParen) {
                    loop {
                        arguments.push(self.parse_expression());
                        if self.cursor.check(&TokenType::Comma) {
                            self.cursor.advance();
                        } else {
                            break;
                        }
                    }
                }

                if self.cursor.check(&TokenType::RightParen) {
                    let close_span = self.cursor.advance().span;
                    let span = expr.span.merge(&close_span);
                    expr = Expr::call(expr, arguments, span);
                } else {
                    let span = if let Some(last_argument) = arguments.last() {
                        expr.span.merge(&last_argument.span)
                    } else {
                        expr.span.merge(&expr.span)
                    };
                    expr = Expr::call(expr, arguments, span);
                    break;
                }
            } else {
                break;
            }
        }

        expr
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

    fn error(&mut self, error: ParserError) {
        eprintln!("Parser error[{}]: {}", error.code(), error.message());
        if let Some(help) = error.help() {
            eprintln!("  = ayuda: {}", help);
        }
    }

    fn expect(&mut self, token_type: TokenType) -> Result<Token, ParserError> {
        if self.cursor.check(&token_type) {
            Ok(self.cursor.advance())
        } else {
            let found = self.cursor.peek().lexeme.clone();
            let expected = match &token_type {
                TokenType::LeftBrace => "{",
                TokenType::RightBrace => "}",
                TokenType::LeftParen => "(",
                TokenType::RightParen => ")",
                TokenType::LeftBracket => "[",
                TokenType::RightBracket => "]",
                TokenType::Semicolon => ";",
                TokenType::Comma => ",",
                TokenType::Colon => ":",
                TokenType::Equal => "=",
                TokenType::ColonEqual => ":=",
                TokenType::Arrow => "=>",
                TokenType::Let => "let",
                TokenType::If => "if",
                TokenType::Elif => "elif",
                TokenType::Else => "else",
                TokenType::While => "while",
                TokenType::For => "for",
                TokenType::In => "in",
                _ => &format!("{:?}", token_type),
            };

            let error = ParserError::UnexpectedToken {
                expected: expected.to_string(),
                found,
            };
            self.error(error.clone());
            Err(error)
        }
    }

    fn synchronize(&mut self) {
        self.cursor.advance();

        while !self.cursor.is_at_end() {
            if self.cursor.peek().token_type == TokenType::Semicolon {
                self.cursor.advance();
                return;
            }

            match self.cursor.peek().token_type {
                TokenType::Let
                | TokenType::Function
                | TokenType::Type
                | TokenType::Protocol
                | TokenType::If
                | TokenType::While
                | TokenType::For
                | TokenType::RightBrace => {
                    return;
                }
                _ => {
                    self.cursor.advance();
                }
            }
        }
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
