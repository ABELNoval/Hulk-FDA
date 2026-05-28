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
use crate::codegen::LlvmLifecycle;
use crate::ir::{BasicBlock, IRFunction, IRInstruction, IRInstructionKind, IRModule, IRValueId, IROperand};

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
    assert!(text.contains("target triple = \"x86_64-unknown-linux-gnu\""));
    assert!(text.contains("target datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\""));
    assert!(text.contains("define i64 @main()"));
    assert!(text.contains("ret i64 0"));
    // runtime declarations should be present
    assert!(text.contains("declare void @print(ptr)"));
    assert!(text.contains("declare ptr @hulk_alloc(i64)"));
}

#[test]
fn backend_uses_custom_target_configuration() {
    let backend = LlvmTextBackend::new();
    let context = CodegenContext::new("sample", CodegenTarget::LlvmIr)
        .with_target_triple("aarch64-unknown-linux-gnu")
        .with_data_layout("e-m:e-p:64:64-i64:64-v128:128-a:0:64-n32:64-S128");

    let artifact = backend
        .emit_module(&sample_module(), &context)
        .expect("codegen should work");
    let text = artifact.as_text().expect("expected text output");

    assert!(text.contains("target triple = \"aarch64-unknown-linux-gnu\""));
    assert!(text.contains("target datalayout = \"e-m:e-p:64:64-i64:64-v128:128-a:0:64-n32:64-S128\""));
}

#[test]
fn backend_writes_llvm_text_to_file() {
    let backend = LlvmTextBackend::new();
    let context = CodegenContext::new("sample", CodegenTarget::LlvmIr);
    let module = sample_module();

    let mut path = std::env::temp_dir();
    path.push(format!("hulk_llvm_{}.ll", std::process::id()));

    let _ = backend
        .emit_module_to_path(&module, &context, &path)
        .expect("should write artifact");

    let contents = std::fs::read_to_string(&path).expect("read written file");
    assert!(contents.contains("ModuleID = 'sample'"));
    assert!(contents.contains("declare void @print(ptr)"));

    // cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn backend_detects_signature_conflict() {
    // Build IR: main calls `foo` with one argument; `foo` is defined with zero params.
    let mut module = IRModule::new("conflict");

    // main function
    let mut main = IRFunction::new("main");
    main.return_type = Some("i64".to_string());
    let mut entry = BasicBlock::new("entry");
    entry.push_instruction(IRInstruction::new(IRInstructionKind::Call {
        target: Some(IRValueId::new("t0".to_string())),
        callee: "foo".to_string(),
        arguments: vec![IROperand::Integer(42)],
        original: None,
    }));
    entry.push_instruction(IRInstruction::new(IRInstructionKind::Return(Some(
        IROperand::Integer(0),
    ))));
    main.add_block(entry);
    module.add_function(main);

    // foo function with zero parameters (conflict)
    let mut foo = IRFunction::new("foo");
    foo.return_type = Some("i64".to_string());
    let mut fentry = BasicBlock::new("entry");
    fentry.push_instruction(IRInstruction::new(IRInstructionKind::Return(Some(
        IROperand::Integer(0),
    ))));
    foo.add_block(fentry);
    module.add_function(foo);

    let backend = LlvmTextBackend::new();
    let context = CodegenContext::new("conflict", CodegenTarget::LlvmIr);

    let err = backend.emit_module(&module, &context).expect_err("should detect conflict");
    assert!(err.to_string().to_lowercase().contains("conflict"));
}

#[test]
fn validate_fails_on_unknown_symbol() {
    let lifecycle = LlvmLifecycle::new();
    let mut llvm_module = lifecycle.create_module("unknown").expect("create module");

    // IR with a call to `mystery` but no prototype or definition registered
    let mut module = IRModule::new("unknown_ir");
    let mut main = IRFunction::new("main");
    main.return_type = Some("i64".to_string());
    let mut entry = BasicBlock::new("entry");
    entry.push_instruction(IRInstruction::new(IRInstructionKind::Call {
        target: Some(IRValueId::new("t0".to_string())),
        callee: "mystery".to_string(),
        arguments: vec![],
        original: None,
    }));
    entry.push_instruction(IRInstruction::new(IRInstructionKind::Return(Some(
        IROperand::Integer(0),
    ))));
    main.add_block(entry);
    module.add_function(main);

    let res = lifecycle.validate_full_module(&llvm_module, &module);
    assert!(res.is_err());
    let err = res.err().unwrap();
    assert!(err.to_string().to_lowercase().contains("unknown symbol") || err.to_string().to_lowercase().contains("unknown"));
}

#[test]
fn llvm_type_mapping_rules_cover_language_and_ir_names() {
    assert_eq!(LlvmTextBackend::llvm_type_for(Some("Number")), "double");
    assert_eq!(LlvmTextBackend::llvm_type_for(Some("Boolean")), "i1");
    assert_eq!(LlvmTextBackend::llvm_type_for(Some("String")), "ptr");
    assert_eq!(LlvmTextBackend::llvm_type_for(Some("void")), "void");
    assert_eq!(LlvmTextBackend::llvm_type_for(Some("i64")), "i64");
    assert_eq!(LlvmTextBackend::llvm_type_for(Some("Widget")), "ptr");
    assert_eq!(LlvmTextBackend::llvm_type_for(None), "i64");
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
