#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenTarget {
    LlvmIr,
    LlvmBitcode,
    Native,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenContext {
    pub module_name: String,
    pub target: CodegenTarget,
    pub emit_comments: bool,
    pub target_triple: Option<String>,
}

// Person A owns this shared configuration because it is the common setup
// layer that backend and tests both rely on.
impl CodegenContext {
    pub fn new(module_name: impl Into<String>, target: CodegenTarget) -> Self {
        Self {
            module_name: module_name.into(),
            target,
            emit_comments: true,
            target_triple: None,
        }
    }

    pub fn with_target_triple(mut self, triple: impl Into<String>) -> Self {
        self.target_triple = Some(triple.into());
        self
    }

    pub fn without_comments(mut self) -> Self {
        self.emit_comments = false;
        self
    }
}
