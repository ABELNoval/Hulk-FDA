use crate::ir::{IRInstructionKind, IRModule, IROperand};

use super::artifact::CodegenArtifact;
use super::backend::CodegenBackend;
use super::context::{CodegenContext, CodegenTarget, LlvmBuilder, LlvmContext, LlvmModule};
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
        module
            .verify_context(&self.context)
            .map_err(|e| CodegenError::BackendFailure { message: e })?;

        Ok(LlvmBuilder::new(module))
    }

    /// Validate that a module is properly constructed before emission.
    pub fn validate_module(&self, module: &LlvmModule) -> CodegenResult<()> {
        module
            .verify_context(&self.context)
            .map_err(|e| CodegenError::BackendFailure { message: e })?;

        if module.symbols().is_empty() {
            return Err(CodegenError::InvalidModule {
                module: module.name().to_string(),
                message: "module has no symbols; at least one function expected".to_string(),
            });
        }

        Ok(())
    }

    /// Validate module consistency against the IRModule and (optionally) an external verifier.
    /// This performs cross-checks between prototypes, runtime declarations and IR call sites.
    pub fn validate_full_module(
        &self,
        llvm_module: &LlvmModule,
        ir_module: &crate::ir::IRModule,
    ) -> CodegenResult<()> {
        // Basic context check
        llvm_module
            .verify_context(&self.context)
            .map_err(|e| CodegenError::BackendFailure { message: e })?;

        // Ensure there is at least one symbol defined (same as before)
        if llvm_module.symbols().is_empty() {
            return Err(CodegenError::InvalidModule {
                module: llvm_module.name().to_string(),
                message: "module has no symbols; at least one function expected".to_string(),
            });
        }

        // Cross-check call sites: parameter counts
        for func in &ir_module.functions {
            for block in &func.blocks {
                for instr in &block.instructions {
                    if let crate::ir::IRInstructionKind::Call {
                        callee, arguments, ..
                    } = &instr.kind
                    {
                        // prefer symbols -> prototypes -> runtime_decls
                        if let Some((_, params)) = llvm_module.symbols().get(callee) {
                            if params.len() != arguments.len() {
                                return Err(CodegenError::InvalidModule {
                                    module: llvm_module.name().to_string(),
                                    message: format!(
                                        "Call to '{}' has {} args but definition has {} parameters",
                                        callee,
                                        arguments.len(),
                                        params.len()
                                    ),
                                });
                            }
                        } else if let Some((_, params, _)) = llvm_module.prototypes().get(callee) {
                            if params.len() != arguments.len() {
                                return Err(CodegenError::InvalidModule {
                                    module: llvm_module.name().to_string(),
                                    message: format!(
                                        "Call to '{}' has {} args but prototype has {} parameters",
                                        callee,
                                        arguments.len(),
                                        params.len()
                                    ),
                                });
                            }
                        } else if llvm_module.runtime_decls().contains_key(callee) {
                            // runtime decl exists; assume correct
                        } else {
                            return Err(CodegenError::InvalidModule {
                                module: llvm_module.name().to_string(),
                                message: format!("Call to unknown symbol '{}'", callee),
                            });
                        }
                    }
                }
            }
        }

        // Placeholder for hooking LLVM verifier via feature flag
        #[cfg(feature = "llvm-verify")]
        {
            // If the project enables `llvm-verify`, the actual verification using
            // LLVM's verifier can be performed here by parsing the emitted LLVM IR
            // and calling `module.verify()` through a binding like `inkwell`.
            // Implementing that requires enabling the feature and ensuring LLVM is
            // present on the system. See README or follow-up task to enable.
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

    /// Emit the module and write the textual LLVM IR to `path`.
    pub fn emit_module_to_path(
        &self,
        module: &crate::ir::IRModule,
        context: &CodegenContext,
        path: impl AsRef<std::path::Path>,
    ) -> CodegenResult<()> {
        let artifact = self.emit_module(module, context)?;
        artifact
            .write_to_file(path)
            .map_err(|e| CodegenError::BackendFailure {
                message: format!("could not write artifact: {}", e),
            })?;
        Ok(())
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

    fn render_type(&self, language_type: Option<&str>) -> String {
        Self::llvm_type_for(language_type)
    }

    fn render_operand(&self, operand: &IROperand) -> String {
        match operand {
            IROperand::Value(id) => format!("%{}", id.0.trim_start_matches('%')),
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
            .map(|param| {
                format!(
                    "{} %{}",
                    Self::llvm_type_for(param.ty.as_deref()),
                    param.id.0
                )
            })
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
                    } => match op {
                        crate::ir::IRUnaryOp::Neg => {
                            format!(
                                "  {} = sub i64 0, {}",
                                target.0,
                                self.render_operand(operand)
                            )
                        }
                        crate::ir::IRUnaryOp::Not => {
                            format!(
                                "  {} = xor i1 true, {}",
                                target.0,
                                self.render_operand(operand)
                            )
                        }
                    },
                    IRInstructionKind::Phi {
                        target, incoming, ..
                    } => {
                        if incoming.is_empty() {
                            return Err(CodegenError::UnsupportedInstruction {
                                function: function.name.clone(),
                                block: block.id.0.clone(),
                                message: format!(
                                    "phi node for target '{}' has no incoming edges",
                                    target.0
                                ),
                            });
                        }

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
                        // TODO: Support precise types. Defaulting to i64.
                        format!("  {} = phi i64 {}", target.0, values)
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
                            .unwrap_or_else(|| "i64".to_string());

                        if target.is_some() && ret_ty == "void" {
                            return Err(CodegenError::UnsupportedInstruction {
                                function: function.name.clone(),
                                block: block.id.0.clone(),
                                message: format!(
                                    "cannot assign result of void function '{}'",
                                    callee
                                ),
                            });
                        }

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
                            Some(value) => {
                                format!("  {} = call {} @{}({})", value.0, ret_ty, callee, args)
                            }
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
        let mut llvm_module = self.lifecycle.create_module(&context.module_name)?;

        // First, scan the IR for external call sites and infer prototypes.
        use crate::ir::IRInstructionKind;
        let infer_operand_ty = |op: &IROperand| -> String {
            match op {
                IROperand::Integer(_) => "i64".to_string(),
                IROperand::Float(_) => "double".to_string(),
                IROperand::Boolean(_) => "i1".to_string(),
                IROperand::Text(_) => "ptr".to_string(),
                IROperand::Value(_) => "i64".to_string(),
            }
        };

        let mut extern_prototypes: std::collections::HashMap<String, (String, Vec<String>)> =
            std::collections::HashMap::new();

        for func in &module.functions {
            for block in &func.blocks {
                for instr in &block.instructions {
                    if let IRInstructionKind::Call {
                        callee, arguments, ..
                    } = &instr.kind
                    {
                        if module.function(callee).is_some() {
                            continue;
                        }
                        if llvm_module.runtime_decls().contains_key(callee) {
                            continue;
                        }

                        let params = arguments.iter().map(infer_operand_ty).collect::<Vec<_>>();
                        let ret = if params.iter().any(|p| p == "double") {
                            "double".to_string()
                        } else {
                            "i64".to_string()
                        };

                        extern_prototypes
                            .entry(callee.clone())
                            .or_insert((ret, params));
                    }
                }
            }
        }

        // Register extern prototypes into the LlvmModule first.
        for (name, (ret, params)) in &extern_prototypes {
            let _ = llvm_module.declare_symbol(name, ret.clone(), params.clone());
        }

        // Now mark defined functions in the LlvmModule (so prototypes become 'defined')
        for function in &module.functions {
            // construct signature
            let return_type = Self::llvm_type_for(function.return_type.as_deref());
            let param_types = function
                .parameters
                .iter()
                .map(|p| Self::llvm_type_for(p.ty.as_deref()))
                .collect::<Vec<_>>();

            // define symbol in module; propagate errors upward
            llvm_module
                .define_symbol(&function.name, return_type, param_types)
                .map_err(|e| CodegenError::BackendFailure { message: e })?;
        }

        // Begin emitting output header and declarations
        let mut output = String::new();
        output.push_str(&format!("; ModuleID = '{}'\n", context.module_name));
        output.push_str(&format!(
            "target triple = \"{}\"\n",
            context.resolved_target_triple()
        ));
        output.push_str(&format!(
            "target datalayout = \"{}\"\n",
            context.resolved_data_layout()
        ));
        output.push('\n');

        // Emit runtime declarations first (declare ...)
        for (name, (ret_ty, params)) in llvm_module.runtime_decls() {
            let params_joined = params.join(", ");
            output.push_str(&format!(
                "declare {} @{}({})\n",
                ret_ty, name, params_joined
            ));
        }

        output.push('\n');

        // Emit declared external prototypes that remain undefined
        for (name, (ret_ty, params, defined)) in llvm_module.prototypes() {
            if *defined {
                continue;
            }
            let params_joined = params.join(", ");
            output.push_str(&format!(
                "declare {} @{}({})\n",
                ret_ty, name, params_joined
            ));
        }

        output.push('\n');

        // Finally emit function definitions from the IR
        for function in &module.functions {
            output.push_str(&self.render_function(module, &function.name)?);
        }

        // Validate module consistency against the IR
        self.lifecycle
            .validate_full_module(&llvm_module, module)
            .map_err(|e| CodegenError::InvalidModule {
                module: module.name.clone(),
                message: format!("validation failed: {}", e),
            })?;

        Ok(CodegenArtifact::text(CodegenTarget::LlvmIr, output))
    }
}
