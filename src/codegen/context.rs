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
    pub data_layout: Option<String>,
}

impl CodegenContext {
    pub const DEFAULT_TARGET_TRIPLE: &'static str = "x86_64-unknown-linux-gnu";
    pub const DEFAULT_DATA_LAYOUT: &'static str =
        "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128";

    pub fn new(module_name: impl Into<String>, target: CodegenTarget) -> Self {
        Self {
            module_name: module_name.into(),
            target,
            emit_comments: true,
            target_triple: None,
            data_layout: None,
        }
    }

    pub fn with_target_triple(mut self, triple: impl Into<String>) -> Self {
        self.target_triple = Some(triple.into());
        self
    }

    pub fn with_data_layout(mut self, layout: impl Into<String>) -> Self {
        self.data_layout = Some(layout.into());
        self
    }

    pub fn resolved_target_triple(&self) -> &str {
        self.target_triple
            .as_deref()
            .unwrap_or(Self::DEFAULT_TARGET_TRIPLE)
    }

    pub fn resolved_data_layout(&self) -> &str {
        self.data_layout
            .as_deref()
            .unwrap_or(Self::DEFAULT_DATA_LAYOUT)
    }

    pub fn without_comments(mut self) -> Self {
        self.emit_comments = false;
        self
    }
}
