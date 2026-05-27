use super::context::CodegenTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenOutput {
    Text(String),
    Binary(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenArtifact {
    pub target: CodegenTarget,
    pub output: CodegenOutput,
}

// Person A owns the artifact type because it is the shared output wrapper
// produced by the backend and consumed by tests or later pipeline stages.
impl CodegenArtifact {
    pub fn text(target: CodegenTarget, text: impl Into<String>) -> Self {
        Self {
            target,
            output: CodegenOutput::Text(text.into()),
        }
    }

    pub fn binary(target: CodegenTarget, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            target,
            output: CodegenOutput::Binary(bytes.into()),
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match &self.output {
            CodegenOutput::Text(text) => Some(text.as_str()),
            CodegenOutput::Binary(_) => None,
        }
    }
}
