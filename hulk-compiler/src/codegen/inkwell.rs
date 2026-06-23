use crate::ir::IRModule;

use super::artifact::CodegenArtifact;
use super::backend::CodegenBackend;
use super::context::CodegenContext;
use super::error::{CodegenError, CodegenResult};

// Two implementations:
// - When the feature `llvm-verify` is enabled we use `inkwell` to emit a real
//   LLVM module and return textual IR/bitcode as requested.
// - When the feature is not enabled we keep a small shim that returns an error
//   if used (so the crate compiles without the optional dependency).

#[cfg(not(feature = "llvm-verify"))]
#[derive(Debug)]
pub struct LlvmInkwellBackend;

#[cfg(not(feature = "llvm-verify"))]
impl LlvmInkwellBackend {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(feature = "llvm-verify"))]
impl Default for LlvmInkwellBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "llvm-verify"))]
impl CodegenBackend for LlvmInkwellBackend {
    fn backend_name(&self) -> &'static str {
        "llvm-inkwell (disabled)"
    }

    fn emit_module(
        &self,
        _module: &IRModule,
        _context: &CodegenContext,
    ) -> CodegenResult<CodegenArtifact> {
        Err(CodegenError::BackendFailure {
            message: "inkwell backend is not enabled; build with feature 'llvm-verify'".to_string(),
        })
    }
}

