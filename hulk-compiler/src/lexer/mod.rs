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
use crate::utils::errors::span::Span;

// Estructura principal del lexer.
// Aquí guardo todo el estado necesario para recorrer el input carácter por carácter.
pub struct Lexer {
    input: Vec<char>,   // input convertido a vector de chars para acceso rápido
    position: usize,    // posición actual dentro del input
    line: usize,        // línea actual (para errores)
    column: usize,      // columna actual (para errores)
    file: String,       // nombre del archivo (para spans)
}

impl Lexer {
    // Constructor del lexer.
    // Inicializo todo y convierto el string en un vector de chars.
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

    // Esta es la función más importante: devuelve el siguiente token del input.
    pub fn next_token(&mut self) -> Token {
        // Primero ignoro espacios y comentarios antes de procesar algo.
        self.skip_whitespace_and_comments();

        // Guardo posición inicial del token (para el span)
        let start_line = self.line;
        let start_col = self.column;

        // Si no hay más caracteres, retorno EOF
        let ch = match self.current_char() {
            Some(c) => c,
            None => return Token::eof(self.file.clone(), self.line, self.column),
        };

        // Aquí hago el dispatch dependiendo del carácter actual
        match ch {
            // ===================== OPERADORES =====================
            // Operadores simples (un solo carácter)
            '+' => self.simple_token(TokenType::Plus),
            '-' => self.simple_token(TokenType::Minus),
            '*' => self.simple_token(TokenType::Star),

            '/' => self.simple_token(TokenType::Slash),

            '^' => self.simple_token(TokenType::Caret),
            '%' => self.simple_token(TokenType::Percent),

            '&' => self.simple_token(TokenType::Ampersand),
            '|' => self.simple_token(TokenType::Pipe),

            // ===================== LOOKAHEAD =====================
            // Operadores que pueden tener más de un carácter

            '=' => {
                // == o =>
                if self.peek() == Some('=') {
                    self.advance(); self.advance();
                    self.make_token("==", TokenType::EqualEqual, start_line, start_col)
                } else if self.peek() == Some('>') {
                    self.advance(); self.advance();
                    self.make_token("=>", TokenType::Arrow, start_line, start_col)
                } else {
                    self.advance();
                    self.make_token("=", TokenType::Equal, start_line, start_col)
                }
            }

            '!' => {
                // != o !
                if self.peek() == Some('=') {
                    self.advance(); self.advance();
                    self.make_token("!=", TokenType::BangEqual, start_line, start_col)
                } else {
                    self.advance();
                    self.make_token("!", TokenType::Bang, start_line, start_col)
                }
            }

            '<' => {
                // <= o <
                if self.peek() == Some('=') {
                    self.advance(); self.advance();
                    self.make_token("<=", TokenType::LessEqual, start_line, start_col)
                } else {
                    self.advance();
                    self.make_token("<", TokenType::Less, start_line, start_col)
                }
            }

            '>' => {
                // >= o >
                if self.peek() == Some('=') {
                    self.advance(); self.advance();
                    self.make_token(">=", TokenType::GreaterEqual, start_line, start_col)
                } else {
                    self.advance();
                    self.make_token(">", TokenType::Greater, start_line, start_col)
                }
            }

            ':' => {
                // := o :
                if self.peek() == Some('=') {
                    self.advance(); self.advance();
                    self.make_token(":=", TokenType::ColonEqual, start_line, start_col)
                } else {
                    self.advance();
                    self.make_token(":", TokenType::Colon, start_line, start_col)
                }
            }

            '@' => {
                // @@ o @
                if self.peek() == Some('@') {
                    self.advance(); self.advance();
                    self.make_token("@@", TokenType::AtAt, start_line, start_col)
                } else {
                    self.advance();
                    self.make_token("@", TokenType::At, start_line, start_col)
                }
            }

            // ===================== DELIMITADORES =====================
            // Tokens estructurales del lenguaje
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
            // Si encuentro comillas, leo un string completo
            '"' => self.read_string(start_line, start_col),

            // ===================== NUMBER =====================
            // Si empieza con número, leo número completo
            '0'..='9' => self.read_number(start_line, start_col),

            // ===================== IDENTIFIER =====================
            // Letras o _ → identificador o keyword
            'a'..='z' | 'A'..='Z' | '_' => {
                self.read_identifier(start_line, start_col)
            }

            // ===================== ERROR =====================
            // Cualquier cosa que no reconozco
            _ => {
                let msg = format!("Invalid character '{}'", ch);
                self.advance();
                self.error_token(msg, start_line, start_col)
            }
        }
    }

    // =========================================================
    // READERS
    // =========================================================

