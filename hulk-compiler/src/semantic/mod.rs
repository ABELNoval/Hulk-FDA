// =============================================================================
// Semantic (Analizador Semántico)
// =============================================================================
//
// El analizador semántico verifica que el programa tenga sentido más allá de
// la sintaxis. Opera sobre el AST producido por el parser.
//
// Responsabilidades:
// - Type Checking: verificar que los tipos sean correctos
// - Scope Resolution: resolver referencias a variables y funciones
// - Symbol Table: mantener tabla de símbolos con declaraciones
// - Control Flow Analysis: verificar returns, breaks, etc.
// - Detección de errores semánticos
//
// Errores semánticos típicos:
// - Variable no declarada
// - Variable declarada múltiples veces en el mismo scope
// - Tipos incompatibles en operaciones
// - Función llamada con número incorrecto de argumentos
// - Return fuera de función
// - Uso de variable antes de inicialización
//
// El resultado puede ser:
// - AST anotado con información de tipos
// - Tabla de símbolos completa
// - Lista de errores semánticos
//
// =============================================================================
