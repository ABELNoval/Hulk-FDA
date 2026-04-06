// =============================================================================
// Lexer (Analizador Léxico / Tokenizador)
// =============================================================================
//
// El lexer es la primera fase del compilador. Transforma el código fuente
// (una secuencia de caracteres) en una secuencia de tokens.
//
// Responsabilidades:
// - Leer el código fuente carácter por carácter
// - Identificar y clasificar tokens (palabras clave, identificadores,
//   literales, operadores, delimitadores, etc.)
// - Ignorar espacios en blanco y comentarios
// - Rastrear posición (línea, columna) para reportar errores
// - Manejar errores léxicos (caracteres inválidos, strings sin cerrar, etc.)
//
// Tipos de tokens típicos:
// - Keywords: let, if, else, while, function, etc.
// - Identificadores: nombres de variables, funciones
// - Literales: números, strings, booleanos
// - Operadores: +, -, *, /, ==, !=, <, >, etc.
// - Delimitadores: (, ), {, }, [, ], ;, ,
// - EOF: fin de archivo
//
// =============================================================================

pub mod token;

// Re-exportar tipos principales para facilitar el uso
pub use token::{Token, TokenType};

use crate::lexer::token::{Token, TokenType};
use crate::utils::errors::span::Span;

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    file: String,
}

