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
    Mod,
    Pow,
    Concat,
    Concatenate,
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
    Global(String),
}

impl IROperand {
    pub fn fmt_display(&self) -> String {
        match self {
            IROperand::Value(id) => id.0.clone(),
            IROperand::Integer(i) => i.to_string(),
            IROperand::Float(f) => f.to_string(),
            IROperand::Boolean(b) => b.to_string(),
            IROperand::Text(s) => format!("\"{}\"", s),
            IROperand::Global(s) => format!("@{}", s),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IRInstructionKind {
    Assign {
        target: IRValueId,
        value: IROperand,
        original: Option<String>,
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
        original: Option<String>,
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
        target: IRValueId,
        callee: String,
        arguments: Vec<IROperand>,
        original: Option<String>,
    },
    Nop,
    GetElementPtr {
        target: IRValueId,
        base: IROperand,
        indices: Vec<IROperand>,
        element_type: String,
    },
    Store {
        address: IROperand, // dónde guardar
        value: IROperand,   // qué guardar
    },
    Load {
        target: IRValueId,
        address: IROperand,
        ty: String,
    },
    CallIndirect {
        target: IRValueId,
        callee_ptr: IROperand,
        arguments: Vec<IROperand>,
        return_type: Option<String>,
        param_types: Vec<String>,
        original: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRInstruction {
    pub kind: IRInstructionKind,
}

impl IRInstruction {
    pub fn new(kind: IRInstructionKind) -> Self {
        Self { kind }
    }

    pub fn defines_value(&self) -> &IRValueId {
        match &self.kind {
            IRInstructionKind::Assign { target, .. }
            | IRInstructionKind::Binary { target, .. }
            | IRInstructionKind::Unary { target, .. }
            | IRInstructionKind::Phi { target, .. } => target,
            IRInstructionKind::Call { target, .. } => target,
            _ => panic!("Instruction does not define a value"),
        }
    }

    pub fn uses_value(&self, value_id: &IRValueId) -> bool {
        match &self.kind {
            IRInstructionKind::Assign { value, .. } => operand_uses_value(value, value_id),
            IRInstructionKind::Binary { left, right, .. } => {
                operand_uses_value(left, value_id) || operand_uses_value(right, value_id)
            }
            IRInstructionKind::Unary { operand, .. } => operand_uses_value(operand, value_id),
            IRInstructionKind::Phi { incoming, .. } => {
                incoming.iter().any(|(vid, _)| vid == value_id)
            }
            IRInstructionKind::Branch { condition, .. } => operand_uses_value(condition, value_id),
            IRInstructionKind::Return(Some(operand)) => operand_uses_value(operand, value_id),
            IRInstructionKind::Call { arguments, .. } => arguments
                .iter()
                .any(|arg| operand_uses_value(arg, value_id)),
            _ => false,
        }
    }

    pub fn used_values(&self) -> Vec<IRValueId> {
        let mut vals = Vec::new();
        match &self.kind {
            IRInstructionKind::Assign {
                value: IROperand::Value(id),
                ..
            } => {
                vals.push(id.clone());
            }
            IRInstructionKind::Binary { left, right, .. } => {
                if let IROperand::Value(l) = left {
                    vals.push(l.clone());
                }

                if let IROperand::Value(r) = right {
                    vals.push(r.clone());
                }
            }
            IRInstructionKind::Unary {
                operand: IROperand::Value(o),
                ..
            } => {
                vals.push(o.clone());
            }
            IRInstructionKind::Phi { incoming, .. } => {
                for (v, _) in incoming {
                    vals.push(v.clone());
                }
            }
            IRInstructionKind::Branch {
                condition: IROperand::Value(c),
                ..
            } => {
                vals.push(c.clone());
            }
            IRInstructionKind::Return(Some(IROperand::Value(v))) => {
                vals.push(v.clone());
            }
            IRInstructionKind::Call { arguments, .. } => {
                vals.extend(arguments.iter().filter_map(|arg| match arg {
                    IROperand::Value(a) => Some(a.clone()),
                    _ => None,
                }));
            }
            IRInstructionKind::CallIndirect {
                callee_ptr,
                arguments,
                ..
            } => {
                if let IROperand::Value(v) = callee_ptr {
                    vals.push(v.clone());
                }
                vals.extend(arguments.iter().filter_map(|arg| match arg {
                    IROperand::Value(a) => Some(a.clone()),
                    _ => None,
                }));
            }
            _ => {}
        }
        vals
    }

    pub fn is_terminator(&self) -> bool {
        matches!(
            &self.kind,
            IRInstructionKind::Jump { .. }
                | IRInstructionKind::Branch { .. }
                | IRInstructionKind::Return(_)
        )
    }

    pub fn is_arithmetic(&self) -> bool {
        matches!(
            &self.kind,
            IRInstructionKind::Binary { .. }
                | IRInstructionKind::Unary { .. }
                | IRInstructionKind::Assign { .. }
        )
    }

    pub fn is_control_flow(&self) -> bool {
        matches!(
            &self.kind,
            IRInstructionKind::Jump { .. }
                | IRInstructionKind::Branch { .. }
                | IRInstructionKind::Return(_)
        )
    }

    pub fn is_phi(&self) -> bool {
        matches!(&self.kind, IRInstructionKind::Phi { .. })
    }

    pub fn is_call(&self) -> bool {
        matches!(&self.kind, IRInstructionKind::Call { .. })
    }

    pub fn is_nop(&self) -> bool {
        matches!(&self.kind, IRInstructionKind::Nop)
    }

    pub fn successor_blocks(&self) -> Vec<&BasicBlockId> {
        match &self.kind {
            IRInstructionKind::Jump { target } => vec![target],
            IRInstructionKind::Branch {
                then_block,
                else_block,
                ..
            } => vec![then_block, else_block],
            _ => vec![],
        }
    }

    pub fn phi_incoming(&self) -> Option<&Vec<(IRValueId, BasicBlockId)>> {
        match &self.kind {
            IRInstructionKind::Phi { incoming, .. } => Some(incoming),
            _ => None,
        }
    }

    pub fn phi_from_block(&self, block_id: &BasicBlockId) -> Option<&IRValueId> {
        match &self.kind {
            IRInstructionKind::Phi { incoming, .. } => incoming
                .iter()
                .find(|(_, bid)| bid == block_id)
                .map(|(vid, _)| vid),
            _ => None,
        }
    }

    pub fn phi_incoming_count(&self) -> Option<usize> {
        match &self.kind {
            IRInstructionKind::Phi { incoming, .. } => Some(incoming.len()),
            _ => None,
        }
    }

    pub fn phi_blocks(&self) -> Option<Vec<&BasicBlockId>> {
        match &self.kind {
            IRInstructionKind::Phi { incoming, .. } => {
                Some(incoming.iter().map(|(_, bid)| bid).collect())
            }
            _ => None,
        }
    }

    pub fn validate_phi(&self) -> Result<(), String> {
        match &self.kind {
            IRInstructionKind::Phi {
                incoming,
                target,
                original: _,
            } => {
                if incoming.len() < 2 {
                    return Err(format!(
                        "Phi node {:?} must have at least 2 incoming values, has {}",
                        target,
                        incoming.len()
                    ));
                }

                for i in 0..incoming.len() {
                    for j in (i + 1)..incoming.len() {
                        if incoming[i].1 == incoming[j].1 {
                            return Err(format!(
                                "Phi node {:?} has duplicate incoming block: {:?}",
                                target, incoming[i].1
                            ));
                        }
                    }
                }

                Ok(())
            }
            _ => Err("Not a phi instruction".to_string()),
        }
    }

    pub fn fmt_display(&self) -> String {
        match &self.kind {
            IRInstructionKind::Assign {
                target,
                value,
                original: _,
            } => {
                format!("{} = {}", target.0, value.fmt_display())
            }
            IRInstructionKind::Binary {
                target,
                op,
                left,
                right,
            } => {
                let op_str = match op {
                    IRBinaryOp::Add => "+",
                    IRBinaryOp::Sub => "-",
                    IRBinaryOp::Mul => "*",
                    IRBinaryOp::Div => "/",
                    IRBinaryOp::Mod => "%",
                    IRBinaryOp::Pow => "^",
                    IRBinaryOp::Concat => "@",
                    IRBinaryOp::Concatenate => "@@",
                    IRBinaryOp::And => "&&",
                    IRBinaryOp::Or => "||",
                    IRBinaryOp::Eq => "==",
                    IRBinaryOp::Ne => "!=",
                    IRBinaryOp::Lt => "<",
                    IRBinaryOp::Le => "<=",
                    IRBinaryOp::Gt => ">",
                    IRBinaryOp::Ge => ">=",
                };
                format!(
                    "{} = {} {} {}",
                    target.0,
                    left.fmt_display(),
                    op_str,
                    right.fmt_display()
                )
            }
            IRInstructionKind::Unary {
                target,
                op,
                operand,
            } => {
                let op_str = match op {
                    IRUnaryOp::Neg => "-",
                    IRUnaryOp::Not => "!",
                };
                format!("{} = {}{}", target.0, op_str, operand.fmt_display())
            }
            IRInstructionKind::Phi {
                target,
                incoming,
                original: _,
            } => {
                let values = incoming
                    .iter()
                    .map(|(v, b)| format!("{}:{}", v.0, b.0))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} = phi({})", target.0, values)
            }
            IRInstructionKind::Jump { target } => {
                format!("jump {}", target.0)
            }
            IRInstructionKind::Branch {
                condition,
                then_block,
                else_block,
            } => {
                format!(
                    "br {} {} {}",
                    condition.fmt_display(),
                    then_block.0,
                    else_block.0
                )
            }
            IRInstructionKind::Return(operand) => match operand {
                Some(op) => format!("return {}", op.fmt_display()),
                None => "return".to_string(),
            },
            IRInstructionKind::Call {
                target,
                callee,
                arguments,
                original: _,
            } => {
                let args = arguments
                    .iter()
                    .map(|arg| arg.fmt_display())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} = call {}({})", target.0, callee, args)
            }
            IRInstructionKind::GetElementPtr {
                target,
                base,
                indices,
                ..
            } => {
                let idxs = indices
                    .iter()
                    .map(|i| i.fmt_display())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "{} = getelementptr {}, {}",
                    target.0,
                    base.fmt_display(),
                    idxs
                )
            }
            IRInstructionKind::Store { address, value } => {
                format!("store {} -> {}", value.fmt_display(), address.fmt_display())
            }
            IRInstructionKind::Load {
                target, address, ..
            } => {
                format!("{} = load {}", target.0, address.fmt_display())
            }
            IRInstructionKind::CallIndirect {
                target,
                callee_ptr,
                arguments,
                ..
            } => {
                let args = arguments
                    .iter()
                    .map(|a| a.fmt_display())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "{} = call_indirect {}({})",
                    target.0,
                    callee_ptr.fmt_display(),
                    args
                )
            }
            IRInstructionKind::Nop => "nop".to_string(),
        }
    }
}

fn operand_uses_value(operand: &IROperand, value_id: &IRValueId) -> bool {
    match operand {
        IROperand::Value(vid) => vid == value_id,
        _ => false,
    }
}
