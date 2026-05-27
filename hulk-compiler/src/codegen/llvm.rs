use crate::ir::{IRInstructionKind, IRModule, IROperand};

use super::artifact::CodegenArtifact;
use super::backend::CodegenBackend;
use super::context::{CodegenContext, CodegenTarget};
use super::error::{CodegenError, CodegenResult};

// Person B owns this file: it contains the LLVM IR lowering and emission
// logic that maps project IR into the LLVM text backend.
#[derive(Debug, Default, Clone, Copy)]
pub struct LlvmTextBackend;

impl LlvmTextBackend {
    pub fn new() -> Self {
        Self
    }

    fn render_operand(&self, operand: &IROperand) -> String {
        match operand {
            IROperand::Value(id) => format!("%{}", id.0),
            IROperand::Integer(value) => value.to_string(),
            IROperand::Float(value) => {
                // LLVM floats must be formatted in IEEE 754 hexadecimal to ensure precision
                format!("0x{:016X}", value.to_bits())
            }
            IROperand::Boolean(value) => {
                if *value {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            IROperand::Text(value) => format!("\"{}\"", value),
        }
    }

    fn render_type(&self, ty: Option<&str>) -> &'static str {
        match ty {
            Some("bool") => "i1",
            Some("f64") | Some("float") => "double",
            Some("string") | Some("str") => "ptr",
            Some("void") => "void",
            Some(_) | None => "i64",
        }
    }

    fn render_function(&self, module: &IRModule, function_name: &str) -> CodegenResult<String> {
        let function =
            module
                .function(function_name)
                .ok_or_else(|| CodegenError::BackendFailure {
                    message: format!("function '{}' disappeared while rendering", function_name),
                })?;

        let return_type = self.render_type(function.return_type.as_deref());
        let params = function
            .parameters
            .iter()
            .map(|param| format!("{} %{}", self.render_type(param.ty.as_deref()), param.id.0))
            .collect::<Vec<_>>()
            .join(", ");

        let mut result = format!("define {} @{}({}) {{\n", return_type, function.name, params);

        for block in &function.blocks {
            result.push_str(&format!("{}:\n", block.id.0));

            for instruction in &block.instructions {
                let line = match &instruction.kind {
                    IRInstructionKind::Assign { target, value, .. } => {
                        // TODO: Support precise types. Defaulting to i64 unless Boolean.
                        let ty = match value {
                            IROperand::Boolean(_) => "i1",
                            IROperand::Float(_) => "double",
                            IROperand::Text(_) => "ptr",
                            _ => "i64",
                        };
                        let base_name = target.0.trim_start_matches('%');
                        let addr = format!("%{}.addr", base_name);
                        format!(
                            "  {} = alloca {}\n  store {} {}, ptr {}\n  {} = load {}, ptr {}",
                            addr,
                            ty,
                            ty,
                            self.render_operand(value),
                            addr,
                            target.0,
                            ty,
                            addr
                        )
                    }
                    IRInstructionKind::Binary {
                        target,
                        op,
                        left,
                        right,
                    } => {
                        let op_name = match op {
                            crate::ir::IRBinaryOp::Add => "add",
                            crate::ir::IRBinaryOp::Sub => "sub",
                            crate::ir::IRBinaryOp::Mul => "mul",
                            crate::ir::IRBinaryOp::Div => "sdiv",
                            crate::ir::IRBinaryOp::And => "and",
                            crate::ir::IRBinaryOp::Or => "or",
                            crate::ir::IRBinaryOp::Eq => "icmp eq",
                            crate::ir::IRBinaryOp::Ne => "icmp ne",
                            crate::ir::IRBinaryOp::Lt => "icmp slt",
                            crate::ir::IRBinaryOp::Le => "icmp sle",
                            crate::ir::IRBinaryOp::Gt => "icmp sgt",
                            crate::ir::IRBinaryOp::Ge => "icmp sge",
                        };
                        let ty = match op {
                            crate::ir::IRBinaryOp::And | crate::ir::IRBinaryOp::Or => "i1",
                            _ => "i64", // default arithmetic type
                        };
                        format!(
                            "  {} = {} {} {}, {}",
                            target.0,
                            op_name,
                            ty,
                            self.render_operand(left),
                            self.render_operand(right)
                        )
                    }
                    IRInstructionKind::Unary {
                        target,
                        op,
                        operand,
                    } => {
                        match op {
                            crate::ir::IRUnaryOp::Neg => {
                                format!("  {} = sub i64 0, {}", target.0, self.render_operand(operand))
                            }
                            crate::ir::IRUnaryOp::Not => {
                                format!("  {} = xor i1 true, {}", target.0, self.render_operand(operand))
                            }
                        }
                    }
                    IRInstructionKind::Phi {
                        target, incoming, ..
                    } => {
                        let values = incoming
                            .iter()
                            .map(|(value, block)| {
                                format!(
                                    "[ {}, %{} ]",
                                    self.render_operand(&IROperand::Value(value.clone())),
                                    block.0
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("  ; {} = phi {}", target.0, values)
                    }
                    IRInstructionKind::Jump { target } => format!("  br label %{}", target.0),
                    IRInstructionKind::Branch {
                        condition,
                        then_block,
                        else_block,
                    } => format!(
                        "  br i1 {}, label %{}, label %{}",
                        self.render_operand(condition),
                        then_block.0,
                        else_block.0
                    ),
                    IRInstructionKind::Return(Some(operand)) => {
                        let ty = self.render_type(function.return_type.as_deref());
                        format!("  ret {} {}", ty, self.render_operand(operand))
                    }
                    IRInstructionKind::Return(None) => "  ret void".to_string(),
                    IRInstructionKind::Call {
                        target,
                        callee,
                        arguments,
                        ..
                    } => {
                        let ret_ty = module
                            .function(callee)
                            .map(|f| self.render_type(f.return_type.as_deref()))
                            .unwrap_or("i64");

                        let args = arguments
                            .iter()
                            .map(|arg| {
                                let arg_ty = match arg {
                                    IROperand::Boolean(_) => "i1",
                                    IROperand::Float(_) => "double",
                                    IROperand::Text(_) => "ptr",
                                    _ => "i64",
                                };
                                format!("{} {}", arg_ty, self.render_operand(arg))
                            })
                            .collect::<Vec<_>>()
                            .join(", ");

                        match target {
                            Some(value) => format!("  {} = call {} @{}({})", value.0, ret_ty, callee, args),
                            None => format!("  call {} @{}({})", ret_ty, callee, args),
                        }
                    }
                    IRInstructionKind::Nop => "  ; nop".to_string(),
                };

                result.push_str(&line);
                result.push('\n');
            }

            result.push_str("}\n\n");
        }

        Ok(result)
    }
}

impl CodegenBackend for LlvmTextBackend {
    fn backend_name(&self) -> &'static str {
        "llvm-text"
    }

    fn emit_module(
        &self,
        module: &IRModule,
        context: &CodegenContext,
    ) -> CodegenResult<CodegenArtifact> {
        if context.target != CodegenTarget::LlvmIr {
            return Err(CodegenError::BackendFailure {
                message: format!(
                    "backend '{}' only supports LLVM IR text output",
                    self.backend_name()
                ),
            });
        }

        module
            .validate()
            .map_err(|errors| CodegenError::InvalidModule {
                module: module.name.clone(),
                message: errors.join("; "),
            })?;

        let mut output = String::new();
        output.push_str(&format!("; ModuleID = '{}'\n", context.module_name));

        if let Some(triple) = &context.target_triple {
            output.push_str(&format!("target triple = \"{}\"\n", triple));
        }

        output.push('\n');

        for function in &module.functions {
            output.push_str(&self.render_function(module, &function.name)?);
        }

        Ok(CodegenArtifact::text(CodegenTarget::LlvmIr, output))
    }
}
