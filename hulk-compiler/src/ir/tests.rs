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

    #[test]
    fn cfg_basic_linking_and_validation() {
        use crate::ir::block::ControlFlowGraph;
        let mut cfg = ControlFlowGraph::new();

        let b1 = cfg.create_block("bb1");
        let b2 = cfg.create_block("bb2");
        let b3 = cfg.create_block("bb3");

        cfg.add_edge(&b1, &b2);
        cfg.add_edge(&b2, &b3);

        assert!(cfg.validate().is_ok());

        assert_eq!(cfg.successors(&b1).unwrap(), &[b2.clone()]);
        assert_eq!(cfg.predecessors(&b2).unwrap(), &[b1.clone()]);
        assert_eq!(cfg.successors(&b2).unwrap(), &[b3.clone()]);
        assert_eq!(cfg.predecessors(&b3).unwrap(), &[b2.clone()]);
    }

    #[test]
    fn cfg_build_if_else() {
        use crate::ir::block::ControlFlowGraph;
        let mut cfg = ControlFlowGraph::new();

        let head = cfg.create_block("head");
        let (then_b, else_b, merge) = cfg.build_if_else(&head, "then", "else", "merge");

        assert!(cfg.validate().is_ok());

        let succs = cfg.successors(&head).unwrap();
        assert!(succs.contains(&then_b));
        assert!(succs.contains(&else_b));

        let then_succs = cfg.successors(&then_b).unwrap();
        assert_eq!(then_succs, &[merge.clone()]);

        let else_succs = cfg.successors(&else_b).unwrap();
        assert_eq!(else_succs, &[merge.clone()]);
    }

    #[test]
    fn cfg_build_loop() {
        use crate::ir::block::ControlFlowGraph;
        let mut cfg = ControlFlowGraph::new();

        let head = cfg.create_block("head");
        let (cond, body, exit) = cfg.build_loop(&head, "cond", "body", "exit");

        assert!(cfg.validate().is_ok());

        assert_eq!(cfg.successors(&head).unwrap(), &[cond.clone()]);
        
        let cond_succs = cfg.successors(&cond).unwrap();
        assert!(cond_succs.contains(&body));
        assert!(cond_succs.contains(&exit));

        assert_eq!(cfg.successors(&body).unwrap(), &[cond.clone()]);
    }

    #[test]
    fn cfg_traversals() {
        use crate::ir::block::ControlFlowGraph;
        let mut cfg = ControlFlowGraph::new();

        let b1 = cfg.create_block("bb1"); // entry
        let b2 = cfg.create_block("bb2");
        let b3 = cfg.create_block("bb3");
        let b4 = cfg.create_block("bb4");

        // b1 -> b2, b3
        // b2 -> b4
        // b3 -> b4
        cfg.add_edge(&b1, &b2);
        cfg.add_edge(&b1, &b3);
        cfg.add_edge(&b2, &b4);
        cfg.add_edge(&b3, &b4);

        let dfs = cfg.dfs_traversal();
        assert_eq!(dfs[0], b1);
        assert_eq!(dfs.len(), 4); // Visited all
        assert!(dfs.contains(&b2) && dfs.contains(&b3) && dfs.contains(&b4));

        let bfs = cfg.bfs_traversal();
        assert_eq!(bfs[0], b1);
        assert!(bfs[1] == b2 || bfs[1] == b3);
        assert!(bfs[2] == b2 || bfs[2] == b3);
        assert_eq!(bfs[3], b4);
    }
}
