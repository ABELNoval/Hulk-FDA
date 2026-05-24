#[cfg(test)]
mod tests {
    use crate::ir::instruction::{IRBinaryOp, IRInstruction, IRInstructionKind, IROperand};
    use crate::ir::test_support::{
        IrTestHarness, assert_module_eq, assert_rendered_module_contains,
    };
    use crate::ir::{IRModule, IRNaming, IRValue, IRValueKind};

    #[test]
    fn harness_builds_simple_module() {
        let mut harness = IrTestHarness::new("sample");
        harness.set_return_type("main", "Number");
        harness.add_parameter("main", "value", "Number");
        harness.add_block("main", IRNaming::basic_block_name(0));
        harness.add_instruction(
            "main",
            IRNaming::basic_block_name(0),
            IRInstruction::new(IRInstructionKind::Return(Some(IROperand::Integer(1)))),
        );

        let module = harness.module();
        assert_eq!(module.name, "sample");
        assert_eq!(module.functions.len(), 1);
        assert_rendered_module_contains(module, "sample");
        assert_rendered_module_contains(module, "Return(Some(Integer(1)))");
    }

    #[test]
    fn harness_compares_modules_through_normalization() {
        let mut left = IRModule::new("sample");
        let mut right = IRModule::new("sample");

        let mut left_function = crate::ir::IRFunction::new("main");
        let mut right_function = crate::ir::IRFunction::new("main");

        left_function
            .parameters
            .push(IRValue::new("value", IRValueKind::Parameter).with_type("Number"));
        right_function
            .parameters
            .push(IRValue::new("value", IRValueKind::Parameter).with_type("Number"));

        let left_block = crate::ir::BasicBlock::new(IRNaming::basic_block_name(0));
        let right_block = crate::ir::BasicBlock::new(IRNaming::basic_block_name(0));

        left_function.add_block(left_block);
        right_function.add_block(right_block);

        left.add_function(left_function);
        right.add_function(right_function);

        assert_module_eq(&left, &right);
    }

    #[test]
    fn naming_helper_uses_stable_prefixes() {
        assert_eq!(IRNaming::temporary_name(3), "%t3");
        assert_eq!(IRNaming::basic_block_name(2), "bb2");
        assert_eq!(IRNaming::module_name("sample"), "module::sample");
    }

    #[test]
    fn instruction_defines_value_for_assignment() {
        let instruction = IRInstruction::new(IRInstructionKind::Binary {
            target: crate::ir::IRValueId::new("%t0"),
            op: IRBinaryOp::Add,
            left: IROperand::Integer(1),
            right: IROperand::Integer(2),
        });

        assert!(instruction.defines_value().is_some());
        assert_eq!(instruction.defines_value().unwrap().0, "%t0");
    }
}
