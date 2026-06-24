use crate::utils::errors::span::Span;
// =============================================================================
// TokenType - Tipos de tokens del lenguaje HULK
// =============================================================================
//
// Basado en la especificación oficial de HULK (secciones 1-16)
//
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // =========================================================================
    // Literales (Sección 1-3)
    // =========================================================================
    /// Literal numérico (entero o decimal)
    /// Ejemplos: 42, 3.14159, 1.5e10
    Number(f64),

    /// Literal de cadena de texto
    /// Ejemplo: "Hello, World!"
    String(String),

    /// Literal booleano true
    True,

    /// Literal booleano false
    False,

    // =========================================================================
    // Identificadores
    // =========================================================================
    /// Identificador (nombres de variables, funciones, tipos)
    /// Debe comenzar con letra o '_', seguido de letras, dígitos o '_'
    Identifier(String),

    // =========================================================================
    // Palabras clave - Control de flujo (Sección 4-5)
    // =========================================================================
    /// Palabra clave 'if'
    If,

    /// Palabra clave 'elif' (else if)
    Elif,

    /// Palabra clave 'else'
    Else,

    /// Palabra clave 'while'
    While,

    /// Palabra clave 'for'
    For,

    /// Palabra clave 'in' (usado en for-in)
    In,

    // =========================================================================
    // Palabras clave - Declaraciones (Sección 6-8)
    // =========================================================================
    /// Palabra clave 'let' (declaración de variables)
    Let,

    /// Palabra clave 'function' (declaración de funciones)
    Function,

    /// Palabra clave 'type' (declaración de tipos)
    Type,

    /// Palabra clave 'new' (instanciación de objetos)
    New,

    /// Palabra clave 'inherits' (herencia de tipos)
    Inherits,

    /// Palabra clave 'self' (referencia al objeto actual)
    SelfKeyword,

    // =========================================================================
    // Palabras clave - Protocolos (Sección 9)
    // =========================================================================
    /// Palabra clave 'protocol' (declaración de protocolos)
    Protocol,

    /// Palabra clave 'extends' (extensión de protocolos)
    Extends,

    // =========================================================================
    // Palabras clave - Operadores de tipo (Sección 11)
    // =========================================================================
    /// Palabra clave 'is' (verificación de tipo en runtime)
    Is,

    /// Palabra clave 'as' (downcasting de tipo)
    As,

    // =========================================================================
    // Operadores aritméticos (Sección 1)
    // =========================================================================
    /// Operador de suma '+'
    Plus,

    /// Operador de resta '-'
    Minus,

    /// Operador de multiplicación '*'
    Star,

    /// Operador de división '/'
    Slash,

    /// Operador de potencia '^'
    Caret,

    /// Operador de módulo '%'
    Percent,

    // =========================================================================
    // Operadores de concatenación (Sección 3)
    // =========================================================================
    /// Operador de concatenación '@' (con espacio)
    At,

    /// Operador de concatenación '@@' (sin espacio)
    AtAt,

    // =========================================================================
    // Operadores de comparación (Sección 1)
    // =========================================================================
    /// Operador de igualdad '=='
    EqualEqual,

    /// Operador de desigualdad '!='
    BangEqual,

    /// Operador menor que '<'
    Less,

    /// Operador mayor que '>'
    Greater,

    /// Operador menor o igual '<='
    LessEqual,

    /// Operador mayor o igual '>='
    GreaterEqual,

    // =========================================================================
    // Operadores lógicos (Sección 1)
    // =========================================================================
    /// Operador AND lógico '&'
    Ampersand,

    /// Operador OR lógico '|'
    Pipe,

    DoublePipe,

    /// Operador NOT lógico '!'
    Bang,

    // =========================================================================
    // Operadores de asignación (Sección 6)
    // =========================================================================
    /// Operador de asignación simple '='
    Equal,

    /// Operador de asignación destructiva ':='
    ColonEqual,

    // =========================================================================
    // Operador lambda (Sección 7)
    // =========================================================================
    /// Operador flecha '=>' (para funciones inline)
    Arrow,

    // =========================================================================
    // Delimitadores
    // =========================================================================
    /// Paréntesis izquierdo '('
    LeftParen,

    /// Paréntesis derecho ')'
    RightParen,

    /// Llave izquierda '{'
    LeftBrace,

    /// Llave derecha '}'
    RightBrace,

    /// Corchete izquierdo '['
    LeftBracket,

    /// Corchete derecho ']'
    RightBracket,

    /// Punto y coma ';'
    Semicolon,

    /// Coma ','
    Comma,

    /// Punto '.' (acceso a miembros)
    Dot,

    /// Dos puntos ':' (anotación de tipos)
    Colon,

    // =========================================================================
    // Constantes matemáticas (Sección 12)
    // =========================================================================
    /// Constante PI
    Pi,

    /// Constante E (número de Euler)
    E,

    // =========================================================================
    // Tokens especiales
    // =========================================================================
    /// Fin de archivo
    Eof,

    /// Token inválido (para manejo de errores)
    Invalid,
}

