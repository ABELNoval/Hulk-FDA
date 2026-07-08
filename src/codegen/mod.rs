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
// - Emisión de LLVM IR
//
// Consideraciones:
// - Convenciones de llamada (calling conventions)
// - Manejo de memoria (stack, heap)
// - Alineación de datos
// - Generación de debug info
//
// =============================================================================
// Ownership split:
// - Person A: backend contract, shared context, and output artifacts.
// - Person B: LLVM lowering and emission implementation.

pub mod artifact;
pub mod backend;
pub mod context;
pub mod error;
pub mod inkwell;

pub use artifact::{CodegenArtifact, CodegenOutput};
pub use backend::CodegenBackend;
pub use context::{CodegenContext, CodegenTarget};
pub use error::{CodegenError, CodegenResult};
pub use inkwell::LlvmInkwellBackend;
