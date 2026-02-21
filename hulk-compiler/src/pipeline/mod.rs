// =============================================================================
// Pipeline (Orquestador de Compilación)
// =============================================================================
//
// Este módulo actúa como el orquestador principal del proceso de compilación.
// Coordina todas las fases del compilador en el orden correcto:
//
// 1. Lectura del código fuente
// 2. Análisis léxico (Lexer) -> Tokens
// 3. Análisis sintáctico (Parser) -> AST
// 4. Análisis semántico -> AST anotado/validado
// 5. Generación de IR -> Representación intermedia
// 6. Optimizaciones (opcional)
// 7. Generación de código -> Código objetivo
//
// Responsabilidades:
// - Manejar el flujo de datos entre fases
// - Propagar errores de manera adecuada
// - Permitir configuración de qué fases ejecutar
// - Proveer hooks para debugging/logging entre fases
// - Manejar múltiples archivos fuente si es necesario
//
// =============================================================================
