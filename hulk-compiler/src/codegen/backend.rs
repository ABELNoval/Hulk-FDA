use crate::ir::IRModule;

use super::artifact::CodegenArtifact;
use super::context::CodegenContext;
use super::error::CodegenResult;

// Person A owns this shared backend trait because it defines the contract
// that the lowering implementation and future backends must follow.
pub trait CodegenBackend {
    fn backend_name(&self) -> &'static str;

    fn emit_module(
        &self,
        module: &IRModule,
        context: &CodegenContext,
    ) -> CodegenResult<CodegenArtifact>;
}
