// Persona 1 — SSA IR design and core representation
// Assigned: Persona1
// Responsibilities: definir instrucciones, operandos y representación de operaciones
// aritméticas y de control (incluyendo phi nodes). Mantener compatibilidad con SSA.

use super::block::BasicBlockId;
use super::value::IRValueId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IRBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IRUnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IROperand {
    Value(IRValueId),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum IRInstructionKind {
    Assign {
        target: IRValueId,
        value: IROperand,
    },
    Binary {
        target: IRValueId,
        op: IRBinaryOp,
        left: IROperand,
        right: IROperand,
    },
    Unary {
        target: IRValueId,
        op: IRUnaryOp,
        operand: IROperand,
    },
    Phi {
        target: IRValueId,
        incoming: Vec<(IRValueId, BasicBlockId)>,
    },
    Jump {
        target: BasicBlockId,
    },
    Branch {
        condition: IROperand,
        then_block: BasicBlockId,
        else_block: BasicBlockId,
    },
    Return(Option<IROperand>),
    Call {
        target: Option<IRValueId>,
        callee: String,
        arguments: Vec<IROperand>,
    },
    Nop,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRInstruction {
    pub kind: IRInstructionKind,
}

impl IRInstruction {
    pub fn new(kind: IRInstructionKind) -> Self {
        Self { kind }
    }

    pub fn defines_value(&self) -> Option<&IRValueId> {
        match &self.kind {
            IRInstructionKind::Assign { target, .. }
            | IRInstructionKind::Binary { target, .. }
            | IRInstructionKind::Unary { target, .. }
            | IRInstructionKind::Phi { target, .. } => Some(target),
            IRInstructionKind::Call {
                target: Some(target),
                ..
            } => Some(target),
            _ => None,
        }
    }
}