// --- Real inkwell implementation ---
#[cfg(feature = "llvm-verify")]
mod real {
    use super::{
        CodegenArtifact, CodegenBackend, CodegenContext, CodegenError, CodegenResult, IRModule,
    };
    use crate::codegen::CodegenTarget;
    use inkwell::builder::Builder;
    use inkwell::context::Context;
    use inkwell::module::Module;
    use inkwell::targets::{TargetData, TargetTriple};
    use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
    use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, PointerValue, ValueKind};
    use inkwell::{AddressSpace, FloatPredicate, IntPredicate};
    use std::collections::HashMap;

    #[derive(Debug)]
    pub struct LlvmInkwellBackend;

    impl LlvmInkwellBackend {
        pub fn new() -> Self {
            Self {}
        }
    }

    impl Default for LlvmInkwellBackend {
        fn default() -> Self {
            Self::new()
        }
    }

    fn ensure_string<'ctx>(
        ctx: &'ctx Context,
        llvm_mod: &Module<'ctx>,
        builder: &Builder<'ctx>,
        val: BasicValueEnum<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        if val.is_float_value() {
            let num_to_str_fn = match llvm_mod.get_function("hulk_num_to_str") {
                Some(f) => f,

                None => {
                    let signature = ctx
                        .ptr_type(AddressSpace::default())
                        .fn_type(&[ctx.f64_type().into()], false);

                    llvm_mod.add_function("hulk_num_to_str", signature, None)
                }
            };

            let call = builder
                .build_call(num_to_str_fn, &[val.into()], "num_cast")
                .map_err(|e| CodegenError::BackendFailure {
                    message: e.to_string(),
                })?;

            match call.try_as_basic_value() {
                ValueKind::Basic(v) => Ok(v),

                _ => Err(CodegenError::BackendFailure {
                    message: "hulk_num_to_str no devolvió un valor".to_string(),
                }),
            }
        } else {
            Ok(val)
        }
    }

    fn map_type<'ctx>(ctx: &'ctx Context, ty: Option<&str>) -> Option<BasicTypeEnum<'ctx>> {
        let i64_t = ctx.i64_type();
        let f64_t = ctx.f64_type();
        let bool_t = ctx.bool_type();

        let Some(raw) = ty.map(str::trim).filter(|s| !s.is_empty()) else {
            return Some(f64_t.into());
        };

        match raw.to_ascii_lowercase().as_str() {
            "number" | "float" | "f64" | "double" => Some(f64_t.into()),
            "boolean" | "bool" => Some(bool_t.into()),
            "string" | "str" | "text" | "ptr" | "pointer" => {
                Some(ctx.ptr_type(AddressSpace::default()).into())
            }
            "void" => None,
            "i8" => Some(ctx.i8_type().into()),
            "i16" => Some(ctx.i16_type().into()),
            "i32" => Some(ctx.i32_type().into()),
            "i64" => Some(i64_t.into()),
            other if other.starts_with('i') && other[1..].chars().all(|c| c.is_ascii_digit()) => {
                // simple parse: i128, etc. fallback to i64 for simplicity
                Some(i64_t.into())
            }
            _ => Some(ctx.ptr_type(AddressSpace::default()).into()),
        }
    }

    fn basic_metadata_types<'ctx>(
        types: &[BasicTypeEnum<'ctx>],
    ) -> Vec<BasicMetadataTypeEnum<'ctx>> {
        types.iter().copied().map(Into::into).collect()
    }

    fn basic_metadata_values<'ctx>(
        values: &[BasicValueEnum<'ctx>],
    ) -> Vec<BasicMetadataValueEnum<'ctx>> {
        values.iter().copied().map(Into::into).collect()
    }

    fn emit_backend_failure<T>(message: impl Into<String>) -> CodegenResult<T> {
        Err(CodegenError::BackendFailure {
            message: message.into(),
        })
    }

    fn lower_operand<'ctx>(
        ctx: &'ctx Context,
        builder: &inkwell::builder::Builder<'ctx>,
        allocas: &HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>,
        operand: &crate::ir::IROperand,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        Ok(match operand {
            crate::ir::IROperand::Integer(value) => {
                ctx.i64_type().const_int(*value as u64, true).into()
            }
            crate::ir::IROperand::Float(value) => ctx.f64_type().const_float(*value).into(),
            crate::ir::IROperand::Boolean(value) => {
                ctx.bool_type().const_int(u64::from(*value), false).into()
            }
            crate::ir::IROperand::Text(text) => builder
                .build_global_string_ptr(text, "strlit")
                .map(|global| global.as_pointer_value().into())
                .map_err(|err| CodegenError::BackendFailure {
                    message: format!("failed to emit string literal: {:?}", err),
                })?,
            crate::ir::IROperand::Value(value_id) => {
                let raw_key = value_id.0.as_str();
                let trimmed_key = raw_key.trim_start_matches('%');
                let prefixed_key = format!("%{}", trimmed_key);
                let slot = allocas
                    .get(raw_key)
                    .or_else(|| allocas.get(trimmed_key))
                    .or_else(|| allocas.get(prefixed_key.as_str()));

                if let Some((ptr, ty)) = slot {
                    builder.build_load(*ty, *ptr, trimmed_key).map_err(|err| {
                        CodegenError::BackendFailure {
                            message: format!(
                                "failed to load '{}' (resolved as '{}'): {:?}",
                                value_id.0, trimmed_key, err
                            ),
                        }
                    })?
                } else {
                    return Err(CodegenError::BackendFailure {
                        message: format!("value '{}' not found in allocas", value_id.0),
                    });
                }
            }
        })
    }

    fn function_type_for<'ctx>(
        ctx: &'ctx Context,
        return_type: Option<&str>,
        parameter_types: &[BasicTypeEnum<'ctx>],
    ) -> inkwell::types::FunctionType<'ctx> {
        let metadata_params = basic_metadata_types(parameter_types);
        match return_type.map(str::trim).filter(|s| !s.is_empty()) {
            None => ctx.f64_type().fn_type(&metadata_params, false),
            Some("void") => ctx.void_type().fn_type(&metadata_params, false),
            Some(ret) => match map_type(ctx, Some(ret)) {
                Some(BasicTypeEnum::IntType(int_ty)) => int_ty.fn_type(&metadata_params, false),
                Some(BasicTypeEnum::FloatType(float_ty)) => {
                    float_ty.fn_type(&metadata_params, false)
                }
                Some(BasicTypeEnum::PointerType(ptr_ty)) => ptr_ty.fn_type(&metadata_params, false),
                Some(BasicTypeEnum::ArrayType(array_ty)) => {
                    array_ty.fn_type(&metadata_params, false)
                }
                Some(BasicTypeEnum::StructType(struct_ty)) => {
                    struct_ty.fn_type(&metadata_params, false)
                }
                Some(BasicTypeEnum::VectorType(vector_ty)) => {
                    vector_ty.fn_type(&metadata_params, false)
                }
                Some(BasicTypeEnum::ScalableVectorType(vector_ty)) => {
                    vector_ty.fn_type(&metadata_params, false)
                }
                None => ctx.void_type().fn_type(&metadata_params, false),
            },
        }
    }

    fn create_alloca<'ctx>(
        builder: &inkwell::builder::Builder<'ctx>,
        name: &str,
        ty: BasicTypeEnum<'ctx>,
    ) -> CodegenResult<PointerValue<'ctx>> {
        builder
            .build_alloca(ty, name)
            .map_err(|err| CodegenError::BackendFailure {
                message: format!("failed to allocate '{}': {:?}", name, err),
            })
    }

    fn ensure_slot<'ctx>(
        builder: &inkwell::builder::Builder<'ctx>,
        name: &str,
        ty: BasicTypeEnum<'ctx>,
        allocas: &mut HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>,
    ) -> CodegenResult<PointerValue<'ctx>> {
        let canonical = name.trim_start_matches('%');
        let prefixed = format!("%{}", canonical);

        if let Some((ptr, _)) = allocas
            .get(canonical)
            .or_else(|| allocas.get(prefixed.as_str()))
        {
            return Ok(*ptr);
        }

        let ptr = create_alloca(builder, canonical, ty)?;
        allocas.insert(canonical.to_string(), (ptr, ty));
        allocas.insert(prefixed, (ptr, ty));
        Ok(ptr)
    }

    fn infer_operand_type<'ctx>(
        ctx: &'ctx Context,
        allocas: &HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>,
        operand: &crate::ir::IROperand,
    ) -> BasicTypeEnum<'ctx> {
        match operand {
            crate::ir::IROperand::Integer(_) => ctx.i64_type().into(),
            crate::ir::IROperand::Float(_) => ctx.f64_type().into(),
            crate::ir::IROperand::Boolean(_) => ctx.bool_type().into(),
            crate::ir::IROperand::Text(_) => ctx.ptr_type(AddressSpace::default()).into(),
            crate::ir::IROperand::Value(value_id) => {
                let raw_key = value_id.0.as_str();
                let trimmed_key = raw_key.trim_start_matches('%');
                let prefixed_key = format!("%{}", trimmed_key);
                allocas
                    .get(raw_key)
                    .or_else(|| allocas.get(trimmed_key))
                    .or_else(|| allocas.get(prefixed_key.as_str()))
                    .map(|(_, ty)| *ty)
                    .unwrap_or_else(|| ctx.f64_type().into())
            }
        }
    }

    impl CodegenBackend for LlvmInkwellBackend {
        fn backend_name(&self) -> &'static str {
            "llvm-inkwell"
        }

        fn emit_module(
            &self,
            module: &IRModule,
            context: &CodegenContext,
        ) -> CodegenResult<CodegenArtifact> {
            if context.target != CodegenTarget::LlvmIr {
                return emit_backend_failure("inkwell backend currently only emits LLVM IR text");
            }

            module
                .validate()
                .map_err(|errors| CodegenError::InvalidModule {
                    module: module.name.clone(),
                    message: errors.join("; "),
                })?;

            let ctx = Context::create();
            let llvm_mod = ctx.create_module(&context.module_name);
            let triple = TargetTriple::create(context.resolved_target_triple());
            let target_data = TargetData::create(context.resolved_data_layout());
            let data_layout = target_data.get_data_layout();
            llvm_mod.set_triple(&triple);
            llvm_mod.set_data_layout(&data_layout);

            use crate::ir::IRInstructionKind;
            let mut extern_prototypes: HashMap<String, (String, Vec<String>)> = HashMap::new();

            for func in &module.functions {
                for block in &func.blocks {
                    for instr in &block.instructions {
                        if let IRInstructionKind::Call {
                            callee, arguments, ..
                        } = &instr.kind
                        {
                            if module.function(&callee).is_some() {
                                continue;
                            }
                            let params = arguments
                                .iter()
                                .map(|a| match a {
                                    crate::ir::IROperand::Float(_) => "double".to_string(),
                                    crate::ir::IROperand::Boolean(_) => "i1".to_string(),
                                    crate::ir::IROperand::Text(_) => "ptr".to_string(),
                                    _ => "double".to_string(),
                                })
                                .collect::<Vec<_>>();
                            let ret = if params.iter().any(|p| p == "double") {
                                "double"
                            } else {
                                "i64"
                            };
                            extern_prototypes
                                .entry(callee.clone())
                                .or_insert((ret.to_string(), params));
                        }
                    }
                }
            }

            for (name, (ret, params)) in &extern_prototypes {
                let param_types: Vec<BasicTypeEnum> = params
                    .iter()
                    .map(|p| map_type(&ctx, Some(p)).unwrap_or_else(|| ctx.f64_type().into()))
                    .collect();
                let fn_type = function_type_for(&ctx, Some(ret.as_str()), &param_types);
                let _ = llvm_mod.add_function(name, fn_type, None);
            }

            // Pre-declare all module functions so forward references work
            for function in &module.functions {
                let param_types: Vec<BasicTypeEnum> = function
                    .parameters
                    .iter()
                    .map(|p| {
                        map_type(&ctx, p.ty.as_deref()).unwrap_or_else(|| ctx.f64_type().into())
                    })
                    .collect();
                let fn_type =
                    function_type_for(&ctx, function.return_type.as_deref(), &param_types);
                let _ = llvm_mod.add_function(&function.name, fn_type, None);
            }

            for function in &module.functions {
                let param_types: Vec<BasicTypeEnum> = function
                    .parameters
                    .iter()
                    .map(|p| {
                        map_type(&ctx, p.ty.as_deref()).unwrap_or_else(|| ctx.f64_type().into())
                    })
                    .collect();
                let fn_type =
                    function_type_for(&ctx, function.return_type.as_deref(), &param_types);

                let fn_val = llvm_mod
                    .get_function(&function.name)
                    .unwrap_or_else(|| llvm_mod.add_function(&function.name, fn_type, None));
                let builder = ctx.create_builder();
                let entry_bb = ctx.append_basic_block(fn_val, "entry");
                builder.position_at_end(entry_bb);

                let mut allocas: HashMap<String, (PointerValue, BasicTypeEnum)> = HashMap::new();

                // Pre-create LLVM basic blocks for the function to avoid appending
                // duplicate blocks when emitting branches/jumps.
                let mut block_map: HashMap<String, inkwell::basic_block::BasicBlock<'_>> =
                    HashMap::new();
                for block in &function.blocks {
                    let bb = if block.id.0 == "entry" {
                        entry_bb
                    } else {
                        ctx.append_basic_block(fn_val, &block.id.0)
                    };
                    block_map.insert(block.id.0.clone(), bb);
                }

                // Materialize parameter slots
                for (idx, param) in function.parameters.iter().enumerate() {
                    if let Some(llvm_param) = fn_val.get_nth_param(idx as u32) {
                        let param_name = param.id.0.trim_start_matches('%');
                        llvm_param.set_name(param_name);
                        let param_type = map_type(&ctx, param.ty.as_deref())
                            .unwrap_or_else(|| llvm_param.get_type().into());
                        let slot = create_alloca(&builder, param_name, param_type)?;
                        builder.build_store(slot, llvm_param).map_err(|err| {
                            CodegenError::BackendFailure {
                                message: format!(
                                    "failed to store parameter '{}': {:?}",
                                    param_name, err
                                ),
                            }
                        })?;
                        let raw_param = param.id.0.clone();
                        let canonical_param = raw_param.trim_start_matches('%').to_string();
                        let prefixed_param = format!("%{}", canonical_param);
                        allocas.insert(canonical_param, (slot, param_type));
                        allocas.insert(prefixed_param, (slot, param_type));
                    }
                }

                let mut pending_phi_incomings = Vec::new();

                for block in &function.blocks {
                    let bb = *block_map.get(&block.id.0).ok_or_else(|| {
                        CodegenError::BackendFailure {
                            message: format!("missing LLVM block mapping for '{}'", block.id.0),
                        }
                    })?;
                    builder.position_at_end(bb);

                    // First emit PHI nodes so incoming values can be referenced
                    // First emit PHI nodes (without incoming values yet)
                    for instr in &block.instructions {
                        if let IRInstructionKind::Phi {
                            target, incoming, ..
                        } = &instr.kind
                        {
                            if incoming.is_empty() {
                                return emit_backend_failure(format!(
                                    "phi '{}' has no incoming edges",
                                    target.0
                                ));
                            }

                            let first_operand = if let Some((value_id, _)) = incoming.first() {
                                crate::ir::IROperand::Value(value_id.clone())
                            } else {
                                unreachable!("phi incoming cannot be empty");
                            };
                            let phi_ty = infer_operand_type(&ctx, &allocas, &first_operand);
                            let phi = builder
                                .build_phi(phi_ty, target.0.trim_start_matches('%'))
                                .map_err(|err| CodegenError::BackendFailure {
                                    message: format!(
                                        "failed to emit phi '{}': {:?}",
                                        target.0, err
                                    ),
                                })?;

                            for (value_id, incoming_block) in incoming {
                                let incoming_bb = *block_map.get(&incoming_block.0).ok_or_else(
                                    || CodegenError::BackendFailure {
                                        message: format!(
                                            "phi incoming block '{}' missing from function '{}'",
                                            incoming_block.0, function.name
                                        ),
                                    },
                                )?;
                                pending_phi_incomings.push((phi, value_id.clone(), incoming_bb));
                            }

                            let phi_value = phi.as_basic_value();
                            let slot = ensure_slot(
                                &builder,
                                target.0.trim_start_matches('%'),
                                phi_value.get_type(),
                                &mut allocas,
                            )?;
                            builder.build_store(slot, phi_value).map_err(|err| {
                                CodegenError::BackendFailure {
                                    message: format!(
                                        "failed to store phi '{}': {:?}",
                                        target.0, err
                                    ),
                                }
                            })?;
                        }
                    }

                    for instr in &block.instructions {
                        use crate::ir::IRInstructionKind;
                        match &instr.kind {
                            IRInstructionKind::Phi { .. } => {}
                            IRInstructionKind::Return(Some(op)) => {
                                let val = lower_operand(&ctx, &builder, &allocas, op)?;
                                builder.build_return(Some(&val)).map_err(|err| {
                                    CodegenError::BackendFailure {
                                        message: format!("failed to emit return: {:?}", err),
                                    }
                                })?;
                            }
                            IRInstructionKind::Return(None) => {
                                builder.build_return(None).map_err(|err| {
                                    CodegenError::BackendFailure {
                                        message: format!("failed to emit return: {:?}", err),
                                    }
                                })?;
                            }
                            IRInstructionKind::Assign { target, value, .. } => {
                                let name = target.0.trim_start_matches('%');
                                let val = lower_operand(&ctx, &builder, &allocas, value)?;
                                let ptr =
                                    ensure_slot(&builder, name, val.get_type(), &mut allocas)?;
                                builder.build_store(ptr, val).map_err(|err| {
                                    CodegenError::BackendFailure {
                                        message: format!("failed to emit store: {:?}", err),
                                    }
                                })?;
                            }
                            IRInstructionKind::Binary {
                                target,
                                op,
                                left,
                                right,
                            } => {
                                let l = lower_operand(&ctx, &builder, &allocas, left)?;
                                let r = lower_operand(&ctx, &builder, &allocas, right)?;
                                let both_bool = matches!(l.get_type(), BasicTypeEnum::IntType(_))
                                    && matches!(r.get_type(), BasicTypeEnum::IntType(_));
                                println!(
                                    "OP={:?}  left={}  right={}",
                                    op,
                                    l.get_type().print_to_string().to_string(),
                                    r.get_type().print_to_string().to_string()
                                );

                                let both_ptr =
                                    matches!(l.get_type(), BasicTypeEnum::PointerType(_))
                                        && matches!(r.get_type(), BasicTypeEnum::PointerType(_));

                                let res = match op {
                                    // ===== Operaciones numéricas =====
                                    crate::ir::IRBinaryOp::Add => builder
                                        .build_float_add(
                                            l.into_float_value(),
                                            r.into_float_value(),
                                            "tmpadd",
                                        )
                                        .map(BasicValueEnum::from)
                                        .map_err(|err| CodegenError::BackendFailure {
                                            message: format!("failed to emit binary op: {:?}", err),
                                        }),

                                    crate::ir::IRBinaryOp::Sub => builder
                                        .build_float_sub(
                                            l.into_float_value(),
                                            r.into_float_value(),
                                            "tmpsub",
                                        )
                                        .map(BasicValueEnum::from)
                                        .map_err(|err| CodegenError::BackendFailure {
                                            message: format!("failed to emit binary op: {:?}", err),
                                        }),

                                    crate::ir::IRBinaryOp::Mul => builder
                                        .build_float_mul(
                                            l.into_float_value(),
                                            r.into_float_value(),
                                            "tmpmul",
                                        )
                                        .map(BasicValueEnum::from)
                                        .map_err(|err| CodegenError::BackendFailure {
                                            message: format!("failed to emit binary op: {:?}", err),
                                        }),

                                    crate::ir::IRBinaryOp::Div => builder
                                        .build_float_div(
                                            l.into_float_value(),
                                            r.into_float_value(),
                                            "tmpdiv",
                                        )
                                        .map(BasicValueEnum::from)
                                        .map_err(|err| CodegenError::BackendFailure {
                                            message: format!("failed to emit binary op: {:?}", err),
                                        }),

                                    // ===== Booleanos =====
                                    crate::ir::IRBinaryOp::And => builder
                                        .build_and(l.into_int_value(), r.into_int_value(), "tmpand")
                                        .map(BasicValueEnum::from)
                                        .map_err(|err| CodegenError::BackendFailure {
                                            message: format!("failed to emit binary op: {:?}", err),
                                        }),

                                    crate::ir::IRBinaryOp::Or => builder
                                        .build_or(l.into_int_value(), r.into_int_value(), "tmpor")
                                        .map(BasicValueEnum::from)
                                        .map_err(|err| CodegenError::BackendFailure {
                                            message: format!("failed to emit binary op: {:?}", err),
                                        }),

                                    // ===== Comparaciones =====
                                    crate::ir::IRBinaryOp::Eq => {
                                        if both_bool {
                                            builder
                                                .build_int_compare(
                                                    IntPredicate::EQ,
                                                    l.into_int_value(),
                                                    r.into_int_value(),
                                                    "tmpeq",
                                                )
                                                .map(BasicValueEnum::from)
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit binary op: {:?}",
                                                        err
                                                    ),
                                                })
                                        } else if both_ptr {
                                            let strcmp_fn = match llvm_mod.get_function("strcmp") {
                                                Some(f) => f,
                                                None => {
                                                    let i32_ty = ctx.i32_type();
                                                    let ptr_ty =
                                                        ctx.ptr_type(AddressSpace::default());
                                                    let sig = i32_ty.fn_type(
                                                        &[ptr_ty.into(), ptr_ty.into()],
                                                        false,
                                                    );
                                                    llvm_mod.add_function("strcmp", sig, None)
                                                }
                                            };
                                            let call = builder
                                                .build_call(
                                                    strcmp_fn,
                                                    &[l.into(), r.into()],
                                                    "strcmp_res",
                                                )
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit strcmp: {:?}",
                                                        err
                                                    ),
                                                })?;
                                            let cmp_result = call
                                                .try_as_basic_value()
                                                .basic()
                                                .ok_or_else(|| CodegenError::BackendFailure {
                                                    message: "strcmp did not return a value"
                                                        .to_string(),
                                                })?;
                                            let zero = ctx.i32_type().const_int(0, false);
                                            builder
                                                .build_int_compare(
                                                    IntPredicate::EQ,
                                                    cmp_result.into_int_value(),
                                                    zero,
                                                    "streq",
                                                )
                                                .map(BasicValueEnum::from)
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit binary op: {:?}",
                                                        err
                                                    ),
                                                })
                                        } else {
                                            builder
                                                .build_float_compare(
                                                    FloatPredicate::OEQ,
                                                    l.into_float_value(),
                                                    r.into_float_value(),
                                                    "tmpeq",
                                                )
                                                .map(BasicValueEnum::from)
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit binary op: {:?}",
                                                        err
                                                    ),
                                                })
                                        }
                                    }

                                    crate::ir::IRBinaryOp::Ne => {
                                        if both_bool {
                                            builder
                                                .build_int_compare(
                                                    IntPredicate::NE,
                                                    l.into_int_value(),
                                                    r.into_int_value(),
                                                    "tmpne",
                                                )
                                                .map(BasicValueEnum::from)
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit binary op: {:?}",
                                                        err
                                                    ),
                                                })
                                        } else if both_ptr {
                                            let strcmp_fn = match llvm_mod.get_function("strcmp") {
                                                Some(f) => f,
                                                None => {
                                                    let i32_ty = ctx.i32_type();
                                                    let ptr_ty =
                                                        ctx.ptr_type(AddressSpace::default());
                                                    let sig = i32_ty.fn_type(
                                                        &[ptr_ty.into(), ptr_ty.into()],
                                                        false,
                                                    );
                                                    llvm_mod.add_function("strcmp", sig, None)
                                                }
                                            };
                                            let call = builder
                                                .build_call(
                                                    strcmp_fn,
                                                    &[l.into(), r.into()],
                                                    "strcmp_res",
                                                )
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit strcmp: {:?}",
                                                        err
                                                    ),
                                                })?;
                                            let cmp_result = call
                                                .try_as_basic_value()
                                                .basic()
                                                .ok_or_else(|| CodegenError::BackendFailure {
                                                    message: "strcmp did not return a value"
                                                        .to_string(),
                                                })?;
                                            let zero = ctx.i32_type().const_int(0, false);
                                            builder
                                                .build_int_compare(
                                                    IntPredicate::NE,
                                                    cmp_result.into_int_value(),
                                                    zero,
                                                    "strne",
                                                )
                                                .map(BasicValueEnum::from)
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit binary op: {:?}",
                                                        err
                                                    ),
                                                })
                                        } else {
                                            builder
                                                .build_float_compare(
                                                    FloatPredicate::ONE,
                                                    l.into_float_value(),
                                                    r.into_float_value(),
                                                    "tmpne",
                                                )
                                                .map(BasicValueEnum::from)
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit binary op: {:?}",
                                                        err
                                                    ),
                                                })
                                        }
                                    }

                                    crate::ir::IRBinaryOp::Lt => {
                                        if both_bool {
                                            builder
                                                .build_int_compare(
                                                    IntPredicate::SLT,
                                                    l.into_int_value(),
                                                    r.into_int_value(),
                                                    "tmplt",
                                                )
                                                .map(BasicValueEnum::from)
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit binary op: {:?}",
                                                        err
                                                    ),
                                                })
                                        } else {
                                            builder
                                                .build_float_compare(
                                                    FloatPredicate::OLT,
                                                    l.into_float_value(),
                                                    r.into_float_value(),
                                                    "tmplt",
                                                )
                                                .map(BasicValueEnum::from)
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit binary op: {:?}",
                                                        err
                                                    ),
                                                })
                                        }
                                    }

                                    crate::ir::IRBinaryOp::Le => {
                                        if both_bool {
                                            builder
                                                .build_int_compare(
                                                    IntPredicate::SLE,
                                                    l.into_int_value(),
                                                    r.into_int_value(),
                                                    "tmple",
                                                )
                                                .map(BasicValueEnum::from)
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit binary op: {:?}",
                                                        err
                                                    ),
                                                })
                                        } else {
                                            builder
                                                .build_float_compare(
                                                    FloatPredicate::OLE,
                                                    l.into_float_value(),
                                                    r.into_float_value(),
                                                    "tmple",
                                                )
                                                .map(BasicValueEnum::from)
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit binary op: {:?}",
                                                        err
                                                    ),
                                                })
                                        }
                                    }

                                    crate::ir::IRBinaryOp::Gt => {
                                        if both_bool {
                                            builder
                                                .build_int_compare(
                                                    IntPredicate::SGT,
                                                    l.into_int_value(),
                                                    r.into_int_value(),
                                                    "tmpgt",
                                                )
                                                .map(BasicValueEnum::from)
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit binary op: {:?}",
                                                        err
                                                    ),
                                                })
                                        } else {
                                            builder
                                                .build_float_compare(
                                                    FloatPredicate::OGT,
                                                    l.into_float_value(),
                                                    r.into_float_value(),
                                                    "tmpgt",
                                                )
                                                .map(BasicValueEnum::from)
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit binary op: {:?}",
                                                        err
                                                    ),
                                                })
                                        }
                                    }

                                    crate::ir::IRBinaryOp::Ge => {
                                        if both_bool {
                                            builder
                                                .build_int_compare(
                                                    IntPredicate::SGE,
                                                    l.into_int_value(),
                                                    r.into_int_value(),
                                                    "tmpge",
                                                )
                                                .map(BasicValueEnum::from)
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit binary op: {:?}",
                                                        err
                                                    ),
                                                })
                                        } else {
                                            builder
                                                .build_float_compare(
                                                    FloatPredicate::OGE,
                                                    l.into_float_value(),
                                                    r.into_float_value(),
                                                    "tmpge",
                                                )
                                                .map(BasicValueEnum::from)
                                                .map_err(|err| CodegenError::BackendFailure {
                                                    message: format!(
                                                        "failed to emit binary op: {:?}",
                                                        err
                                                    ),
                                                })
                                        }
                                    }

                                    crate::ir::IRBinaryOp::Mod => builder
                                        .build_float_rem(
                                            l.into_float_value(),
                                            r.into_float_value(),
                                            "tmpmod",
                                        )
                                        .map(BasicValueEnum::from)
                                        .map_err(|err| CodegenError::BackendFailure {
                                            message: format!("failed to emit binary op: {:?}", err),
                                        }),

                                    crate::ir::IRBinaryOp::Pow => {
                                        let pow_fn = match llvm_mod.get_function("pow") {
                                            Some(f) => f,

                                            None => {
                                                let sig = ctx.f64_type().fn_type(
                                                    &[ctx.f64_type().into(), ctx.f64_type().into()],
                                                    false,
                                                );

                                                llvm_mod.add_function("pow", sig, None)
                                            }
                                        };
                                        builder
                                            .build_call(pow_fn, &[l.into(), r.into()], "tmppow")
                                            .map(|call| call.try_as_basic_value().basic().unwrap())
                                            .map_err(|err| CodegenError::BackendFailure {
                                                message: format!(
                                                    "failed to emit binary op: {:?}",
                                                    err
                                                ),
                                            })
                                    }
                                    crate::ir::IRBinaryOp::Concat
                                    | crate::ir::IRBinaryOp::Concatenate => {
                                        let left_str = ensure_string(&ctx, &llvm_mod, &builder, l)?;

                                        let right_str =
                                            ensure_string(&ctx, &llvm_mod, &builder, r)?;

                                        let func_name = match op {
                                            crate::ir::IRBinaryOp::Concat => "hulk_concat",

                                            _ => "hulk_concat_space",
                                        };

                                        let concat_fn = match llvm_mod.get_function(func_name) {
                                            Some(f) => f,

                                            None => {
                                                let ptr_ty = ctx.ptr_type(AddressSpace::default());

                                                let signature = ptr_ty.fn_type(
                                                    &[ptr_ty.into(), ptr_ty.into()],
                                                    false,
                                                );

                                                llvm_mod.add_function(func_name, signature, None)
                                            }
                                        };

                                        builder
                                            .build_call(
                                                concat_fn,
                                                &[left_str.into(), right_str.into()],
                                                "concat_res",
                                            )
                                            .map(|call| match call.try_as_basic_value() {
                                                ValueKind::Basic(v) => v,

                                                _ => panic!("{} no devolvió un valor", func_name),
                                            })
                                            .map_err(|e| CodegenError::BackendFailure {
                                                message: e.to_string(),
                                            })
                                    }
                                }?;
                                let name = target.0.trim_start_matches('%');
                                let ptr =
                                    ensure_slot(&builder, name, res.get_type(), &mut allocas)?;
                                builder.build_store(ptr, res).map_err(|err| {
                                    CodegenError::BackendFailure {
                                        message: format!(
                                            "failed to store binary result: {:?}",
                                            err
                                        ),
                                    }
                                })?;
                            }
                            IRInstructionKind::Call {
                                target,
                                callee,
                                arguments,
                                ..
                            } => {
                                let callee_fn =
                                    llvm_mod.get_function(callee.as_str()).ok_or_else(|| {
                                        CodegenError::BackendFailure {
                                            message: format!("unknown function '{}'", callee),
                                        }
                                    })?;
                                let argsv: Vec<BasicValueEnum> = arguments
                                    .iter()
                                    .map(|a| lower_operand(&ctx, &builder, &allocas, a))
                                    .collect::<Result<_, _>>()?;
                                let call_site = builder
                                    .build_call(
                                        callee_fn,
                                        &basic_metadata_values(&argsv),
                                        "calltmp",
                                    )
                                    .map_err(|err| CodegenError::BackendFailure {
                                        message: format!(
                                            "failed to emit call '{}': {:?}",
                                            callee, err
                                        ),
                                    })?;
                                if let t = target {
                                    let name = t.0.trim_start_matches('%');
                                    if let Some(rv) = call_site.try_as_basic_value().basic() {
                                        let ptr = ensure_slot(
                                            &builder,
                                            name,
                                            rv.get_type(),
                                            &mut allocas,
                                        )?;
                                        builder.build_store(ptr, rv).map_err(|err| {
                                            CodegenError::BackendFailure {
                                                message: format!(
                                                    "failed to store call result: {:?}",
                                                    err
                                                ),
                                            }
                                        })?;
                                    }
                                }
                            }
                            IRInstructionKind::Jump { target } => {
                                let dest = *block_map.get(&target.0).ok_or_else(|| {
                                    CodegenError::BackendFailure {
                                        message: format!(
                                            "jump target block '{}' missing from function '{}'",
                                            target.0, function.name
                                        ),
                                    }
                                })?;
                                builder.build_unconditional_branch(dest).map_err(|err| {
                                    CodegenError::BackendFailure {
                                        message: format!("failed to emit jump: {:?}", err),
                                    }
                                })?;
                            }
                            IRInstructionKind::Branch {
                                condition,
                                then_block,
                                else_block,
                            } => {
                                let cond = lower_operand(&ctx, &builder, &allocas, condition)?;
                                let then_bb = *block_map.get(&then_block.0).ok_or_else(|| {
                                    CodegenError::BackendFailure {
                                        message: format!(
                                            "branch target block '{}' missing from function '{}'",
                                            then_block.0, function.name
                                        ),
                                    }
                                })?;
                                let else_bb = *block_map.get(&else_block.0).ok_or_else(|| {
                                    CodegenError::BackendFailure {
                                        message: format!(
                                            "branch target block '{}' missing from function '{}'",
                                            else_block.0, function.name
                                        ),
                                    }
                                })?;
                                builder
                                    .build_conditional_branch(
                                        cond.into_int_value(),
                                        then_bb,
                                        else_bb,
                                    )
                                    .map_err(|err| CodegenError::BackendFailure {
                                        message: format!("failed to emit branch: {:?}", err),
                                    })?;
                            }
                            IRInstructionKind::Unary {
                                target,
                                op,
                                operand,
                            } => {
                                let val = lower_operand(&ctx, &builder, &allocas, operand)?;
                                let use_float_ops =
                                    matches!(val.get_type(), BasicTypeEnum::FloatType(_));
                                let res: BasicValueEnum = match op {
                                    crate::ir::IRUnaryOp::Neg => {
                                        if use_float_ops {
                                            builder
                                                .build_float_neg(val.into_float_value(), "tmpneg")
                                                .map(BasicValueEnum::from)
                                        } else {
                                            builder
                                                .build_int_neg(val.into_int_value(), "tmpneg")
                                                .map(BasicValueEnum::from)
                                        }
                                    }
                                    crate::ir::IRUnaryOp::Not => builder
                                        .build_not(val.into_int_value(), "tmpnot")
                                        .map(BasicValueEnum::from),
                                }
                                .map_err(|err| CodegenError::BackendFailure {
                                    message: format!("failed to emit unary op: {:?}", err),
                                })?;
                                let name = target.0.trim_start_matches('%');
                                let ptr =
                                    ensure_slot(&builder, name, res.get_type(), &mut allocas)?;
                                builder.build_store(ptr, res).map_err(|err| {
                                    CodegenError::BackendFailure {
                                        message: format!("failed to store unary result: {:?}", err),
                                    }
                                })?;
                            }
                            IRInstructionKind::Nop => {}
                        }
                    }
                }
                // Second pass: add incoming values to phi nodes
                for (phi, value_id, incoming_bb) in &pending_phi_incomings {
                    let phi_incoming_builder = ctx.create_builder();
                    let terminator = incoming_bb.get_terminator().ok_or_else(|| {
                        CodegenError::BackendFailure {
                            message: format!(
                                "phi incoming block '{}' has no terminator after all blocks processed",
                                incoming_bb.get_name().to_str().unwrap_or("?")
                            ),
                        }
                    })?;
                    phi_incoming_builder.position_before(&terminator);
                    let incoming_value = lower_operand(
                        &ctx,
                        &phi_incoming_builder,
                        &allocas,
                        &crate::ir::IROperand::Value(value_id.clone()),
                    )?;
                    phi.add_incoming(&[(&incoming_value, *incoming_bb)]);
                }
            }

            let ir_str = llvm_mod.print_to_string().to_string();
            Ok(CodegenArtifact::text(CodegenTarget::LlvmIr, ir_str))
        }
    }
}

#[cfg(feature = "llvm-verify")]
pub use real::LlvmInkwellBackend;
