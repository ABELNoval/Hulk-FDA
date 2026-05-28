use crate::ir::{IRInstructionKind, IRModule, IROperand};

use super::artifact::CodegenArtifact;
use super::backend::CodegenBackend;
use super::context::{CodegenContext, CodegenTarget, LlvmContext, LlvmModule, LlvmBuilder};
use super::error::{CodegenError, CodegenResult};

// =============================================================================
// LLVM Lifecycle Management Module
// =============================================================================
//
// This module provides safe lifecycle management for LLVM compilation.
// It ensures proper initialization, configuration, and cleanup of contexts,
// modules, and builders.
//

/// Manages the lifecycle of LLVM compilation components.
/// Provides high-level operations for context creation, module setup, and cleanup.
#[derive(Debug)]
pub struct LlvmLifecycle {
    context: LlvmContext,
}

impl LlvmLifecycle {
    /// Create a new LLVM lifecycle manager.
    pub fn new() -> Self {
        Self {
            context: LlvmContext::new(),
        }
    }

    /// Get the LLVM context.
    pub fn context(&self) -> &LlvmContext {
        &self.context
    }

    /// Create a new module within this context.
    pub fn create_module(&self, name: impl Into<String>) -> CodegenResult<LlvmModule> {
        let mut m = LlvmModule::new(&self.context, name);
        // populate default runtime declarations so lowering can rely on them
        m.add_default_runtime_decls();
        Ok(m)
    }

    /// Create a builder for module construction.
    pub fn create_builder(&self, module: LlvmModule) -> CodegenResult<LlvmBuilder> {
        module.verify_context(&self.context)
            .map_err(|e| CodegenError::BackendFailure { message: e })?;

        Ok(LlvmBuilder::new(module))
    }

    /// Validate that a module is properly constructed before emission.
    pub fn validate_module(&self, module: &LlvmModule) -> CodegenResult<()> {
        module.verify_context(&self.context)
            .map_err(|e| CodegenError::BackendFailure { message: e })?;

        if module.symbols().is_empty() {
            return Err(CodegenError::InvalidModule {
                module: module.name().to_string(),
                message: "module has no symbols; at least one function expected".to_string(),
            });
        }

        Ok(())
    }
}

impl Default for LlvmLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// LLVM Text Backend
// =============================================================================
//
// The text backend emits LLVM IR as human-readable text.
// It uses the LlvmLifecycle manager to ensure proper setup.
//

// Person B owns this file: it contains the LLVM IR lowering and emission
// logic that maps project IR into the LLVM text backend.
#[derive(Debug)]
pub struct LlvmTextBackend {
    lifecycle: LlvmLifecycle,
}

impl LlvmTextBackend {
    pub fn new() -> Self {
        Self {
            lifecycle: LlvmLifecycle::new(),
        }
    }

    pub fn lifecycle(&self) -> &LlvmLifecycle {
        &self.lifecycle
    }

    /// Convert a compiler type name into the LLVM spelling used in function
    /// signatures and runtime declarations.
    pub(crate) fn llvm_type_for(language_type: Option<&str>) -> String {
        let Some(raw_type) = language_type.map(str::trim).filter(|ty| !ty.is_empty()) else {
            return "i64".to_string();
        };

        let lowered = raw_type.to_ascii_lowercase();

        match lowered.as_str() {
            "number" | "float" | "f64" | "double" => "double".to_string(),
            "boolean" | "bool" => "i1".to_string(),
            "string" | "str" | "text" => "ptr".to_string(),
            "void" => "void".to_string(),
            "i8" | "i16" | "i32" | "i64" => lowered,
            "u8" | "u16" | "u32" | "u64" | "int" | "integer" | "isize" | "usize" => {
                "i64".to_string()
            }
            "ptr" | "pointer" => "ptr".to_string(),
            _ if lowered.starts_with('i')
                && lowered.len() > 1
                && lowered[1..].chars().all(|ch| ch.is_ascii_digit()) =>
            {
                lowered
            }
            _ => "ptr".to_string(),
        }
    }

