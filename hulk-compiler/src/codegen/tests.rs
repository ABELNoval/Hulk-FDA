use crate::codegen::{CodegenBackend, CodegenContext, CodegenTarget, LlvmInkwellBackend};
use crate::ir::{BasicBlock, IRFunction, IRInstruction, IRInstructionKind, IRModule, IROperand};

fn sample_module() -> IRModule {
    let mut module = IRModule::new("sample");
    let mut function = IRFunction::new("main");
    function.return_type = Some("i64".to_string());

    let mut entry = BasicBlock::new("entry");
    entry.push_instruction(IRInstruction::new(IRInstructionKind::Return(Some(
        IROperand::Integer(0),
    ))));

    function.add_block(entry);
    module.add_function(function);
    module
}

#[cfg(feature = "llvm-verify")]
#[test]
fn inkwell_backend_emits_llvm_ir_for_valid_module() {
    let backend = LlvmInkwellBackend::new();
    let context = CodegenContext::new("sample", CodegenTarget::LlvmIr);

    let artifact = backend
        .emit_module(&sample_module(), &context)
        .expect("codegen should work");
    let text = artifact.as_text().expect("expected text output");

    assert!(text.contains("ModuleID = 'sample'"));
    assert!(text.contains("define i64 @main()"));
    assert!(text.contains("ret i64 0"));
}

#[cfg(feature = "llvm-verify")]
#[test]
fn inkwell_backend_rejects_non_llvm_target() {
    let backend = LlvmInkwellBackend::new();
    let context = CodegenContext::new("sample", CodegenTarget::Native);

    let error = backend
        .emit_module(&sample_module(), &context)
        .expect_err("backend should reject native target");

    assert!(error.to_string().contains("only emits LLVM IR text"));
}

#[cfg(not(feature = "llvm-verify"))]
#[test]
fn inkwell_backend_is_disabled_without_feature() {
    let backend = LlvmInkwellBackend::new();
    let context = CodegenContext::new("sample", CodegenTarget::LlvmIr);

    let error = backend
        .emit_module(&sample_module(), &context)
        .expect_err("backend should be disabled without feature");

    assert!(error.to_string().contains("feature 'llvm-verify'"));
}
