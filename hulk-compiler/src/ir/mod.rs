// =============================================================================
// IR (Intermediate Representation - Representación Intermedia)
// =============================================================================
//
// Este módulo expone la API compartida de la IR para SSA, CFG y lowering.
// La idea es separar responsabilidades para que cada frente tenga un punto
// claro de extensión sin concentrar todo en un único archivo.
//
// =============================================================================

pub mod block;
pub mod instruction;
pub mod lowering;
pub mod module;
pub mod naming;
pub mod value;

#[cfg(test)]
pub mod test_support;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use block::{BasicBlock, BasicBlockId, ControlFlowGraph};
#[allow(unused_imports)]
pub use instruction::{IRBinaryOp, IRInstruction, IRInstructionKind, IROperand, IRUnaryOp};
#[allow(unused_imports)]
pub use lowering::{IRLoweringContext, IRLoweringError, IRLoweringResult};
#[allow(unused_imports)]
pub use module::{IRFunction, IRModule};
#[allow(unused_imports)]
pub use naming::IRNaming;
#[allow(unused_imports)]
pub use value::{IRValue, IRValueId, IRValueKind};
