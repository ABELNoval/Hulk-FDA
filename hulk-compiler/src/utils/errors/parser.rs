use super::DisplayError;

/// Enum con todos los tipos de errores que puede producir el Parser
#[derive(Debug, Clone, PartialEq)]
pub enum ParserError {
    // ==================== TOKENS INESPERADOS ====================
    /// Token que no se esperaba en ese contexto
    /// Ejemplo: let = 5;  (falta el identificador)
    UnexpectedToken { expected: String, found: String },

    /// Se encontró EOF cuando se esperaba algo más
    /// Ejemplo: let x =
    UnexpectedEOF { expected: String },

    // ==================== EXPRESIONES ====================
    /// Se esperaba una expresión pero no se encontró
    /// Ejemplo: let x = ;
    ExpectedExpression { found: String },

    /// Se esperaba un identificador
    /// Ejemplo: let 5 = x;
    ExpectedIdentifier { found: String },

    /// Se esperaba un tipo
    /// Ejemplo: let x: = 5;
    ExpectedType { found: String },

    /// Se esperaba un literal (número, string, bool)
    /// Ejemplo: cuando se requiere un valor constante
    ExpectedLiteral { found: String },

    // ==================== DELIMITADORES ====================
    /// Paréntesis sin cerrar
    /// Ejemplo: let x = (5 + 3;
    UnclosedParenthesis {
        start_line: usize,
        start_column: usize,
    },

    /// Corchete sin cerrar
    /// Ejemplo: let arr = [1, 2, 3;
    UnclosedBracket {
        start_line: usize,
        start_column: usize,
    },

    /// Llave sin cerrar
    /// Ejemplo: if (true) { x = 5;
    UnclosedBrace {
        start_line: usize,
        start_column: usize,
    },

    /// Delimitador de cierre sin apertura correspondiente
    /// Ejemplo: let x = 5);
    UnmatchedClosingDelimiter { delimiter: char },

    // ==================== PUNTUACIÓN ====================
    /// Falta punto y coma
    /// Ejemplo: let x = 5 let y = 3
    ExpectedSemicolon { found: String },

    /// Falta coma en lista
    /// Ejemplo: foo(1 2 3)
    ExpectedComma {
        context: String, // "argumentos", "parámetros", "elementos de array"
    },

    /// Falta dos puntos
    /// Ejemplo: let x Number = 5;  (falta : antes de Number)
    ExpectedColon { context: String },

    /// Falta flecha (=> o ->)
    /// Ejemplo: function foo() Number { ... }  (falta =>)
    ExpectedArrow { found: String },

    // ==================== ASIGNACIÓN ====================
    /// Expresión inválida en el lado izquierdo de asignación
    /// Ejemplo: 5 + 3 = x;
    InvalidAssignmentTarget { target: String },

    /// Falta operador de asignación
    /// Ejemplo: let x 5;
    ExpectedAssignment { found: String },

    // ==================== OPERADORES ====================
    /// Operador binario sin operando derecho
    /// Ejemplo: let x = 5 +;
    MissingRightOperand { operator: String },

    /// Operador binario sin operando izquierdo
    /// Ejemplo: let x = + 5; (si + no es unario)
    MissingLeftOperand { operator: String },

    /// Operador unario sin operando
    /// Ejemplo: let x = !;
    MissingUnaryOperand { operator: String },

    // ==================== CONTROL DE FLUJO ====================
    /// Break fuera de un loop
    /// Ejemplo: break; (sin estar en while/for)
    BreakOutsideLoop,

    /// Continue fuera de un loop
    /// Ejemplo: continue; (sin estar en while/for)
    ContinueOutsideLoop,

    /// Return fuera de una función
    /// Ejemplo: return 5; (en el scope global)
    ReturnOutsideFunction,

    // ==================== DECLARACIONES ====================
    /// Falta nombre de función
    /// Ejemplo: function () { }
    ExpectedFunctionName,

    /// Falta cuerpo de función
    /// Ejemplo: function foo();
    ExpectedFunctionBody,

    /// Falta nombre de clase/tipo
    /// Ejemplo: type = { }
    ExpectedTypeName,

    /// Falta cuerpo de clase/tipo
    /// Ejemplo: type MyType;
    ExpectedTypeBody,

    /// Parámetro de función mal formado
    /// Ejemplo: function foo(x, , y) { }
    InvalidParameter { reason: String },

    // ==================== CONDICIONALES ====================
    /// Falta condición en if/while
    /// Ejemplo: if { }
    ExpectedCondition {
        construct: String, // "if", "while", "for"
    },

    /// Falta cuerpo en if/while
    /// Ejemplo: if (true)
    ExpectedBody { construct: String },

    /// Else sin if previo
    /// Ejemplo: else { }
    ElseWithoutIf,

    // ==================== LLAMADAS A FUNCIÓN ====================
    /// Argumentos mal formados en llamada
    /// Ejemplo: foo(,)
    InvalidArguments { reason: String },