    // Lee identificadores o keywords
    fn read_identifier(&mut self, line: usize, col: usize) -> Token {
        let start = self.position;

        // Consumo letras, números o _
        while let Some(c) = self.current_char() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        // Construyo el lexeme
        let lexeme: String = self.input[start..self.position].iter().collect();

        // Verifico si es keyword o identificador
        let token_type = TokenType::from_keyword(&lexeme)
            .unwrap_or(TokenType::Identifier(lexeme.clone()));

        self.make_token(&lexeme, token_type, line, col)
    }

    // Lee números (enteros, decimales y científicos)
    fn read_number(&mut self, line: usize, col: usize) -> Token {
        let start = self.position;

        // Parte entera
        while self.current_char().map_or(false, |c| c.is_ascii_digit()) {
            self.advance();
        }

        // Parte decimal
        if self.current_char() == Some('.') {
            self.advance();
            while self.current_char().map_or(false, |c| c.is_ascii_digit()) {
                self.advance();
            }
        }

        // Notación científica (e o E)
        if matches!(self.current_char(), Some('e') | Some('E')) {
            self.advance();

            // signo opcional
            if matches!(self.current_char(), Some('+') | Some('-')) {
                self.advance();
            }

            // debe haber al menos un número después
            if !self.current_char().map_or(false, |c| c.is_ascii_digit()) {
                return self.error_token("Invalid scientific notation".into(), line, col);
            }

            while self.current_char().map_or(false, |c| c.is_ascii_digit()) {
                self.advance();
            }
        }

        let lexeme: String = self.input[start..self.position].iter().collect();

        // Intento parsear a f64
        match lexeme.parse::<f64>() {
            Ok(value) => self.make_token(&lexeme, TokenType::Number(value), line, col),
            Err(_) => self.error_token("Invalid number format".into(), line, col),
        }
    }

    // Lee strings con soporte de escapes
    fn read_string(&mut self, line: usize, col: usize) -> Token {
        self.advance(); // salto la comilla inicial

        let mut result = String::new();

        while let Some(c) = self.current_char() {
            match c {
                '"' => break, // fin del string

                '\\' => {
                    // manejo de escapes
                    self.advance();
                    match self.current_char() {
                        Some('n') => result.push('\n'),
                        Some('t') => result.push('\t'),
                        Some('"') => result.push('"'),
                        Some('\\') => result.push('\\'),
                        Some(other) => {
                            return self.error_token(
                                format!("Invalid escape sequence \\{}", other),
                                line,
                                col,
                            )
                        }
                        None => return self.error_token("Unterminated escape".into(), line, col),
                    }
                }

                // string sin cerrar
                '\n' => return self.error_token("Unterminated string".into(), line, col),

                _ => result.push(c),
            }

            self.advance();
        }

        // EOF sin cerrar string
        if self.current_char().is_none() {
            return self.error_token("Unterminated string".into(), line, col);
        }

        self.advance(); // salto comilla final

        self.make_token(&result, TokenType::String(result.clone()), line, col)
    }

    // =========================================================
    // COMMENTS + WHITESPACE
    // =========================================================

    // Salta espacios y comentarios
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.current_char() {
                // espacios simples
                Some(' ' | '\t' | '\r') => self.advance(),

                // nueva línea
                Some('\n') => {
                    self.line += 1;
                    self.column = 1;
                    self.position += 1;
                }

                // comentario de una línea
                Some('/') if self.peek() == Some('/') => {
                    while self.current_char() != Some('\n') && self.current_char().is_some() {
                        self.advance();
                    }
                }

                // comentario multilínea
                Some('/') if self.peek() == Some('*') => {
                    self.advance(); self.advance();

                    while let Some(c) = self.current_char() {
                        if c == '*' && self.peek() == Some('/') {
                            self.advance(); self.advance();
                            break;
                        }
                        self.advance();
                    }
                }

                _ => break,
            }
        }
    }

    // =========================================================
    // HELPERS
    // =========================================================

    // Para tokens de un solo carácter
    fn simple_token(&mut self, token_type: TokenType) -> Token {
        let ch = self.current_char().unwrap();
        let line = self.line;
        let col = self.column;

        self.advance();

        self.make_token(&ch.to_string(), token_type, line, col)
    }

    // Construcción estándar de token con span
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

    // Token de error (mensaje en lexeme)
    fn error_token(&self, message: String, line: usize, col: usize) -> Token {
        Token::new(
            message,
            TokenType::Invalid,
            Span::new(self.file.clone(), line, col, self.line, self.column),
        )
    }

    // Devuelve carácter actual
    fn current_char(&self) -> Option<char> {
        self.input.get(self.position).copied()
    }

    // Lookahead (sin consumir)
    fn peek(&self) -> Option<char> {
        self.input.get(self.position + 1).copied()
    }

    // Avanza una posición actualizando línea/columna
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

    // Tokeniza todo el input hasta EOF
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