    fn render_operand(&self, operand: &IROperand) -> String {
        match operand {
            IROperand::Value(id) => format!("%{}", id.0),
            IROperand::Integer(value) => value.to_string(),
            IROperand::Float(value) => value.to_string(),
            IROperand::Boolean(value) => {
                if *value {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            IROperand::Text(value) => format!("\"{}\"", value),
        }
    }

    fn render_function(&self, module: &IRModule, function_name: &str) -> CodegenResult<String> {
        let function =
            module
                .function(function_name)
                .ok_or_else(|| CodegenError::BackendFailure {
                    message: format!("function '{}' disappeared while rendering", function_name),
                })?;

        let return_type = Self::llvm_type_for(function.return_type.as_deref());
        let params = function
            .parameters
            .iter()
            .map(|param| format!("{} %{}", Self::llvm_type_for(param.ty.as_deref()), param.id.0))
            .collect::<Vec<_>>()
            .join(", ");

        let mut result = format!("define {} @{}({}) {{\n", return_type, function.name, params);

        for block in &function.blocks {
            result.push_str(&format!("{}:\n", block.id.0));

            for instruction in &block.instructions {
                let line = match &instruction.kind {
                    IRInstructionKind::Assign { target, value, .. } => {
                        format!("  ; {} = {}", target.0, self.render_operand(value))
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
                        format!(
                            "  ; {} = {} {}, {}",
                            target.0,
                            op_name,
                            self.render_operand(left),
                            self.render_operand(right)
                        )
                    }
                    IRInstructionKind::Unary {
                        target,
                        op,
                        operand,
                    } => {
                        let op_name = match op {
                            crate::ir::IRUnaryOp::Neg => "sub",
                            crate::ir::IRUnaryOp::Not => "xor",
                        };
                        format!(
                            "  ; {} = {} {}",
                            target.0,
                            op_name,
                            self.render_operand(operand)
                        )
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
                        let ty = match operand {
                            IROperand::Boolean(_) => "i1",
                            IROperand::Float(_) => "double",
                            IROperand::Text(_) => "ptr",
                            _ => "i64",
                        };
                        format!("  ret {} {}", ty, self.render_operand(operand))
                    }
                    IRInstructionKind::Return(None) => "  ret void".to_string(),
                    IRInstructionKind::Call {
                        target,
                        callee,
                        arguments,
                        ..
                    } => {
                        let args = arguments
                            .iter()
                            .map(|arg| self.render_operand(arg))
                            .collect::<Vec<_>>()
                            .join(", ");
                        match target {
                            Some(value) => format!("  ; {} = call @{}({})", value.0, callee, args),
                            None => format!("  ; call @{}({})", callee, args),
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

impl Default for LlvmTextBackend {
    fn default() -> Self {
        Self::new()
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

        // create an LLVM module abstraction for declarations and lifecycle
        let llvm_module = self.lifecycle.create_module(&context.module_name)?;

        let mut output = String::new();
        output.push_str(&format!("; ModuleID = '{}'\n", context.module_name));
        output.push_str(&format!("target triple = \"{}\"\n", context.resolved_target_triple()));
        output.push_str(&format!("target datalayout = \"{}\"\n", context.resolved_data_layout()));
        output.push('\n');

        // Emit runtime declarations first (declare ...)
        for (name, (ret_ty, params)) in llvm_module.runtime_decls() {
            let params_joined = params.join(", ");
            output.push_str(&format!("declare {} @{}({})\n", ret_ty, name, params_joined));
        }

        output.push('\n');

        // Then emit function definitions from the IR
        for function in &module.functions {
            output.push_str(&self.render_function(module, &function.name)?);
        }

        Ok(CodegenArtifact::text(CodegenTarget::LlvmIr, output))
    }
}