    // ==================== LET/DECLARACIÓN DE VARIABLES ====================
    /// Let sin nombre de variable
    /// Ejemplo: let = 5;
    ExpectedVariableName,

    /// Let sin valor (si es requerido)
    /// Ejemplo: let x;  (si no se permite)
    ExpectedInitializer,

    // ==================== ARRAYS/COLECCIONES ====================
    /// Índice de array mal formado
    /// Ejemplo: arr[,]
    InvalidArrayIndex,

    /// Elemento de array mal formado
    /// Ejemplo: [1, , 3]
    InvalidArrayElement,

    // ==================== ERRORES GENERALES ====================
    /// Declaración inválida en este contexto
    /// Ejemplo: una declaración de función dentro de una expresión
    InvalidDeclaration {
        declaration_type: String,
        context: String,
    },

    /// Expresión incompleta
    /// Ejemplo: let x = if (true)
    IncompleteExpression { what: String },

    /// Múltiples errores de parsing (para recuperación)
    MultipleErrors { count: usize },
}

impl DisplayError for ParserError {
    /// Retorna el código de error (E1xxx)
    fn code(&self) -> &'static str {
        match self {
            ParserError::UnexpectedToken { .. } => "E1001",
            ParserError::UnexpectedEOF { .. } => "E1002",
            ParserError::ExpectedExpression { .. } => "E1010",
            ParserError::ExpectedIdentifier { .. } => "E1011",
            ParserError::ExpectedType { .. } => "E1012",
            ParserError::ExpectedLiteral { .. } => "E1013",
            ParserError::UnclosedParenthesis { .. } => "E1020",
            ParserError::UnclosedBracket { .. } => "E1021",
            ParserError::UnclosedBrace { .. } => "E1022",
            ParserError::UnmatchedClosingDelimiter { .. } => "E1023",
            ParserError::ExpectedSemicolon { .. } => "E1030",
            ParserError::ExpectedComma { .. } => "E1031",
            ParserError::ExpectedColon { .. } => "E1032",
            ParserError::ExpectedArrow { .. } => "E1033",
            ParserError::InvalidAssignmentTarget { .. } => "E1040",
            ParserError::ExpectedAssignment { .. } => "E1041",
            ParserError::MissingRightOperand { .. } => "E1050",
            ParserError::MissingLeftOperand { .. } => "E1051",
            ParserError::MissingUnaryOperand { .. } => "E1052",
            ParserError::BreakOutsideLoop => "E1060",
            ParserError::ContinueOutsideLoop => "E1061",
            ParserError::ReturnOutsideFunction => "E1062",
            ParserError::ExpectedFunctionName => "E1070",
            ParserError::ExpectedFunctionBody => "E1071",
            ParserError::ExpectedTypeName => "E1072",
            ParserError::ExpectedTypeBody => "E1073",
            ParserError::InvalidParameter { .. } => "E1074",
            ParserError::ExpectedCondition { .. } => "E1080",
            ParserError::ExpectedBody { .. } => "E1081",
            ParserError::ElseWithoutIf => "E1082",
            ParserError::InvalidArguments { .. } => "E1090",
            ParserError::ExpectedVariableName => "E1100",
            ParserError::ExpectedInitializer => "E1101",
            ParserError::InvalidArrayIndex => "E1110",
            ParserError::InvalidArrayElement => "E1111",
            ParserError::InvalidDeclaration { .. } => "E1120",
            ParserError::IncompleteExpression { .. } => "E1121",
            ParserError::MultipleErrors { .. } => "E1199",
        }
    }

    /// Retorna el mensaje de error descriptivo
    fn message(&self) -> String {
        match self {
            ParserError::UnexpectedToken { expected, found } => {
                format!("se esperaba {}, se encontró '{}'", expected, found)
            }
            ParserError::UnexpectedEOF { expected } => {
                format!("fin de archivo inesperado, se esperaba {}", expected)
            }
            ParserError::ExpectedExpression { found } => {
                format!("se esperaba una expresión, se encontró '{}'", found)
            }
            ParserError::ExpectedIdentifier { found } => {
                format!("se esperaba un identificador, se encontró '{}'", found)
            }
            ParserError::ExpectedType { found } => {
                format!("se esperaba un tipo, se encontró '{}'", found)
            }
            ParserError::ExpectedLiteral { found } => {
                format!("se esperaba un literal, se encontró '{}'", found)
            }
            ParserError::UnclosedParenthesis {
                start_line,
                start_column,
            } => {
                format!(
                    "paréntesis '(' sin cerrar (abierto en línea {}, columna {})",
                    start_line, start_column
                )
            }
            ParserError::UnclosedBracket {
                start_line,
                start_column,
            } => {
                format!(
                    "corchete '[' sin cerrar (abierto en línea {}, columna {})",
                    start_line, start_column
                )
            }
            ParserError::UnclosedBrace {
                start_line,
                start_column,
            } => {
                format!(
                    "llave '{{' sin cerrar (abierta en línea {}, columna {})",
                    start_line, start_column
                )
            }
            ParserError::UnmatchedClosingDelimiter { delimiter } => {
                format!(
                    "delimitador de cierre '{}' sin apertura correspondiente",
                    delimiter
                )
            }
            ParserError::ExpectedSemicolon { found } => {
                format!("se esperaba ';', se encontró '{}'", found)
            }
            ParserError::ExpectedComma { context } => {
                format!("se esperaba ',' entre {}", context)
            }
            ParserError::ExpectedColon { context } => {
                format!("se esperaba ':' {}", context)
            }
            ParserError::ExpectedArrow { found } => {
                format!("se esperaba '=>' o '->', se encontró '{}'", found)
            }
            ParserError::InvalidAssignmentTarget { target } => {
                format!("'{}' no es un objetivo de asignación válido", target)
            }
            ParserError::ExpectedAssignment { found } => {
                format!("se esperaba '=', se encontró '{}'", found)
            }
            ParserError::MissingRightOperand { operator } => {
                format!("falta operando derecho para el operador '{}'", operator)
            }
            ParserError::MissingLeftOperand { operator } => {
                format!("falta operando izquierdo para el operador '{}'", operator)
            }
            ParserError::MissingUnaryOperand { operator } => {
                format!("falta operando para el operador unario '{}'", operator)
            }
            ParserError::BreakOutsideLoop => {
                "break solo puede usarse dentro de un bucle".to_string()
            }
            ParserError::ContinueOutsideLoop => {
                "continue solo puede usarse dentro de un bucle".to_string()
            }
            ParserError::ReturnOutsideFunction => {
                "return solo puede usarse dentro de una función".to_string()
            }
            ParserError::ExpectedFunctionName => "se esperaba el nombre de la función".to_string(),
            ParserError::ExpectedFunctionBody => "se esperaba el cuerpo de la función".to_string(),
            ParserError::ExpectedTypeName => "se esperaba el nombre del tipo".to_string(),
            ParserError::ExpectedTypeBody => "se esperaba el cuerpo del tipo".to_string(),
            ParserError::InvalidParameter { reason } => {
                format!("parámetro inválido: {}", reason)
            }
            ParserError::ExpectedCondition { construct } => {
                format!("se esperaba una condición después de '{}'", construct)
            }
            ParserError::ExpectedBody { construct } => {
                format!("se esperaba un cuerpo para '{}'", construct)
            }
            ParserError::ElseWithoutIf => "else sin if correspondiente".to_string(),
            ParserError::InvalidArguments { reason } => {
                format!("argumentos inválidos: {}", reason)
            }
            ParserError::ExpectedVariableName => {
                "se esperaba el nombre de la variable después de 'let'".to_string()
            }
            ParserError::ExpectedInitializer => {
                "se esperaba un valor inicial para la variable".to_string()
            }
            ParserError::InvalidArrayIndex => "índice de array inválido".to_string(),
            ParserError::InvalidArrayElement => "elemento de array inválido".to_string(),
            ParserError::InvalidDeclaration {
                declaration_type,
                context,
            } => {
                format!(
                    "declaración de {} no permitida en {}",
                    declaration_type, context
                )
            }
            ParserError::IncompleteExpression { what } => {
                format!("expresión {} incompleta", what)
            }
            ParserError::MultipleErrors { count } => {
                format!("se encontraron {} errores de sintaxis", count)
            }
        }
    }

    /// Retorna una sugerencia de cómo arreglar el error (si aplica)
    fn help(&self) -> Option<String> {
        match self {
            ParserError::UnclosedParenthesis { .. } => {
                Some("agrega ')' para cerrar el paréntesis".to_string())
            }
            ParserError::UnclosedBracket { .. } => {
                Some("agrega ']' para cerrar el corchete".to_string())
            }
            ParserError::UnclosedBrace { .. } => {
                Some("agrega '}' para cerrar la llave".to_string())
            }
            ParserError::ExpectedSemicolon { .. } => {
                Some("agrega ';' al final de la sentencia".to_string())
            }
            ParserError::BreakOutsideLoop => {
                Some("usa break solo dentro de while o for".to_string())
            }
            ParserError::ContinueOutsideLoop => {
                Some("usa continue solo dentro de while o for".to_string())
            }
            ParserError::ReturnOutsideFunction => {
                Some("usa return solo dentro del cuerpo de una función".to_string())
            }
            ParserError::InvalidAssignmentTarget { .. } => {
                Some("solo se puede asignar a variables o propiedades".to_string())
            }
            ParserError::ElseWithoutIf => Some("else debe ir precedido de un if".to_string()),
            ParserError::ExpectedInitializer => {
                Some("agrega '= valor' después del nombre de la variable".to_string())
            }
            _ => None,
        }
    }
}
