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
use crate::utils::errors::ParserError;
use crate::utils::errors::DisplayError;

pub mod ast;
pub use ast::{Declaration, Expr, ExprKind, Literal, Program, TypeReference};

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

    pub fn parse_type_reference(&mut self) -> TypeReference {
        unimplemented!()
    }

    pub fn cursor(&self) -> &TokenCursor {
        &self.cursor
    }

    pub fn cursor_mut(&mut self) -> &mut TokenCursor {
        &mut self.cursor
    }

    // =========================================================================
    // Métodos privados - Parsing de Expresiones Estructuradas (Tarea #28)
    // =========================================================================

    /// Parse una expresión en el nivel más alto.
    /// 
    /// Intenta parsear en este orden (TOP-DOWN):
    /// 1. Let expressions (let x = 5)
    /// 2. If expressions (if ...)
    /// 3. While loops (while ...)
    /// 4. For loops (for x in ...)
    /// 5. Assignment expressions (x := 5)
    /// 6. Block expressions ({ ... })
    /// 7. Si nada coincide, parsea una expresión primaria (otra persona)
    pub fn parse_expression(&mut self) -> Expr {
        // TOP-DOWN: intentamos cada patrón de mayor a menor jerarquía
        
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
            // Intenta parsear assignment o delega a parse_primary (caja negra)
            self.parse_assignment_expr()
        }
    }

    /// Parse un bloque de código: { expr1; expr2; ... }
    /// 
    /// Gramática:
    /// ```
    /// block = "{" expression* "}"
    /// ```
    /// 
    /// Ejemplos:
    /// - `{ 5 + 3 }`
    /// - `{ let x = 5; x + 1 }`
    /// - `{ }`
    fn parse_block(&mut self) -> Expr {
        let start_token = self.expect(TokenType::LeftBrace);
        
        if let Err(_) = start_token {
            // Si no hay "{", recuperamos y retornamos error
            return Expr::new(
                ExprKind::Literal(Literal::Number(0.0)),
                self.cursor.peek().span.clone(),
            );
        }

        let start_span = start_token.unwrap().span;
        let mut expressions = Vec::new();

        // Parsea expresiones hasta encontrar "}"
        while !self.cursor.check(&TokenType::RightBrace) && !self.cursor.is_at_end() {
            let expr = self.parse_expression();
            expressions.push(expr);

            // Puede haber punto y coma opcional entre expresiones
            self.cursor.match_token(&TokenType::Semicolon);
        }

        // Esperamos el "}"
        if let Err(_) = self.expect(TokenType::RightBrace) {
            self.synchronize();
        }

        let end_span = self.cursor.peek().span.clone();
        let span = start_span.merge(&end_span);

        Expr::new(ExprKind::Block(expressions), span)
    }

    /// Parse una expresión let con posible múltiples bindings.
    /// 
    /// Gramática (simplificada):
    /// ```
    /// let_expr = "let" binding (";" binding)* "in" expression
    /// binding = identifier [":" type_ref] "=" expression
    /// ```
    /// 
    /// Ejemplos:
    /// - `let x = 5 in x + 1`
    /// - `let x = 5; y = 3 in x + y`
    /// - `let x: Number = 5 in x`
    fn parse_let_binding(&mut self) -> Expr {
        let start_token = self.expect(TokenType::Let);
        
        if let Err(_) = start_token {
            return Expr::new(
                ExprKind::Literal(Literal::Number(0.0)),
                self.cursor.peek().span.clone(),
            );
        }

        let start_span = start_token.unwrap().span;
        let mut let_exprs = Vec::new();

        // Parsea el primer binding
        if let Some(expr) = self.parse_single_let_binding() {
            let_exprs.push(expr);
        }

        // Parsea bindings adicionales si hay ";"
        while self.cursor.match_token(&TokenType::Semicolon) {
            if let Some(expr) = self.parse_single_let_binding() {
                let_exprs.push(expr);
            } else {
                break;
            }
        }

        let end_span = self.cursor.peek().span.clone();

        // Si solo hay un binding, lo retornamos directamente
        if let_exprs.len() == 1 {
            let expr = let_exprs.into_iter().next().unwrap();
            let span = start_span.merge(&expr.span);
            Expr::new(expr.kind, span)
        } else if let_exprs.len() > 1 {
            // Si hay múltiples, los envolvemos en un bloque de let
            let span = start_span.merge(&end_span);
            Expr::new(ExprKind::Block(let_exprs), span)
        } else {
            // Error: no hubo ningún binding válido
            Expr::new(
                ExprKind::Literal(Literal::Number(0.0)),
                start_span.merge(&end_span),
            )
        }
    }

    /// Helper para parsear un único binding de let
    /// 
    /// Formato: `identifier [":" type_ref] "=" expression`
    fn parse_single_let_binding(&mut self) -> Option<Expr> {
        // Esperamos un identificador
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

        // Tipo opcional: ": Type"
        let annotation = if self.cursor.match_token(&TokenType::Colon) {
            Some(self.parse_type_reference())
        } else {
            None
        };

        // Esperamos "="
        if let Err(_) = self.expect(TokenType::Equal) {
            self.synchronize();
            return None;
        }

        // Parsea la expresión del lado derecho
        let value = self.parse_expression();
        let end_span = value.span.clone();
        let span = start_span.merge(&end_span);

        Some(Expr::new(
            ExprKind::Let {
                name,
                annotation,
                value: Some(Box::new(value)),
            },
            span,
        ))
    }

    /// Parse una expresión if/elif/else.
    /// 
    /// Gramática:
    /// ```
    /// if_expr = "if" "(" condition ")" expr 
    ///           ("elif" "(" condition ")" expr)*
    ///           ("else" expr)?
    /// ```
    /// 
    /// Ejemplo: `if (x > 5) { ... } elif (x == 5) { ... } else { ... }`
    fn parse_if_expr(&mut self) -> Expr {
        let start_token = self.expect(TokenType::If);
        
        if let Err(_) = start_token {
            return Expr::new(
                ExprKind::Literal(Literal::Number(0.0)),
                self.cursor.peek().span.clone(),
            );
        }

        let start_span = start_token.unwrap().span;

        // Esperamos "("
        if let Err(_) = self.expect(TokenType::LeftParen) {
            self.synchronize();
            return Expr::new(
                ExprKind::Literal(Literal::Number(0.0)),
                start_span,
            );
        }

        // Parsea la condición
        let condition = self.parse_expression();

        // Esperamos ")"
        if let Err(_) = self.expect(TokenType::RightParen) {
            self.synchronize();
        }

        // Parsea la expresión entonces
        let then_expr = self.parse_expression();

        // Parsea elif/else
        let (elif_parts, else_expr) = self.parse_elif_parts();

        let end_span = else_expr
            .as_ref()
            .map(|e| e.span.clone())
            .or_else(|| {
                elif_parts
                    .last()
                    .map(|(_, e)| e.span.clone())
            })
            .unwrap_or_else(|| then_expr.span.clone());

        let span = start_span.merge(&end_span);

        Expr::new(
            ExprKind::If {
                condition: Box::new(condition),
                then_expr: Box::new(then_expr),
                elif_parts,
                else_expr: else_expr.map(Box::new),
            },
            span,
        )
    }

    /// Helper para parsear elif y else en un if
    /// 
    /// Retorna: (vec de (condición, expresión) para elifs, expresión else opcional)
    fn parse_elif_parts(&mut self) -> (Vec<(Expr, Expr)>, Option<Expr>) {
        let mut elif_parts = Vec::new();

        // Parsea elif clauses
        while self.cursor.match_token(&TokenType::Elif) {
            // Esperamos "("
            if let Err(_) = self.expect(TokenType::LeftParen) {
                self.synchronize();
                break;
            }

            let condition = self.parse_expression();

            // Esperamos ")"
            if let Err(_) = self.expect(TokenType::RightParen) {
                self.synchronize();
            }

            let elif_expr = self.parse_expression();
            elif_parts.push((condition, elif_expr));
        }

        // Parsea else clause
        let else_expr = if self.cursor.match_token(&TokenType::Else) {
            Some(self.parse_expression())
        } else {
            None
        };

        (elif_parts, else_expr)
    }

    /// Parse un while loop.
    /// 
    /// Gramática:
    /// ```
    /// while_expr = "while" "(" condition ")" body_expr
    /// ```
    /// 
    /// Ejemplo: `while (x > 0) { x := x - 1 }`
    fn parse_while_expr(&mut self) -> Expr {
        let start_token = self.expect(TokenType::While);
        
        if let Err(_) = start_token {
            return Expr::new(
                ExprKind::Literal(Literal::Number(0.0)),
                self.cursor.peek().span.clone(),
            );
        }

        let start_span = start_token.unwrap().span;

        // Esperamos "("
        if let Err(_) = self.expect(TokenType::LeftParen) {
            self.synchronize();
            return Expr::new(
                ExprKind::Literal(Literal::Number(0.0)),
                start_span,
            );
        }

        // Parsea la condición
        let condition = self.parse_expression();

        // Esperamos ")"
        if let Err(_) = self.expect(TokenType::RightParen) {
            self.synchronize();
        }

        // Parsea el cuerpo
        let body = self.parse_expression();
        let end_span = body.span.clone();
        let span = start_span.merge(&end_span);

        Expr::new(
            ExprKind::While {
                condition: Box::new(condition),
                body: Box::new(body),
            },
            span,
        )
    }

    /// Parse un for loop.
    /// 
    /// Gramática:
    /// ```
    /// for_expr = "for" identifier "in" iterable_expr body_expr
    /// ```
    /// 
    /// Ejemplo: `for i in range(1, 10) { print(i) }`
    fn parse_for_expr(&mut self) -> Expr {
        let start_token = self.expect(TokenType::For);
        
        if let Err(_) = start_token {
            return Expr::new(
                ExprKind::Literal(Literal::Number(0.0)),
                self.cursor.peek().span.clone(),
            );
        }

        let start_span = start_token.unwrap().span;

        // Esperamos identificador (variable de iteración)
        let var_token = self.cursor.peek().clone();
        
        if !matches!(var_token.token_type, TokenType::Identifier(_)) {
            self.error(ParserError::ExpectedIdentifier {
                found: var_token.lexeme.clone(),
            });
            self.synchronize();
            return Expr::new(
                ExprKind::Literal(Literal::Number(0.0)),
                start_span,
            );
        }

        let variable = match &var_token.token_type {
            TokenType::Identifier(v) => v.clone(),
            _ => return Expr::new(
                ExprKind::Literal(Literal::Number(0.0)),
                start_span,
            ),
        };

        self.cursor.advance();

        // Esperamos "in"
        if let Err(_) = self.expect(TokenType::In) {
            self.synchronize();
            return Expr::new(
                ExprKind::Literal(Literal::Number(0.0)),
                start_span,
            );
        }

        // Parsea la expresión iterable
        let iterable = self.parse_expression();

        // Parsea el cuerpo
        let body = self.parse_expression();
        let end_span = body.span.clone();
        let span = start_span.merge(&end_span);

        Expr::new(
            ExprKind::For {
                variable,
                iterable: Box::new(iterable),
                body: Box::new(body),
            },
            span,
        )
    }

    /// Parse una expresión de asignación o delega a parse_primary.
    /// 
    /// Intenta detectar: `identifier := value`
    /// Si no hay `:=`, delega a otra persona (parse_primary)
    fn parse_assignment_expr(&mut self) -> Expr {
        // Mira adelante para ver si es una asignación
        // Patrón: identifier := expr
        
        let checkpoint = self.cursor.current_position();
        
        // Intenta obtener un identificador
        if let TokenType::Identifier(_) = &self.cursor.peek().token_type {
            let identifier_token = self.cursor.advance().clone();
            
            // Verificamos si hay ":="
            if self.cursor.check(&TokenType::ColonEqual) {
                self.cursor.advance(); // Consumir ":="
                
                let target = Expr::identifier(
                    match &identifier_token.token_type {
                        TokenType::Identifier(n) => n.clone(),
                        _ => String::new(),
                    },
                    identifier_token.span.clone(),
                );
                
                let value = self.parse_expression();
                let span = target.span.merge(&value.span);
                
                return Expr::new(
                    ExprKind::Assignment {
                        target: Box::new(target),
                        value: Box::new(value),
                    },
                    span,
                );
            } else {
                // No es asignación, retrocedemos
                self.cursor.set_position(checkpoint);
            }
        }

        // No es asignación, delega a parse_primary (caja negra)
        self.parse_primary()
    }

    /// Parsea una expresión primaria (otro método stub que usamos como caja negra)
    fn parse_primary(&mut self) -> Expr {
        // Esto es un método stub que otra persona implementa
        // Por ahora retornamos un placeholder
        let token = self.cursor.peek().clone();
        self.cursor.advance();
        
        Expr::new(
            ExprKind::Literal(Literal::Number(0.0)),
            token.span,
        )
    }

    // =========================================================================
    // Utilidades internas - Helpers para parsing
    // =========================================================================

    /// Reporta un error de parsing.
    /// 
    /// En una versión más completa, los errores se acumularían en un vector
    /// para reportarlos todos al final. Por ahora los reportamos inmediatamente.
    fn error(&mut self, error: ParserError) {
        eprintln!("Parser error[{}]: {}", error.code(), error.message());
        if let Some(help) = error.help() {
            eprintln!("  = ayuda: {}", help);
        }
    }

    /// Intenta consumir un token del tipo especificado.
    /// 
    /// Si el token actual coincide con el tipo esperado, lo consume y retorna Ok.
    /// Si no coincide, reporta un error y retorna Err.
    /// 
    /// # Argumentos
    /// * `token_type` - El tipo de token que se espera
    /// 
    /// # Retorna
    /// `Ok(token)` si el token coincide, `Err(ParserError)` si no
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

    /// Realiza recuperación de errores (error recovery).
    /// 
    /// Avanza tokens hasta encontrar un punto de sincronización seguro,
    /// permitiendo que el parser continúe parseando después de un error.
    /// 
    /// Los puntos de sincronización típicos son:
    /// - `;` (fin de expresión)
    /// - `}` (fin de bloque)
    /// - Palabras clave como `let`, `if`, `while`, `for`, `function`, etc.
    fn synchronize(&mut self) {
        self.cursor.advance();

        while !self.cursor.is_at_end() {
            // Si vemos un punto y coma, avanzamos una posición más y sincronizamos
            if self.cursor.peek().token_type == TokenType::Semicolon {
                self.cursor.advance();
                return;
            }

            // Si vemos un token que comienza una nueva declaración/expresión,
            // nos sincronizamos en ese punto
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
