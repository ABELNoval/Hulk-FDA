use super::DisplayError;
use super::span::Span;

/// Enum con todos los tipos de errores que puede producir el análisis semántico
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticError {
    // ==================== ERRORES DE VARIABLES ====================
    /// Variable usada pero nunca declarada
    /// Ejemplo: print(x);  // x no existe
    UndeclaredVariable { name: String, span: Span },

    /// Variable declarada múltiples veces en el mismo scope
    /// Ejemplo: let x = 5; let x = 10;
    VariableAlreadyDeclared {
        name: String,
        first_line: usize,
        first_column: usize,
    },

    /// Variable usada antes de ser inicializada
    /// Ejemplo: let x; print(x);
    UninitializedVariable { name: String },

    /// Asignación a variable inmutable (constante)
    /// Ejemplo: const x = 5; x = 10;
    AssignmentToImmutable { name: String },

    /// Variable declarada pero nunca usada (warning)
    UnusedVariable { name: String },

    // ==================== ERRORES DE TIPOS ====================
    /// Tipos incompatibles en asignación u operación
    /// Ejemplo: let x: Number = "hola";
    TypeMismatch {
        expected: String,
        found: String,
        context: String, // "asignación", "retorno", "argumento", etc.
        span: Span,
    },

    /// Operador no soportado para esos tipos
    /// Ejemplo: "hola" - "mundo"
    InvalidBinaryOperator {
        operator: String,
        left_type: String,
        right_type: String,
    },

    /// Operador unario no soportado para el tipo
    /// Ejemplo: -"hola"
    InvalidUnaryOperator {
        operator: String,
        operand_type: String,
    },

    /// No se puede inferir el tipo de la expresión
    CannotInferType { context: String },

    /// Tipo no declarado/no existe
    /// Ejemplo: let x: MiTipo = 5;  // MiTipo no existe
    UndeclaredType { name: String },

    /// Protocolo no declarado/no existe
    /// Ejemplo: let x: MiProtocolo = 5;  // MiProtocolo no existe
    UndeclaredProtocol { name: String },

    /// Tipos incompatibles en operación de comparación
    /// Ejemplo: 5 == "hola"
    IncomparableTypes {
        left_type: String,
        right_type: String,
    },

    /// Tipo de operando inválido en un contexto específico
    InvalidOperandType {
        expected: String,
        found: String,
        context: String,
    },

    /// Target de evaluación que no es válido (ej. asignación a rvalue)
    InvalidTarget { context: String },

    // ==================== ERRORES DE FUNCIONES ====================
    /// Función no declarada
    /// Ejemplo: foo();  // foo no existe
    UndeclaredFunction { name: String },

    /// Función no definida
    /// Ejemplo: foo();  // foo no está definida
    UndefinedFunction { name: String },

    /// Función declarada múltiples veces
    FunctionAlreadyDeclared {
        name: String,
        first_line: usize,
        first_column: usize,
    },

    /// Número incorrecto de argumentos en llamada
    /// Ejemplo: foo(1, 2) cuando foo espera 3 args
    WrongArgumentCount {
        function: String,
        expected: usize,
        found: usize,
    },

    /// Tipo de argumento incorrecto
    ArgumentTypeMismatch {
        function: String,
        parameter_name: String,
        parameter_position: usize,
        expected: String,
        found: String,
    },

    /// Función debe retornar un valor pero no lo hace
    MissingReturnValue {
        function: String,
        expected_type: String,
    },

    /// Función no debe retornar valor pero lo hace
    UnexpectedReturnValue { function: String },

    /// Return con tipo incorrecto
    ReturnTypeMismatch {
        function: String,
        expected: String,
        found: String,
    },

    /// No todos los caminos de la función retornan un valor
    NotAllPathsReturn { function: String },

    /// Función recursiva sin caso base detectado (warning)
    PossibleInfiniteRecursion { function: String },

    // ==================== ERRORES DE CLASES/TIPOS DEFINIDOS ====================
    /// Tipo/clase ya declarada
    TypeAlreadyDeclared {
        name: String,
        first_line: usize,
        first_column: usize,
    },

    /// Protocolo ya declarado
    ProtocolAlreadyDeclared {
        name: String,
        first_line: usize,
        first_column: usize,
    },

    /// Herencia circular detectada
    /// Ejemplo: type A inherits B { } type B inherits A { }
    CircularInheritance {
        type_name: String,
        cycle: Vec<String>, // ["A", "B", "A"]
    },

    /// Herencia de un tipo que no existe
    InheritFromUndeclared {
        type_name: String,
        parent_name: String,
    },

    /// Miembro/atributo no encontrado en el tipo
    /// Ejemplo: obj.foo donde obj no tiene foo
    MemberNotFound {
        type_name: String,
        member_name: String,
    },

    /// Miembro ya declarado en el tipo
    MemberAlreadyDeclared {
        type_name: String,
        member_name: String,
    },

    /// Acceso a miembro privado desde fuera
    PrivateMemberAccess {
        type_name: String,
        member_name: String,
    },

    /// Método no encontrado
    MethodNotFound {
        type_name: String,
        method_name: String,
    },

    /// Constructor no encontrado o inválido
    InvalidConstructor { type_name: String, reason: String },

    /// Self usado fuera de un método
    SelfOutsideMethod,

    // ==================== ERRORES DE CONTROL DE FLUJO ====================
    /// Condición no es de tipo booleano
    /// Ejemplo: if (5) { }
    NonBooleanCondition {
        found_type: String,
        context: String, // "if", "while", "for"
    },

    /// Las ramas de if tienen tipos incompatibles
    /// Ejemplo: if (x) 5 else "hola"
    IncompatibleBranchTypes {
        then_type: String,
        else_type: String,
    },

    // ==================== ERRORES DE ARRAYS/COLECCIONES ====================
    /// Tipo no es indexable
    /// Ejemplo: let x = 5; x[0];
    NotIndexable { type_name: String },

    /// Índice no es de tipo entero
    InvalidIndexType { expected: String, found: String },

    /// Elementos de array con tipos inconsistentes
    InconsistentArrayTypes {
        expected: String,
        found: String,
        position: usize,
    },

    // ==================== ERRORES ARITMÉTICOS (compile-time) ====================
    /// División por cero detectada en tiempo de compilación
    DivisionByZero,

    /// Módulo por cero detectado en tiempo de compilación
    ModuloByZero,

    /// Overflow detectado en tiempo de compilación
    IntegerOverflow { operation: String },

    // ==================== ERRORES DE PATRONES (si aplica) ====================
    /// Patrón no exhaustivo en match
    NonExhaustivePattern { missing: Vec<String> },

    /// Patrón inalcanzable
    UnreachablePattern,

    // ==================== OTROS ====================
    /// Referencia cíclica en definiciones
    CyclicDefinition { names: Vec<String> },

    /// Expresión constante requerida pero no proporcionada
    NonConstantExpression { context: String },

    /// Expresión no soportada
    UnsupportedExpression { expression_type: String },

    /// Característica del lenguaje no soportada
    UnsupportedFeature { feature: String },
}

