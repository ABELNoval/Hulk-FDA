// =============================================================================
// IR (Intermediate Representation - Representación Intermedia)
// =============================================================================
//
// La IR es una representación del programa que está entre el código fuente
// de alto nivel y el código máquina de bajo nivel.
//
// Propósitos de la IR:
// - Independencia de la arquitectura objetivo
// - Facilitar optimizaciones
// - Simplificar la generación de código
// - Permitir múltiples backends (x86, ARM, LLVM, etc.)
//
// Formas comunes de IR:
// - Three-Address Code (TAC): instrucciones de máximo 3 operandos
// - SSA (Static Single Assignment): cada variable se asigna una sola vez
// - Stack-based: operaciones sobre una pila virtual
// - Graph-based: CFG (Control Flow Graph), DFG (Data Flow Graph)
//
// Componentes típicos:
// - Definición de la estructura de la IR
// - Conversión de AST a IR (lowering)
// - Pretty printing de la IR para debugging
// - Passes de optimización sobre la IR
//
// =============================================================================
