// Persona 1 — SSA IR design and core representation
// Assigned: Persona1
// Responsibilities: modelado de valores SSA, identificadores únicos y metadatos
// (tipos, span). Requisito para operaciones SSA y phi nodes.

use crate::utils::errors::span::Span;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IRValueId(pub String);

impl IRValueId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IRValueKind {
    Temporary,
    Parameter,
    Constant,
    Phi,
    Named,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRValue {
    pub id: IRValueId,
    pub kind: IRValueKind,
    pub ty: Option<String>,
    pub span: Option<Span>,
}

impl IRValue {
    pub fn new(id: impl Into<String>, kind: IRValueKind) -> Self {
        Self {
            id: IRValueId::new(id),
            kind,
            ty: None,
            span: None,
        }
    }

    pub fn with_type(mut self, ty: impl Into<String>) -> Self {
        self.ty = Some(ty.into());
        self
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}
