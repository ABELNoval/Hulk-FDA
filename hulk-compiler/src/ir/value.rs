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
    pub dependencies: Vec<IRValueId>,
}

impl IRValue {
    pub fn new(id: impl Into<String>, kind: IRValueKind) -> Self {
        Self {
            id: IRValueId::new(id),
            kind,
            ty: None,
            span: None,
            dependencies: Vec::new(),
        }
    }

    pub fn temporary(id: impl Into<String>) -> Self {
        Self::new(id, IRValueKind::Temporary)
    }

    pub fn parameter(id: impl Into<String>) -> Self {
        Self::new(id, IRValueKind::Parameter)
    }

    pub fn constant(id: impl Into<String>) -> Self {
        Self::new(id, IRValueKind::Constant)
    }

    pub fn named(id: impl Into<String>) -> Self {
        Self::new(id, IRValueKind::Named)
    }

    pub fn phi(id: impl Into<String>) -> Self {
        Self::new(id, IRValueKind::Phi)
    }

    pub fn with_type(mut self, ty: impl Into<String>) -> Self {
        self.ty = Some(ty.into());
        self
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn is_parameter(&self) -> bool {
        matches!(self.kind, IRValueKind::Parameter)
    }

    pub fn is_temporary(&self) -> bool {
        matches!(self.kind, IRValueKind::Temporary)
    }

    pub fn is_constant(&self) -> bool {
        matches!(self.kind, IRValueKind::Constant)
    }

    pub fn is_phi(&self) -> bool {
        matches!(self.kind, IRValueKind::Phi)
    }

    pub fn with_dependencies(mut self, deps: Vec<IRValueId>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn add_dependency(&mut self, dep: IRValueId) {
        if !self.dependencies.contains(&dep) {
            self.dependencies.push(dep);
        }
    }

    pub fn get_dependencies(&self) -> &[IRValueId] {
        &self.dependencies
    }

    pub fn depends_on(&self, value_id: &IRValueId) -> bool {
        self.dependencies.contains(value_id)
    }

    pub fn has_dependencies(&self) -> bool {
        !self.dependencies.is_empty()
    }

    pub fn dependency_count(&self) -> usize {
        self.dependencies.len()
    }

    pub fn fmt_display(&self) -> String {
        let kind_str = match self.kind {
            IRValueKind::Temporary => "temp",
            IRValueKind::Parameter => "param",
            IRValueKind::Constant => "const",
            IRValueKind::Phi => "phi",
            IRValueKind::Named => "named",
        };

        let type_str = self.ty.as_deref().unwrap_or("?");
        let dep_str = if self.dependencies.is_empty() {
            String::new()
        } else {
            let deps = self
                .dependencies
                .iter()
                .map(|d| d.0.clone())
                .collect::<Vec<_>>()
                .join(", ");
            format!(" [deps: {}]", deps)
        };

        format!("{}: {} ({}){}", self.id.0, kind_str, type_str, dep_str)
    }
}
