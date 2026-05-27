// =============================================================================
// Tests Unitarios - Codegen
// =============================================================================
//
// Tests para el módulo Codegen. Aquí se prueban:
//
// - Generación correcta de código para cada construcción
// - Register allocation funciona correctamente
// - Calling conventions respetadas
// - Código generado es válido y ejecutable
// - Optimizaciones de bajo nivel
//
// =============================================================================
// Organizer / QA owns these smoke tests because they validate the contract
// between the backend scaffold and the LLVM lowering implementation.

use crate::codegen::{CodegenBackend, CodegenContext, CodegenTarget, LlvmTextBackend};
use crate::ir::{BasicBlock, IRFunction, IRInstruction, IRInstructionKind, IRModule};

fn sample_module() -> IRModule {
    let mut module = IRModule::new("sample");
    let mut function = IRFunction::new("main");
    function.return_type = Some("i64".to_string());

    let mut entry = BasicBlock::new("entry");
    entry.push_instruction(IRInstruction::new(IRInstructionKind::Return(Some(
        crate::ir::IROperand::Integer(0),
    ))));

    function.add_block(entry);
    module.add_function(function);
    module
}

#[test]
fn backend_emits_llvm_text_for_valid_module() {
    let backend = LlvmTextBackend::new();
    let context = CodegenContext::new("sample", CodegenTarget::LlvmIr);
    let module = sample_module();

    let artifact = backend
        .emit_module(&module, &context)
        .expect("codegen should work");
    let text = artifact.as_text().expect("expected text output");

    assert!(text.contains("ModuleID = 'sample'"));
    assert!(text.contains("define i64 @main()"));
    assert!(text.contains("ret i64 0"));
}

#[test]
fn backend_rejects_non_llvm_target() {
    let backend = LlvmTextBackend::new();
    let context = CodegenContext::new("sample", CodegenTarget::Native);

    let error = backend
        .emit_module(&sample_module(), &context)
        .expect_err("backend should reject native target");

    assert!(
        error
            .to_string()
            .contains("only supports LLVM IR text output")
    );
}