// =============================================================================
// Token - Representa un token con su valor, tipo y posición
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// El texto original del token en el código fuente
    pub lexeme: String,
    /// El tipo de token
    pub token_type: TokenType,
    /// La posición del token en el código fuente
    pub span: Span,
}

impl Token {
    /// Crea un nuevo token
    pub fn new(lexeme: String, token_type: TokenType, span: Span) -> Self {
        Self {
            lexeme,
            token_type,
            span,
        }
    }

    /// Crea un token de fin de archivo
    pub fn eof(file: String, line: usize, column: usize) -> Self {
        Self {
            lexeme: String::new(),
            token_type: TokenType::Eof,
            span: Span::new(file, line, column, line, column),
        }
    }

    /// Verifica si el token es una palabra clave
    pub fn is_keyword(&self) -> bool {
        matches!(
            self.token_type,
            TokenType::If
                | TokenType::Elif
                | TokenType::Else
                | TokenType::While
                | TokenType::For
                | TokenType::In
                | TokenType::Let
                | TokenType::Function
                | TokenType::Type
                | TokenType::New
                | TokenType::Inherits
                | TokenType::SelfKeyword
                | TokenType::Protocol
                | TokenType::Extends
                | TokenType::Is
                | TokenType::As
                | TokenType::True
                | TokenType::False
        )
    }

    /// Verifica si el token es un literal
    pub fn is_literal(&self) -> bool {
        matches!(
            self.token_type,
            TokenType::Number(_) | TokenType::String(_) | TokenType::True | TokenType::False
        )
    }

    /// Verifica si el token es un operador
    pub fn is_operator(&self) -> bool {
        matches!(
            self.token_type,
            TokenType::Plus
                | TokenType::Minus
                | TokenType::Star
                | TokenType::Slash
                | TokenType::Caret
                | TokenType::Percent
                | TokenType::At
                | TokenType::AtAt
                | TokenType::EqualEqual
                | TokenType::BangEqual
                | TokenType::Less
                | TokenType::Greater
                | TokenType::LessEqual
                | TokenType::GreaterEqual
                | TokenType::Ampersand
                | TokenType::Pipe
                | TokenType::Bang
                | TokenType::Equal
                | TokenType::ColonEqual
                | TokenType::Arrow
        )
    }

    /// Verifica si el token es un delimitador
    pub fn is_delimiter(&self) -> bool {
        matches!(
            self.token_type,
            TokenType::LeftParen
                | TokenType::RightParen
                | TokenType::LeftBrace
                | TokenType::RightBrace
                | TokenType::LeftBracket
                | TokenType::RightBracket
                | TokenType::Semicolon
                | TokenType::Comma
                | TokenType::Dot
                | TokenType::Colon
        )
    }

