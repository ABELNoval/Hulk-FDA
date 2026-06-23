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
use crate::parser::ast::{BinaryOperator, ExprKind, UnaryOperator};
use crate::utils::errors::ParserError;
use crate::utils::errors::span::Span;

pub mod ast;
pub use ast::{Declaration, Expr, Literal, Program, TypeReference};

// =============================================================================
// Parser - API común de alto nivel
// =============================================================================

#[derive(Debug, Clone)]
pub struct Parser {
    cursor: TokenCursor,
    errors: Vec<ParserDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParserDiagnostic {
    pub error: ParserError,
    pub span: Span,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            cursor: TokenCursor::new(tokens),
            errors: Vec::new(),
        }
    }

    pub fn errors(&self) -> &[ParserDiagnostic] {
        &self.errors
    }

    pub fn take_errors(&mut self) -> Vec<ParserDiagnostic> {
        std::mem::take(&mut self.errors)
    }

    pub fn parse_program_with_errors(&mut self) -> (Program, Vec<ParserDiagnostic>) {
        let program = self.parse_program();
        let errors = self.take_errors();
        (program, errors)
    }

    pub fn parse_expression_with_errors(&mut self) -> (Expr, Vec<ParserDiagnostic>) {
        let expression = self.parse_expression();
        let errors = self.take_errors();
        (expression, errors)
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
                program_span = if declarations.is_empty() {
                    expr.span.clone()
                } else {
                    program_span.merge(&expr.span)
                };
            }

            if !self.cursor.is_at_end() {
                match self.expect(TokenType::Semicolon) {
                    Ok(token) => token.span,
                    Err(_) => {
                        self.synchronize();
                        self.cursor.peek().span.clone()
                    }
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

        if start_token.is_err() {
            return Expr::literal(Literal::Number(0.0), self.cursor.peek().span.clone());
        }

        let start_span = start_token.unwrap().span;
        let mut expressions = Vec::new();

        while !self.cursor.check(&TokenType::RightBrace) && !self.cursor.is_at_end() {
            let expr = self.parse_expression();
            expressions.push(expr);
            match self.expect(TokenType::Semicolon) {
                Ok(token) => token.span,
                Err(_) => {
                    self.synchronize();
                    self.cursor.peek().span.clone()
                }
            };
        }

        let end_span = match self.expect(TokenType::RightBrace) {
            Ok(token) => token.span,
            Err(_) => {
                self.synchronize();
                self.cursor.peek().span.clone()
            }
        };

        let span = start_span.merge(&end_span);
        Expr::block(expressions, span)
    }

    fn parse_let_binding(&mut self) -> Expr {
        let start_token = self.expect(TokenType::Let);

        if start_token.is_err() {
            return Expr::literal(Literal::Number(0.0), self.cursor.peek().span.clone());
        }

        let start_span = start_token.unwrap().span;
        let mut bindings = Vec::new();

        if let Some(expr) = self.parse_single_let_binding() {
            bindings.push(expr);
        }

        while self.cursor.match_token(&TokenType::Comma) {
            if let Some(expr) = self.parse_single_let_binding() {
                bindings.push(expr);
            } else {
                break;
            }
        }

        if self.expect(TokenType::In).is_err() {
            self.synchronize();

            return Expr::literal(Literal::Number(0.0), start_span);
        }

        let body = self.parse_expression();

        let mut expr = body;

        for (name, annotation, value) in bindings.into_iter().rev() {
            let span = start_span.merge(&expr.span);

            expr = Expr::let_expr(name, annotation, value, expr, span);
        }

        expr
    }

    fn parse_single_let_binding(&mut self) -> Option<(String, Option<TypeReference>, Expr)> {
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

        if self.expect(TokenType::Equal).is_err() {
            self.synchronize();
            return None;
        }

        let value = self.parse_expression();
        let end_span = value.span.clone();
        let span = start_span.merge(&end_span);

        Some((name, annotation, value))
    }

    fn parse_if_expr(&mut self) -> Expr {
        let start_token = self.expect(TokenType::If);

        if start_token.is_err() {
            return Expr::literal(Literal::Number(0.0), self.cursor.peek().span.clone());
        }

        let start_span = start_token.unwrap().span;

        if self.expect(TokenType::LeftParen).is_err() {
            self.synchronize();
            return Expr::literal(Literal::Number(0.0), start_span);
        }

        let condition = self.parse_expression();

        if self.expect(TokenType::RightParen).is_err() {
            self.synchronize();
        }

        let then_expr = self.parse_expression();
        let (elif_parts, else_expr) = self.parse_elif_parts();

        let end_span = else_expr.span.clone();

        let span = start_span.merge(&end_span);
        Expr::if_expr(condition, then_expr, elif_parts, Some(else_expr), span)
    }

    fn parse_elif_parts(&mut self) -> (Vec<(Expr, Expr)>, Expr) {
        let mut elif_parts = Vec::new();

        while self.cursor.match_token(&TokenType::Elif) {
            if self.expect(TokenType::LeftParen).is_err() {
                self.synchronize();
                break;
            }

            let condition = self.parse_expression();

            if self.expect(TokenType::RightParen).is_err() {
                self.synchronize();
            }

            let elif_expr = self.parse_expression();
            elif_parts.push((condition, elif_expr));
        }

        if self.expect(TokenType::Else).is_err() {
            self.synchronize();

            return (
                elif_parts,
                Expr::literal(Literal::Number(0.0), self.cursor.peek().span.clone()),
            );
        }

        let else_expr = self.parse_expression();

        (elif_parts, else_expr)
    }

    fn parse_while_expr(&mut self) -> Expr {
        let start_token = self.expect(TokenType::While);

        if start_token.is_err() {
            return Expr::literal(Literal::Number(0.0), self.cursor.peek().span.clone());
        }

        let start_span = start_token.unwrap().span;

        if self.expect(TokenType::LeftParen).is_err() {
            self.synchronize();
            return Expr::literal(Literal::Number(0.0), start_span);
        }

        let condition = self.parse_expression();

        if self.expect(TokenType::RightParen).is_err() {
            self.synchronize();
        }

        let body = self.parse_expression();
        let end_span = body.span.clone();
        let span = start_span.merge(&end_span);
        Expr::while_expr(condition, body, span)
    }

    fn parse_for_expr(&mut self) -> Expr {
        let start_token = self.expect(TokenType::For);

        if start_token.is_err() {
            return Expr::literal(Literal::Number(0.0), self.cursor.peek().span.clone());
        }

        let start_span = start_token.unwrap().span;

        if self.expect(TokenType::LeftParen).is_err() {
            self.synchronize();
            return Expr::literal(Literal::Number(0.0), start_span);
        }

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

        if self.expect(TokenType::In).is_err() {
            self.synchronize();
            return Expr::literal(Literal::Number(0.0), start_span);
        }

        let iterable = self.parse_expression();

        if self.expect(TokenType::RightParen).is_err() {
            self.synchronize();
        }

        let body = self.parse_expression();
        let end_span = body.span.clone();
        let span = start_span.merge(&end_span);
        Expr::for_expr(variable, iterable, body, span)
    }

    fn parse_assignment_expr(&mut self) -> Expr {
        let target = self.parse_logical_or();

        if self.cursor.match_token(&TokenType::ColonEqual) {
            let value = self.parse_expression();

            if !matches!(
                &target.kind,
                ExprKind::Identifier(_)
                    | ExprKind::MemberAccess { .. }
                    | ExprKind::IndexAccess { .. }
            ) {
                self.error(ParserError::InvalidAssignmentTarget {
                    target: self.assignment_target_name(&target),
                });
                return value;
            }

            let span = target.span.merge(&value.span);
            return Expr::assignment(target, value, span);
        }

        target
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

        while self
            .cursor
            .check_any(&[TokenType::EqualEqual, TokenType::BangEqual])
        {
            let operator = self.cursor.advance();
            let right = self.parse_comparison();
            let span = expr.span.merge(&right.span);
            let op = BinaryOperator::from_token_type(&operator.token_type).unwrap();
            expr = Expr::binary(expr, op, right, span);
        }

        expr
    }

    fn parse_comparison(&mut self) -> Expr {
        let mut expr = self.parse_type_operations();

        while self.cursor.check_any(&[
            TokenType::Less,
            TokenType::LessEqual,
            TokenType::Greater,
            TokenType::GreaterEqual,
        ]) {
            let operator = self.cursor.advance();
            let right = self.parse_type_operations();
            let span = expr.span.merge(&right.span);
            let op = BinaryOperator::from_token_type(&operator.token_type).unwrap();
            expr = Expr::binary(expr, op, right, span);
        }

        expr
    }

    fn parse_type_operations(&mut self) -> Expr {
        let mut expr = self.parse_concatenation();

        loop {
            if self.cursor.match_token(&TokenType::Is) {
                let type_reference = self.parse_type_reference();
                let span = expr.span.merge(&type_reference.span);
                expr = Expr::type_check(expr, type_reference, span);
            } else if self.cursor.match_token(&TokenType::As) {
                let type_reference = self.parse_type_reference();
                let span = expr.span.merge(&type_reference.span);
                expr = Expr::type_cast(expr, type_reference, span);
            } else {
                break;
            }
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

        while self
            .cursor
            .check_any(&[TokenType::Star, TokenType::Slash, TokenType::Percent])
        {
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
        if self.cursor.check_any(&[TokenType::Minus, TokenType::Bang]) {
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
                let open_token = self.cursor.advance();
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
                    self.error(ParserError::UnclosedParenthesis {
                        start_line: open_token.span.start_line,
                        start_column: open_token.span.start_column,
                    });
                    let span = if let Some(last_argument) = arguments.last() {
                        expr.span.merge(&last_argument.span)
                    } else {
                        expr.span.clone()
                    };
                    expr = Expr::call(expr, arguments, span);
                    break;
                }
            } else if self.cursor.match_token(&TokenType::Dot) {
                let member_token = self.cursor.peek().clone();

                match &member_token.token_type {
                    TokenType::Identifier(member_name) => {
                        self.cursor.advance();
                        let span = expr.span.merge(&member_token.span);
                        expr = Expr::member_access(expr, member_name.clone(), span);
                    }
                    _ => {
                        self.error(ParserError::ExpectedIdentifier {
                            found: member_token.lexeme,
                        });
                        break;
                    }
                }
            } else if self.cursor.check(&TokenType::LeftBracket) {
                let open_token = self.cursor.advance();
                let index = self.parse_expression();

                if self.cursor.check(&TokenType::RightBracket) {
                    let close_span = self.cursor.advance().span;
                    let span = expr.span.merge(&close_span);
                    expr = Expr::index_access(expr, index, span);
                } else {
                    self.error(ParserError::UnclosedBracket {
                        start_line: open_token.span.start_line,
                        start_column: open_token.span.start_column,
                    });
                    let span = expr.span.merge(&index.span);
                    expr = Expr::index_access(expr, index, span);
                    break;
                }
            } else {
                break;
            }
        }

        expr
    }

    fn parse_vector(&mut self, start_span: Span) -> Expr {
        let mut elements = Vec::new();

        if self.cursor.match_token(&TokenType::RightBracket) {
            return Expr::vector_literal(vec![], start_span.clone());
        }

        let first_expr = self.parse_expression();

        // comprehension
        if self.cursor.match_token(&TokenType::DoublePipe) {
            let identifier = self.cursor.peek().clone();

            let binding = match &identifier.token_type {
                TokenType::Identifier(name) => {
                    self.cursor.advance();

                    name.clone()
                }

                _ => {
                    self.error(ParserError::ExpectedIdentifier {
                        found: identifier.lexeme,
                    });

                    return first_expr;
                }
            };

            if self.expect(TokenType::In).is_err() {
                self.synchronize();

                return first_expr;
            }

            let iterable = self.parse_expression();

            let end_span = match self.expect(TokenType::RightBracket) {
                Ok(token) => token.span,

                Err(_) => iterable.span.clone(),
            };

            let span = start_span.merge(&end_span);

            return Expr::vector_comprehension(first_expr, binding, iterable, span);
        }

        // vector normal

        elements.push(first_expr);

        while self.cursor.match_token(&TokenType::Comma) {
            elements.push(self.parse_expression());
        }

        let end_span = match self.expect(TokenType::RightBracket) {
            Ok(token) => token.span,

            Err(_) => elements.last().unwrap().span.clone(),
        };

        let span = start_span.merge(&end_span);

        Expr::vector_literal(elements, span)
    }

    fn parse_primary(&mut self) -> Expr {
        if self.cursor.check(&TokenType::LeftParen) {
            let open_token = self.cursor.advance();
            let expr = self.parse_expression();
            if self.cursor.check(&TokenType::RightParen) {
                self.cursor.advance();
            } else {
                self.error(ParserError::UnclosedParenthesis {
                    start_line: open_token.span.start_line,
                    start_column: open_token.span.start_column,
                });
            }
            return expr;
        }

        // Reconocer estructuras de control como subexpresiones
        if self.cursor.check(&TokenType::Let) {
            return self.parse_let_binding();
        } else if self.cursor.check(&TokenType::If) {
            return self.parse_if_expr();
        } else if self.cursor.check(&TokenType::While) {
            return self.parse_while_expr();
        } else if self.cursor.check(&TokenType::For) {
            return self.parse_for_expr();
        } else if self.cursor.check(&TokenType::LeftBrace) {
            return self.parse_block();
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
            TokenType::SelfKeyword => Expr::self_expr(span),
            TokenType::Base => Expr::base_expr(span),
            TokenType::New => {
                let type_reference = self.parse_type_reference();
                let mut arguments = Vec::new();
                let mut end_span = type_reference.span.clone();

                if self.cursor.check(&TokenType::LeftParen) {
                    let open_token = self.cursor.advance();

                    if !self.cursor.check(&TokenType::RightParen) {
                        loop {
                            arguments.push(self.parse_expression());
                            if self.cursor.match_token(&TokenType::Comma) {
                                continue;
                            }
                            break;
                        }
                    }

                    if self.cursor.check(&TokenType::RightParen) {
                        end_span = self.cursor.advance().span;
                    } else {
                        self.error(ParserError::UnclosedParenthesis {
                            start_line: open_token.span.start_line,
                            start_column: open_token.span.start_column,
                        });

                        if let Some(last_arg) = arguments.last() {
                            end_span = last_arg.span.clone();
                        } else {
                            end_span = open_token.span;
                        }
                    }
                } else {
                    self.error(ParserError::UnexpectedToken {
                        expected: "(".to_string(),
                        found: self.cursor.peek().lexeme.clone(),
                    });
                }

                let full_span = span.merge(&end_span);
                Expr::new_expr(type_reference, arguments, full_span)
            }
            TokenType::RightParen => {
                self.error(ParserError::UnmatchedClosingDelimiter { delimiter: ')' });
                Expr::literal(Literal::Number(0.0), span)
            }
            TokenType::RightBracket => {
                self.error(ParserError::UnmatchedClosingDelimiter { delimiter: ']' });
                Expr::literal(Literal::Number(0.0), span)
            }
            TokenType::RightBrace => {
                self.error(ParserError::UnmatchedClosingDelimiter { delimiter: '}' });
                Expr::literal(Literal::Number(0.0), span)
            }
            TokenType::LeftBracket => {
                let token = self.cursor.advance();

                return self.parse_vector(token.span);
            }
            _ => {
                self.error(ParserError::ExpectedExpression {
                    found: token.lexeme.clone(),
                });
                Expr::literal(Literal::Number(0.0), span)
            }
        }
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

    fn assignment_target_name(&self, expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Identifier(name) => name.clone(),
            ExprKind::MemberAccess { member, .. } => format!(".{}", member),
            ExprKind::IndexAccess { .. } => "index access".to_string(),
            _ => "expression".to_string(),
        }
    }

    fn is_expression_terminator(&self) -> bool {
        self.cursor
            .check_any(&[TokenType::Semicolon, TokenType::RightBrace, TokenType::Eof])
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
                if let Some(declaration) = self.parse_function_declaration()
                    && let DeclarationKind::Function(function) = declaration.kind
                {
                    members.push(TypeMember::Method(function));
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

        let end_span = match self.expect(TokenType::RightBrace) {
            Ok(token) => token.span,
            Err(_) => {
                self.synchronize();
                self.cursor.peek().span.clone()
            }
        };

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
                None => parameter_token.span.clone(),
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

    // =========================================================================
    // Utilidades internas - Helpers para parsing
    // =========================================================================

    /// Reporta un error de parsing.
    ///
    /// En una versión más completa, los errores se acumularían en un vector
    /// para reportarlos todos al final. Por ahora los reportamos inmediatamente.
    fn error(&mut self, error: ParserError) {
        let span = self.cursor.peek().span.clone();
        self.error_at(error, span);
    }

    fn error_at(&mut self, error: ParserError, span: Span) {
        self.errors.push(ParserDiagnostic { error, span });
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
