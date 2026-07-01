use crate::ir::IRModule;

use super::artifact::CodegenArtifact;
use super::context::CodegenContext;
use super::error::CodegenResult;

pub trait CodegenBackend {
    fn backend_name(&self) -> &'static str;

    fn emit_module(
        &self,
        module: &IRModule,
        context: &CodegenContext,
    ) -> CodegenResult<CodegenArtifact>;
}
