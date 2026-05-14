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

use self::ast::{
    AttributeDeclaration, DeclarationKind, FunctionDeclaration, Parameter, TypeDeclaration,
    TypeMember,
};
use self::ast::{ProtocolDeclaration, ProtocolMethodSignature};
use crate::lexer::Token;
use crate::lexer::TokenType;
use crate::utils::errors::DisplayError;
use crate::utils::errors::ParserError;

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
        let mut declarations = Vec::new();
        let mut entry_expression = None;
        let mut program_span = self.cursor.peek().span.clone();

        while self.cursor.match_token(&TokenType::Semicolon) {}

        while !self.cursor.is_at_end() {
            if self.cursor.check(&TokenType::Function)
                || self.cursor.check(&TokenType::Type)
                || self.cursor.check(&TokenType::Protocol)
            {
                if let Some(declaration) = self.parse_declaration() {
                    program_span = if declarations.is_empty() && entry_expression.is_none() {
                        declaration.span.clone()
                    } else {
                        program_span.merge(&declaration.span)
                    };
                    declarations.push(declaration);
                } else {
                    self.synchronize();
                }

                while self.cursor.match_token(&TokenType::Semicolon) {}
                continue;
            }

            entry_expression = Some(self.parse_expression());
            if let Some(expr) = &entry_expression {
                program_span = if declarations.is_empty() && program_span == self.cursor.peek().span
                {
                    expr.span.clone()
                } else {
                    program_span.merge(&expr.span)
                };
            }

            while self.cursor.match_token(&TokenType::Semicolon) {}
            break;
        }

        if declarations.is_empty() && entry_expression.is_none() {
            program_span = self.cursor.peek().span.clone();
        }

        Program::new(declarations, entry_expression, program_span)
    }

    pub fn parse_declaration(&mut self) -> Option<Declaration> {
        if self.cursor.check(&TokenType::Function) {
            self.parse_function_declaration()
        } else if self.cursor.check(&TokenType::Type) {
            self.parse_type_declaration()
        } else if self.cursor.check(&TokenType::Protocol) {
            self.parse_protocol_declaration()
        } else {
            None
        }
    }

    pub fn parse_type_reference(&mut self) -> TypeReference {
        let type_token = self.cursor.peek().clone();

        let mut type_reference = match &type_token.token_type {
            TokenType::Identifier(name) => {
                self.cursor.advance();
                TypeReference::new(name.clone(), type_token.span)
            }
            _ => {
                self.error(ParserError::ExpectedType {
                    found: type_token.lexeme,
                });
                TypeReference::new("ErrorType".to_string(), type_token.span)
            }
        };

        loop {
            if self.cursor.check(&TokenType::Star) {
                let star_token = self.cursor.advance();
                let span = type_reference.span.merge(&star_token.span);
                type_reference = TypeReference::iterable_of(type_reference, span);
            } else if self.cursor.check(&TokenType::LeftBracket) {
                let left_bracket = self.cursor.advance();

                let right_bracket = match self.expect(TokenType::RightBracket) {
                    Ok(token) => token,
                    Err(_) => {
                        self.synchronize();
                        break;
                    }
                };

                let span = type_reference
                    .span
                    .merge(&left_bracket.span)
                    .merge(&right_bracket.span);
                type_reference = TypeReference::vector_of(type_reference, span);
            } else {
                break;
            }
        }

        type_reference
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

    /// Parse una declaración de función top-level.
    ///
    /// Gramática (MVP):
    /// function_decl = "function" identifier "(" parameters? ")"
    ///                 [":" type_ref] ("=>" expression | block)
    fn parse_function_declaration(&mut self) -> Option<Declaration> {
        let function_token = match self.expect(TokenType::Function) {
            Ok(token) => token,
            Err(_) => return None,
        };

        let start_span = function_token.span;

        let name_token = self.cursor.peek().clone();
        let name = match &name_token.token_type {
            TokenType::Identifier(name) => {
                self.cursor.advance();
                name.clone()
            }
            _ => {
                self.error(ParserError::ExpectedFunctionName);
                self.synchronize();
                return None;
            }
        };

        if self.expect(TokenType::LeftParen).is_err() {
            self.synchronize();
            return None;
        }

        let parameters = self.parse_parameter_list();

        let return_type = if self.cursor.match_token(&TokenType::Colon) {
            Some(self.parse_type_reference())
        } else {
            None
        };

        let body = if self.cursor.match_token(&TokenType::Arrow) {
            self.parse_expression()
        } else if self.cursor.check(&TokenType::LeftBrace) {
            self.parse_block()
        } else {
            self.error(ParserError::ExpectedFunctionBody);
            self.synchronize();
            return None;
        };

        let span = start_span.merge(&body.span);

        Some(Declaration::new(
            DeclarationKind::Function(FunctionDeclaration {
                name,
                parameters,
                return_type,
                body,
            }),
            span,
        ))
    }

    /// Parse una declaración de tipo top-level.
    ///
    /// Gramática (MVP):
    /// type_decl = "type" identifier "(" parameters? ")"
    ///             ["inherits" type_ref ["(" arguments? ")"]]
    ///             "{" type_member* "}"
    fn parse_type_declaration(&mut self) -> Option<Declaration> {
        let type_token = match self.expect(TokenType::Type) {
            Ok(token) => token,
            Err(_) => return None,
        };

        let start_span = type_token.span;

        let name_token = self.cursor.peek().clone();
        let name = match &name_token.token_type {
            TokenType::Identifier(name) => {
                self.cursor.advance();
                name.clone()
            }
            _ => {
                self.error(ParserError::ExpectedTypeName);
                self.synchronize();
                return None;
            }
        };

        if self.expect(TokenType::LeftParen).is_err() {
            self.synchronize();
            return None;
        }

        let parameters = self.parse_parameter_list();

        let inherits = if self.cursor.match_token(&TokenType::Inherits) {
            Some(self.parse_type_reference())
        } else {
            None
        };

        let parent_arguments = if inherits.is_some() && self.cursor.check(&TokenType::LeftParen) {
            self.parse_argument_list()
        } else {
            Vec::new()
        };

        let members = if self.cursor.check(&TokenType::LeftBrace) {
            self.parse_type_members()
        } else {
            self.error(ParserError::ExpectedTypeBody);
            self.synchronize();
            return None;
        };

        let end_span = members
            .last()
            .map(|member| match member {
                TypeMember::Attribute(attribute) => attribute.span.clone(),
                TypeMember::Method(function) => function.body.span.clone(),
            })
            .unwrap_or_else(|| self.cursor.peek().span.clone());

        let span = start_span.merge(&end_span);

        Some(Declaration::new(
            DeclarationKind::Type(TypeDeclaration {
                name,
                parameters,
                inherits,
                parent_arguments,
                members,
            }),
            span,
        ))
    }

    /// Parsea los miembros dentro de una declaración de tipo.
    fn parse_type_members(&mut self) -> Vec<TypeMember> {
        let mut members = Vec::new();

        if self.expect(TokenType::LeftBrace).is_err() {
            return members;
        }

        while !self.cursor.check(&TokenType::RightBrace) && !self.cursor.is_at_end() {
            if self.cursor.match_token(&TokenType::Semicolon) {
                continue;
            }

            if self.cursor.check(&TokenType::Function) {
                if let Some(declaration) = self.parse_function_declaration() {
                    if let DeclarationKind::Function(function) = declaration.kind {
                        members.push(TypeMember::Method(function));
                    }
                }
            } else if matches!(self.cursor.peek().token_type, TokenType::Identifier(_)) {
                if let Some(attribute) = self.parse_type_attribute() {
                    members.push(TypeMember::Attribute(attribute));
                }
            } else {
                self.error(ParserError::InvalidDeclaration {
                    declaration_type: "type member".to_string(),
                    context: "type body".to_string(),
                });
                self.synchronize();
            }

            self.cursor.match_token(&TokenType::Semicolon);
        }

        if self.expect(TokenType::RightBrace).is_err() {
            self.synchronize();
        }

        members
    }

    /// Parsea un atributo dentro de un tipo.
    fn parse_type_attribute(&mut self) -> Option<AttributeDeclaration> {
        let name_token = self.cursor.peek().clone();
        let name = match &name_token.token_type {
            TokenType::Identifier(name) => {
                self.cursor.advance();
                name.clone()
            }
            _ => return None,
        };

        let annotation = if self.cursor.match_token(&TokenType::Colon) {
            Some(self.parse_type_reference())
        } else {
            None
        };

        if self.expect(TokenType::Equal).is_err() {
            self.synchronize();
            return None;
        }

        let initializer = self.parse_expression();
        let span = name_token.span.merge(&initializer.span);

        Some(AttributeDeclaration {
            name,
            annotation,
            initializer,
            span,
        })
    }

    /// Parsea una lista de argumentos entre paréntesis.
    fn parse_argument_list(&mut self) -> Vec<Expr> {
        let mut arguments = Vec::new();

        if self.expect(TokenType::LeftParen).is_err() {
            return arguments;
        }

        if self.cursor.match_token(&TokenType::RightParen) {
            return arguments;
        }

        loop {
            arguments.push(self.parse_expression());

            if self.cursor.match_token(&TokenType::Comma) {
                continue;
            }

            break;
        }

        if self.expect(TokenType::RightParen).is_err() {
            self.synchronize();
        }

        arguments
    }

    /// Parse una declaración de protocolo.
    ///
    /// Gramática (MVP):
    /// protocol_decl = "protocol" identifier ["extends" type_ref ("," type_ref)*]
    ///                 "{" protocol_member* "}"
    fn parse_protocol_declaration(&mut self) -> Option<Declaration> {
        let protocol_token = match self.expect(TokenType::Protocol) {
            Ok(t) => t,
            Err(_) => return None,
        };

        let start_span = protocol_token.span;

        let name_token = self.cursor.peek().clone();
        let name = match &name_token.token_type {
            TokenType::Identifier(n) => {
                self.cursor.advance();
                n.clone()
            }
            _ => {
                self.error(ParserError::ExpectedTypeName);
                self.synchronize();
                return None;
            }
        };

        let mut extends = Vec::new();
        if self.cursor.match_token(&TokenType::Extends) {
            // one or more type refs separated by commas
            loop {
                extends.push(self.parse_type_reference());
                if !self.cursor.match_token(&TokenType::Comma) {
                    break;
                }
            }
        }

        // members
        let mut members = Vec::new();

        if self.expect(TokenType::LeftBrace).is_err() {
            self.synchronize();
            return None;
        }

        while !self.cursor.check(&TokenType::RightBrace) && !self.cursor.is_at_end() {
            // parse method signature
            if let Some(sig) = self.parse_protocol_method_signature() {
                members.push(sig);
            } else {
                // skip token to avoid infinite loop
                self.cursor.advance();
            }

            // optional semicolon between members
            self.cursor.match_token(&TokenType::Semicolon);
        }

        if self.expect(TokenType::RightBrace).is_err() {
            self.synchronize();
        }

        let end_span = self.cursor.peek().span.clone();
        let span = start_span.merge(&end_span);

        Some(Declaration::new(
            DeclarationKind::Protocol(ProtocolDeclaration {
                name,
                extends,
                members,
            }),
            span,
        ))
    }

    /// Parsea una firma de método dentro de un protocolo: name(params): Type
    fn parse_protocol_method_signature(&mut self) -> Option<ProtocolMethodSignature> {
        let name_token = self.cursor.peek().clone();
        let name = match &name_token.token_type {
            TokenType::Identifier(n) => {
                self.cursor.advance();
                n.clone()
            }
            _ => return None,
        };

        if self.expect(TokenType::LeftParen).is_err() {
            self.synchronize();
            return None;
        }

        let parameters = self.parse_parameter_list();

        // Expect colon and return type
        let return_type = if self.cursor.match_token(&TokenType::Colon) {
            self.parse_type_reference()
        } else {
            // default to error type if missing
            self.error(ParserError::ExpectedType {
                found: "<missing>".to_string(),
            });
            TypeReference::new("Void".to_string(), self.cursor.peek().span.clone())
        };

        let span = name_token.span.merge(&return_type.span);

        Some(ProtocolMethodSignature {
            name,
            parameters,
            return_type,
            span,
        })
    }

    /// Parsea la lista de parámetros de una función.
    fn parse_parameter_list(&mut self) -> Vec<Parameter> {
        let mut parameters = Vec::new();

        if self.cursor.match_token(&TokenType::RightParen) {
            return parameters;
        }

        loop {
            let parameter_token = self.cursor.peek().clone();

            let parameter_name = match &parameter_token.token_type {
                TokenType::Identifier(name) => {
                    self.cursor.advance();
                    name.clone()
                }
                _ => {
                    self.error(ParserError::InvalidParameter {
                        reason: "se esperaba un identificador de parámetro".to_string(),
                    });

                    while !self.cursor.is_at_end()
                        && !self.cursor.check(&TokenType::Comma)
                        && !self.cursor.check(&TokenType::RightParen)
                    {
                        self.cursor.advance();
                    }

                    if self.cursor.match_token(&TokenType::Comma) {
                        continue;
                    }

                    break;
                }
            };

            let annotation = if self.cursor.match_token(&TokenType::Colon) {
                Some(self.parse_type_reference())
            } else {
                None
            };

            let parameter_span = match &annotation {
                Some(type_ref) => parameter_token.span.merge(&type_ref.span),
                None => parameter_token.span,
            };

            parameters.push(Parameter::new(parameter_name, annotation, parameter_span));

            if self.cursor.match_token(&TokenType::Comma) {
                continue;
            }

            break;
        }

        if self.expect(TokenType::RightParen).is_err() {
            self.synchronize();
        }

        parameters
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
            return Expr::new(ExprKind::Literal(Literal::Number(0.0)), start_span);
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
            .or_else(|| elif_parts.last().map(|(_, e)| e.span.clone()))
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
            return Expr::new(ExprKind::Literal(Literal::Number(0.0)), start_span);
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
            return Expr::new(ExprKind::Literal(Literal::Number(0.0)), start_span);
        }

        let variable = match &var_token.token_type {
            TokenType::Identifier(v) => v.clone(),
            _ => return Expr::new(ExprKind::Literal(Literal::Number(0.0)), start_span),
        };

        self.cursor.advance();

        // Esperamos "in"
        if let Err(_) = self.expect(TokenType::In) {
            self.synchronize();
            return Expr::new(ExprKind::Literal(Literal::Number(0.0)), start_span);
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

        Expr::new(ExprKind::Literal(Literal::Number(0.0)), token.span)
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