impl DisplayError for SemanticError {
    /// Retorna el código de error (E2xxx)
    fn code(&self) -> &'static str {
        match self {
            // Variables
            SemanticError::UndeclaredVariable { .. } => "E2001",
            SemanticError::VariableAlreadyDeclared { .. } => "E2002",
            SemanticError::UninitializedVariable { .. } => "E2003",
            SemanticError::AssignmentToImmutable { .. } => "E2004",
            SemanticError::UnusedVariable { .. } => "E2005",
            // Tipos
            SemanticError::TypeMismatch { .. } => "E2010",
            SemanticError::InvalidBinaryOperator { .. } => "E2011",
            SemanticError::InvalidUnaryOperator { .. } => "E2012",
            SemanticError::CannotInferType { .. } => "E2013",
            SemanticError::UndeclaredType { .. } => "E2014",
            SemanticError::IncomparableTypes { .. } => "E2015",
            SemanticError::InvalidOperandType { .. } => "E2016",
            SemanticError::InvalidTarget { .. } => "E2017",
            // Funciones
            SemanticError::UndeclaredFunction { .. } => "E2020",
            /// Función ya declarada
            SemanticError::FunctionAlreadyDeclared { .. } => "E2021",
            SemanticError::WrongArgumentCount { .. } => "E2022",
            SemanticError::ArgumentTypeMismatch { .. } => "E2023",
            SemanticError::MissingReturnValue { .. } => "E2024",
            SemanticError::UnexpectedReturnValue { .. } => "E2025",
            SemanticError::ReturnTypeMismatch { .. } => "E2026",
            SemanticError::NotAllPathsReturn { .. } => "E2027",
            SemanticError::PossibleInfiniteRecursion { .. } => "E2028",
            // Clases/Tipos
            SemanticError::TypeAlreadyDeclared { .. } => "E2030",
            SemanticError::CircularInheritance { .. } => "E2031",
            SemanticError::InheritFromUndeclared { .. } => "E2032",
            SemanticError::MemberNotFound { .. } => "E2033",
            SemanticError::MemberAlreadyDeclared { .. } => "E2034",
            SemanticError::PrivateMemberAccess { .. } => "E2035",
            SemanticError::MethodNotFound { .. } => "E2036",
            SemanticError::InvalidConstructor { .. } => "E2037",
            SemanticError::SelfOutsideMethod => "E2038",
            // Control de flujo
            SemanticError::NonBooleanCondition { .. } => "E2040",
            SemanticError::IncompatibleBranchTypes { .. } => "E2041",
            // Arrays
            SemanticError::NotIndexable { .. } => "E2050",
            SemanticError::InvalidIndexType { .. } => "E2051",
            SemanticError::InconsistentArrayTypes { .. } => "E2052",
            // Aritméticos
            SemanticError::DivisionByZero => "E2060",
            SemanticError::ModuloByZero => "E2061",
            SemanticError::IntegerOverflow { .. } => "E2062",
            // Patrones
            SemanticError::NonExhaustivePattern { .. } => "E2070",
            SemanticError::UnreachablePattern => "E2071",
            // Otros
            SemanticError::CyclicDefinition { .. } => "E2080",
            SemanticError::NonConstantExpression { .. } => "E2081",
            SemanticError::UnsupportedExpression { .. } => "E2082",
            SemanticError::UnsupportedFeature { .. } => "E2099",
            SemanticError::UndeclaredProtocol { name } => "E2100",
            SemanticError::ProtocolAlreadyDeclared {
                name,
                first_line,
                first_column,
            } => "E2101",
            SemanticError::UndefinedFunction { name } => "E2102",
        }
    }

    /// Retorna el mensaje de error descriptivo
    fn message(&self) -> String {
        match self {
            // Variables
            SemanticError::UndeclaredVariable { name, .. } => {
                format!("variable '{}' no declarada", name)
            }
            SemanticError::VariableAlreadyDeclared {
                name,
                first_line,
                first_column,
            } => {
                format!(
                    "variable '{}' ya fue declarada (primera declaración en línea {}, columna {})",
                    name, first_line, first_column
                )
            }
            SemanticError::UninitializedVariable { name } => {
                format!("variable '{}' usada antes de ser inicializada", name)
            }
            SemanticError::AssignmentToImmutable { name } => {
                format!("no se puede asignar a '{}' porque es inmutable", name)
            }
            SemanticError::UnusedVariable { name } => {
                format!("variable '{}' declarada pero nunca usada", name)
            }
            // Tipos
            SemanticError::TypeMismatch {
                expected,
                found,
                context,
                ..
            } => {
                format!(
                    "tipos incompatibles en {}: se esperaba '{}', se encontró '{}'",
                    context, expected, found
                )
            }
            SemanticError::InvalidBinaryOperator {
                operator,
                left_type,
                right_type,
            } => {
                format!(
                    "operador '{}' no puede aplicarse a '{}' y '{}'",
                    operator, left_type, right_type
                )
            }
            SemanticError::InvalidUnaryOperator {
                operator,
                operand_type,
            } => {
                format!(
                    "operador '{}' no puede aplicarse a '{}'",
                    operator, operand_type
                )
            }
            SemanticError::CannotInferType { context } => {
                format!("no se puede inferir el tipo en {}", context)
            }
            SemanticError::UndeclaredType { name } => {
                format!("tipo '{}' no declarado", name)
            }
            SemanticError::IncomparableTypes {
                left_type,
                right_type,
            } => {
                format!(
                    "no se pueden comparar valores de tipo '{}' y '{}'",
                    left_type, right_type
                )
            }
            SemanticError::InvalidOperandType {
                expected,
                found,
                context,
            } => {
                format!(
                    "tipo de operando inválido en {}: esperaba '{}', se encontró '{}'",
                    context, expected, found
                )
            }
            SemanticError::InvalidTarget { context } => {
                format!("target no válido para {}", context)
            }
            // Funciones
            SemanticError::UndeclaredFunction { name } => {
                format!("función '{}' no declarada", name)
            }
            SemanticError::FunctionAlreadyDeclared {
                name,
                first_line,
                first_column,
            } => {
                format!(
                    "función '{}' ya fue declarada (primera declaración en línea {}, columna {})",
                    name, first_line, first_column
                )
            }
            SemanticError::WrongArgumentCount {
                function,
                expected,
                found,
            } => {
                format!(
                    "función '{}' espera {} argumento(s), se proporcionaron {}",
                    function, expected, found
                )
            }
            SemanticError::ArgumentTypeMismatch {
                function,
                parameter_name,
                parameter_position,
                expected,
                found,
            } => {
                format!(
                    "argumento {} ('{}') de '{}': se esperaba '{}', se encontró '{}'",
                    parameter_position, parameter_name, function, expected, found
                )
            }
            SemanticError::MissingReturnValue {
                function,
                expected_type,
            } => {
                format!(
                    "función '{}' debe retornar '{}' pero no tiene sentencia return",
                    function, expected_type
                )
            }
            SemanticError::UnexpectedReturnValue { function } => {
                format!("función '{}' no debe retornar un valor", function)
            }
            SemanticError::ReturnTypeMismatch {
                function,
                expected,
                found,
            } => {
                format!(
                    "tipo de retorno incorrecto en '{}': se esperaba '{}', se encontró '{}'",
                    function, expected, found
                )
            }
            SemanticError::NotAllPathsReturn { function } => {
                format!(
                    "no todos los caminos de ejecución en '{}' retornan un valor",
                    function
                )
            }
            SemanticError::PossibleInfiniteRecursion { function } => {
                format!(
                    "posible recursión infinita en '{}': no se detectó caso base",
                    function
                )
            }
            // Clases/Tipos
            SemanticError::TypeAlreadyDeclared {
                name,
                first_line,
                first_column,
            } => {
                format!(
                    "tipo '{}' ya fue declarado (primera declaración en línea {}, columna {})",
                    name, first_line, first_column
                )
            }
            SemanticError::CircularInheritance { type_name, cycle } => {
                format!(
                    "herencia circular detectada en '{}': {}",
                    type_name,
                    cycle.join(" -> ")
                )
            }
            SemanticError::InheritFromUndeclared {
                type_name,
                parent_name,
            } => {
                format!(
                    "tipo '{}' intenta heredar de '{}' que no está declarado",
                    type_name, parent_name
                )
            }
            SemanticError::MemberNotFound {
                type_name,
                member_name,
            } => {
                format!(
                    "tipo '{}' no tiene un miembro llamado '{}'",
                    type_name, member_name
                )
            }
            SemanticError::MemberAlreadyDeclared {
                type_name,
                member_name,
            } => {
                format!(
                    "miembro '{}' ya existe en tipo '{}'",
                    member_name, type_name
                )
            }
            SemanticError::PrivateMemberAccess {
                type_name,
                member_name,
            } => {
                format!(
                    "no se puede acceder al miembro privado '{}' de '{}'",
                    member_name, type_name
                )
            }
            SemanticError::MethodNotFound {
                type_name,
                method_name,
            } => {
                format!(
                    "tipo '{}' no tiene un método llamado '{}'",
                    type_name, method_name
                )
            }
            SemanticError::InvalidConstructor { type_name, reason } => {
                format!("constructor inválido para '{}': {}", type_name, reason)
            }
            SemanticError::SelfOutsideMethod => {
                "self solo puede usarse dentro de un método".to_string()
            }
            // Control de flujo
            SemanticError::NonBooleanCondition {
                found_type,
                context,
            } => {
                format!(
                    "condición de '{}' debe ser Boolean, se encontró '{}'",
                    context, found_type
                )
            }
            SemanticError::IncompatibleBranchTypes {
                then_type,
                else_type,
            } => {
                format!(
                    "las ramas del if tienen tipos incompatibles: '{}' y '{}'",
                    then_type, else_type
                )
            }
            // Arrays
            SemanticError::NotIndexable { type_name } => {
                format!("tipo '{}' no es indexable", type_name)
            }
            SemanticError::InvalidIndexType { expected, found } => {
                format!(
                    "índice debe ser de tipo '{}', se encontró '{}'",
                    expected, found
                )
            }
            SemanticError::InconsistentArrayTypes {
                expected,
                found,
                position,
            } => {
                format!(
                    "elemento {} del array tiene tipo '{}', se esperaba '{}'",
                    position, found, expected
                )
            }
            // Aritméticos
            SemanticError::DivisionByZero => "división por cero".to_string(),
            SemanticError::ModuloByZero => "módulo por cero".to_string(),
            SemanticError::IntegerOverflow { operation } => {
                format!("overflow de entero en operación '{}'", operation)
            }
            // Patrones
            SemanticError::NonExhaustivePattern { missing } => {
                format!(
                    "patrón no exhaustivo, faltan los casos: {}",
                    missing.join(", ")
                )
            }
            SemanticError::UnreachablePattern => "este patrón nunca será alcanzado".to_string(),
            // Otros
            SemanticError::CyclicDefinition { names } => {
                format!("definición cíclica detectada: {}", names.join(" -> "))
            }
            SemanticError::NonConstantExpression { context } => {
                format!("se requiere una expresión constante en {}", context)
            }
            SemanticError::UnsupportedExpression { expression_type } => {
                format!("tipo de expresión no soportada: {}", expression_type)
            }
            SemanticError::UnsupportedFeature { feature } => {
                format!("característica no soportada: {}", feature)
            }
            SemanticError::UndeclaredProtocol { name } => {
                format!("protocolo no declarado: {}", name)
            }
            SemanticError::ProtocolAlreadyDeclared {
                name,
                first_line,
                first_column,
            } => {
                format!(
                    "protocolo ya declarado: {} en la línea {}, columna {}",
                    name, first_line, first_column
                )
            }
            SemanticError::UndefinedFunction { name } => {
                format!("función no definida: {}", name)
            }
        }
    }

    /// Retorna una sugerencia de cómo arreglar el error (si aplica)
    fn help(&self) -> Option<String> {
        match self {
            SemanticError::UndeclaredVariable { name, .. } => {
                Some(format!("declara la variable con: let {} = valor;", name))
            }
            SemanticError::VariableAlreadyDeclared { .. } => {
                Some("usa un nombre diferente o elimina la declaración duplicada".to_string())
            }
            SemanticError::UninitializedVariable { name } => {
                Some(format!("asigna un valor a '{}' antes de usarla", name))
            }
            SemanticError::AssignmentToImmutable { .. } => {
                Some("declara la variable como mutable si necesitas modificarla".to_string())
            }
            SemanticError::TypeMismatch { expected, .. } => Some(format!(
                "asegúrate de que la expresión sea de tipo '{}'",
                expected
            )),
            SemanticError::UndeclaredFunction { name } => {
                Some(format!("define la función '{}' antes de usarla", name))
            }
            SemanticError::WrongArgumentCount { expected, .. } => {
                Some(format!("proporciona exactamente {} argumento(s)", expected))
            }
            SemanticError::MissingReturnValue { .. } => {
                Some("agrega una sentencia 'return valor;' en la función".to_string())
            }
            SemanticError::NotAllPathsReturn { .. } => {
                Some("asegúrate de que todas las ramas del código retornen un valor".to_string())
            }
            SemanticError::CircularInheritance { .. } => {
                Some("reorganiza la jerarquía de tipos para evitar ciclos".to_string())
            }
            SemanticError::NonBooleanCondition { .. } => {
                Some("usa una expresión que evalúe a true o false".to_string())
            }
            SemanticError::IncompatibleBranchTypes { .. } => {
                Some("asegúrate de que ambas ramas retornen el mismo tipo".to_string())
            }
            SemanticError::DivisionByZero => {
                Some("verifica el divisor antes de dividir".to_string())
            }
            SemanticError::SelfOutsideMethod => {
                Some("self solo es válido dentro de métodos de una clase".to_string())
            }
            _ => None,
        }
    }
}

impl SemanticError {
    /// Retorna una ubicación best-effort para errores semánticos con posición explícita.
    pub fn location(&self) -> Option<(usize, usize)> {
        match self {
            SemanticError::UndeclaredVariable { span, .. } => {
                Some((span.start_line, span.start_column))
            }
            SemanticError::TypeMismatch { span, .. } => Some((span.start_line, span.start_column)),
            SemanticError::VariableAlreadyDeclared {
                first_line,
                first_column,
                ..
            }
            | SemanticError::FunctionAlreadyDeclared {
                first_line,
                first_column,
                ..
            }
            | SemanticError::TypeAlreadyDeclared {
                first_line,
                first_column,
                ..
            } => Some((*first_line, *first_column)),
            _ => None,
        }
    }
}