impl Lexer {
    pub fn new(input: String, file: String) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            file,
        }
    }

    // =========================================================
    // CORE
    // =========================================================

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let start_line = self.line;
        let start_col = self.column;

        let ch = match self.current_char() {
            Some(c) => c,
            None => return Token::eof(self.file.clone(), self.line, self.column),
        };

        let token = match ch {
            // ===================== OPERADORES =====================
            '+' => self.simple_token(TokenType::Plus),
            '-' => self.simple_token(TokenType::Minus),
            '*' => self.simple_token(TokenType::Star),
            '/' => self.simple_token(TokenType::Slash),
            '^' => self.simple_token(TokenType::Caret),
            '%' => self.simple_token(TokenType::Percent),

            '&' => self.simple_token(TokenType::Ampersand),
            '|' => self.simple_token(TokenType::Pipe),

            // ===================== LOOKAHEAD =====================
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.advance();
                    self.make_token("==", TokenType::EqualEqual, start_line, start_col)
                } else if self.peek() == Some('>') {
                    self.advance();
                    self.advance();
                    self.make_token("=>", TokenType::Arrow, start_line, start_col)
                } else {
                    self.advance();
                    self.make_token("=", TokenType::Equal, start_line, start_col)
                }
            }

            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.advance();
                    self.make_token("!=", TokenType::BangEqual, start_line, start_col)
                } else {
                    self.advance();
                    self.make_token("!", TokenType::Bang, start_line, start_col)
                }
            }

            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.advance();
                    self.make_token("<=", TokenType::LessEqual, start_line, start_col)
                } else {
                    self.advance();
                    self.make_token("<", TokenType::Less, start_line, start_col)
                }
            }

            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.advance();
                    self.make_token(">=", TokenType::GreaterEqual, start_line, start_col)
                } else {
                    self.advance();
                    self.make_token(">", TokenType::Greater, start_line, start_col)
                }
            }

            ':' => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.advance();
                    self.make_token(":=", TokenType::ColonEqual, start_line, start_col)
                } else {
                    self.advance();
                    self.make_token(":", TokenType::Colon, start_line, start_col)
                }
            }

            '@' => {
                if self.peek() == Some('@') {
                    self.advance();
                    self.advance();
                    self.make_token("@@", TokenType::AtAt, start_line, start_col)
                } else {
                    self.advance();
                    self.make_token("@", TokenType::At, start_line, start_col)
                }
            }

            // ===================== DELIMITADORES =====================
            '(' => self.simple_token(TokenType::LeftParen),
            ')' => self.simple_token(TokenType::RightParen),
            '{' => self.simple_token(TokenType::LeftBrace),
            '}' => self.simple_token(TokenType::RightBrace),
            '[' => self.simple_token(TokenType::LeftBracket),
            ']' => self.simple_token(TokenType::RightBracket),
            ';' => self.simple_token(TokenType::Semicolon),
            ',' => self.simple_token(TokenType::Comma),
            '.' => self.simple_token(TokenType::Dot),

            // ===================== STRING =====================
            '"' => return self.read_string(start_line, start_col),

            // ===================== NUMBER =====================
            '0'..='9' => return self.read_number(start_line, start_col),

            // ===================== IDENTIFIER / KEYWORD =====================
            'a'..='z' | 'A'..='Z' | '_' => {
                return self.read_identifier(start_line, start_col)
            }

            // ===================== ERROR =====================
            _ => {
                let lexeme = ch.to_string();
                self.advance();
                self.make_token(&lexeme, TokenType::Invalid, start_line, start_col)
            }
        };

        token
    }

    // =========================================================
    // READERS
    // =========================================================

    fn read_identifier(&mut self, line: usize, col: usize) -> Token {
        let start = self.position;

        while let Some(c) = self.current_char() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let lexeme: String = self.input[start..self.position].iter().collect();

        let token_type = TokenType::from_keyword(&lexeme)
            .unwrap_or(TokenType::Identifier(lexeme.clone()));

        self.make_token(&lexeme, token_type, line, col)
    }

    fn read_number(&mut self, line: usize, col: usize) -> Token {
        let start = self.position;

        while self.current_char().map_or(false, |c| c.is_ascii_digit()) {
            self.advance();
        }

        // decimal
        if self.current_char() == Some('.') {
            self.advance();
            while self.current_char().map_or(false, |c| c.is_ascii_digit()) {
                self.advance();
            }
        }

        let lexeme: String = self.input[start..self.position].iter().collect();

        let value = lexeme.parse::<f64>().unwrap_or(0.0);

        self.make_token(&lexeme, TokenType::Number(value), line, col)
    }

    fn read_string(&mut self, line: usize, col: usize) -> Token {
        self.advance(); // skip opening "

        let start = self.position;

        while let Some(c) = self.current_char() {
            if c == '"' {
                break;
            }
            if c == '\n' {
                return self.make_token("", TokenType::Invalid, line, col);
            }
            self.advance();
        }

        if self.current_char().is_none() {
            return self.make_token("", TokenType::Invalid, line, col);
        }

        let lexeme: String = self.input[start..self.position].iter().collect();

        self.advance(); // closing "

        self.make_token(&lexeme, TokenType::String(lexeme.clone()), line, col)
    }

    // =========================================================
    // HELPERS
    // =========================================================

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current_char() {
            match c {
                ' ' | '\t' | '\r' => self.advance(),
                '\n' => {
                    self.line += 1;
                    self.column = 1;
                    self.position += 1;
                }
                _ => break,
            }
        }
    }

    fn simple_token(&mut self, token_type: TokenType) -> Token {
        let ch = self.current_char().unwrap();
        let line = self.line;
        let col = self.column;

        self.advance();

        self.make_token(&ch.to_string(), token_type, line, col)
    }

    fn make_token(
        &self,
        lexeme: &str,
        token_type: TokenType,
        start_line: usize,
        start_col: usize,
    ) -> Token {
        Token::new(
            lexeme.to_string(),
            token_type,
            Span::new(
                self.file.clone(),
                start_line,
                start_col,
                self.line,
                self.column,
            ),
        )
    }

    fn current_char(&self) -> Option<char> {
        self.input.get(self.position).copied()
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.position + 1).copied()
    }

    fn advance(&mut self) {
        if let Some(c) = self.current_char() {
            self.position += 1;
            self.column += 1;

            if c == '\n' {
                self.line += 1;
                self.column = 1;
            }
        }
    }

    // =========================================================
    // UTILIDAD
    // =========================================================

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token();
            let is_eof = token.is_eof();
            tokens.push(token);

            if is_eof {
                break;
            }
        }

        tokens
    }
}