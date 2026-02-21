// =============================================================================
// Utils (Utilidades)
// =============================================================================
//
// Módulo de utilidades compartidas por todo el compilador.
// Contiene funcionalidad común que no pertenece a ninguna fase específica.
//
// Componentes típicos:
//
// - Source Location: estructuras para rastrear posiciones en el código
//   (archivo, línea, columna, spans)
//
// - Error Reporting: sistema de reportes de error con colores,
//   subrayado del código problemático, sugerencias
//
// - Diagnostics: warnings, hints, notas informativas
//
// - Interner: interning de strings para eficiencia de memoria
//
// - Arena Allocator: para asignación eficiente del AST
//
// - File Handling: lectura de archivos, manejo de encoding
//
// - Pretty Printing: formateo de estructuras para debugging
//
// - Logger: sistema de logging con niveles (debug, info, warn, error)
//
// =============================================================================
