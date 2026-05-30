use crate::ir::IRModule;

use super::artifact::CodegenArtifact;
use super::backend::CodegenBackend;
use super::context::CodegenContext;
use super::error::{CodegenError, CodegenResult};
use super::llvm::LlvmTextBackend;

/// Minimal `LlvmInkwellBackend` placeholder.
///
/// For now this backend delegates to the existing `LlvmTextBackend` implementation
/// so the integration point exists while we progressively replace the lowering
/// with direct Inkwell usage. The repository already declares `inkwell` as an
/// optional dependency in `Cargo.toml` behind the `llvm-verify` feature.
#[derive(Debug)]
pub struct LlvmInkwellBackend {
    inner: LlvmTextBackend,
}

impl LlvmInkwellBackend {
    pub fn new() -> Self {
        Self {
            inner: LlvmTextBackend::new(),
        }
    }
}

impl Default for LlvmInkwellBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CodegenBackend for LlvmInkwellBackend {
    fn backend_name(&self) -> &'static str {
        "llvm-inkwell"
    }

    fn emit_module(
        &self,
        module: &IRModule,
        context: &CodegenContext,
    ) -> CodegenResult<CodegenArtifact> {
        // TODO: Replace this delegation with a real inkwell-based lowering.
        // For now, reuse the textual backend so enabling the backend is
        // non-disruptive and preserves current behavior.
        self.inner.emit_module(module, context)
    }
}
