// =============================================================================
// Tests de Integración / Flujo Completo
// =============================================================================
//
// Este directorio contiene tests de integración que verifican el flujo
// completo del compilador, desde código fuente hasta código generado.
//
// A diferencia de los tests unitarios (que prueban componentes aislados),
// estos tests verifican que todas las fases trabajen correctamente juntas.
//
// Tipos de tests de integración:
//
// 1. End-to-End Tests:
//    - Compilar un programa completo
//    - Ejecutar el resultado
//    - Verificar la salida
//
// 2. Snapshot Tests:
//    - Comparar salida del compilador con salida esperada guardada
//    - Útil para detectar regresiones
//
// 3. Error Tests:
//    - Verificar que programas inválidos produzcan los errores correctos
//    - Probar mensajes de error útiles
//
// 4. Performance Tests:
//    - Medir tiempo de compilación
//    - Detectar regresiones de rendimiento
//
// Estructura sugerida:
//    test/
//    ├── integration_tests.rs    <- Este archivo
//    ├── programs/               <- Programas de prueba (.hulk files)
//    │   ├── valid/             <- Programas válidos
//    │   └── invalid/           <- Programas con errores esperados
//    └── expected/              <- Salidas esperadas para snapshots
//
// =============================================================================

// Los tests de integración irán aquí