    /// Verifica si el token es EOF
    pub fn is_eof(&self) -> bool {
        matches!(self.token_type, TokenType::Eof)
    }
}

// =============================================================================
// Utilidades para mapeo de palabras clave
// =============================================================================

impl TokenType {
    /// Convierte un string a su TokenType correspondiente si es una palabra clave
    /// Retorna None si no es una palabra clave
    pub fn from_keyword(word: &str) -> Option<TokenType> {
        match word {
            // Control de flujo
            "if" => Some(TokenType::If),
            "elif" => Some(TokenType::Elif),
            "else" => Some(TokenType::Else),
            "while" => Some(TokenType::While),
            "for" => Some(TokenType::For),
            "in" => Some(TokenType::In),

            // Declaraciones
            "let" => Some(TokenType::Let),
            "function" => Some(TokenType::Function),
            "type" => Some(TokenType::Type),
            "new" => Some(TokenType::New),
            "inherits" => Some(TokenType::Inherits),
            "self" => Some(TokenType::SelfKeyword),

            // Protocolos
            "protocol" => Some(TokenType::Protocol),
            "extends" => Some(TokenType::Extends),

            // Operadores de tipo
            "is" => Some(TokenType::Is),
            "as" => Some(TokenType::As),

            // Booleanos
            "true" => Some(TokenType::True),
            "false" => Some(TokenType::False),

            // Constantes matemáticas
            "PI" => Some(TokenType::Pi),
            "E" => Some(TokenType::E),

            _ => None,
        }
    }

    /// Returns a human-readable name for the token type
    pub fn name(&self) -> &'static str {
        match self {
            // Literals
            TokenType::Number(_) => "number",
            TokenType::String(_) => "string",
            TokenType::True => "true",
            TokenType::False => "false",

            // Identifier
            TokenType::Identifier(_) => "identifier",

            // Control flow
            TokenType::If => "if",
            TokenType::Elif => "elif",
            TokenType::Else => "else",
            TokenType::While => "while",
            TokenType::For => "for",
            TokenType::In => "in",

            // Declarations
            TokenType::Let => "let",
            TokenType::Function => "function",
            TokenType::Type => "type",
            TokenType::New => "new",
            TokenType::Inherits => "inherits",
            TokenType::SelfKeyword => "self",

            // Protocols
            TokenType::Protocol => "protocol",
            TokenType::Extends => "extends",

            // Type operators
            TokenType::Is => "is",
            TokenType::As => "as",

            // Arithmetic operators
            TokenType::Plus => "+",
            TokenType::Minus => "-",
            TokenType::Star => "*",
            TokenType::Slash => "/",
            TokenType::Caret => "^",
            TokenType::Percent => "%",

            // Concatenation operators
            TokenType::At => "@",
            TokenType::AtAt => "@@",

            // Comparison operators
            TokenType::EqualEqual => "==",
            TokenType::BangEqual => "!=",
            TokenType::Less => "<",
            TokenType::Greater => ">",
            TokenType::LessEqual => "<=",
            TokenType::GreaterEqual => ">=",

            // Logical operators
            TokenType::Ampersand => "&",
            TokenType::Pipe => "|",
            TokenType::DoublePipe => "||",
            TokenType::Bang => "!",

            // Assignment
            TokenType::Equal => "=",
            TokenType::ColonEqual => ":=",

            // Lambda
            TokenType::Arrow => "=>",

            // Delimiters
            TokenType::LeftParen => "(",
            TokenType::RightParen => ")",
            TokenType::LeftBrace => "{",
            TokenType::RightBrace => "}",
            TokenType::LeftBracket => "[",
            TokenType::RightBracket => "]",
            TokenType::Semicolon => ";",
            TokenType::Comma => ",",
            TokenType::Dot => ".",
            TokenType::Colon => ":",

            // Constants
            TokenType::Pi => "PI",
            TokenType::E => "E",

            // Special
            TokenType::Eof => "end of file",
            TokenType::Invalid => "invalid token",
        }
    }
}
