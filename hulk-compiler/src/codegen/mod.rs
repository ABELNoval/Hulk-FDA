// =============================================================================
// Codegen (Code Generation - Generación de Código)
// =============================================================================
//
// El módulo de generación de código es la fase final del compilador.
// Transforma la IR en código ejecutable para la plataforma objetivo.
//
// Responsabilidades:
// - Instruction Selection: elegir las instrucciones apropiadas
// - Register Allocation: asignar variables a registros
// - Instruction Scheduling: ordenar instrucciones óptimamente
// - Emisión del código final
//
// Posibles objetivos (backends):
// - Código máquina nativo (x86-64, ARM, RISC-V)
// - LLVM IR (para usar el backend de LLVM)
// - Bytecode para una máquina virtual
// - WebAssembly (WASM)
// - C code (transpilación)
// - Intérprete directo (tree-walking interpreter)
//
// Consideraciones:
// - Convenciones de llamada (calling conventions)
// - Manejo de memoria (stack, heap)
// - Alineación de datos
// - Generación de debug info
//
// =============================================================================
