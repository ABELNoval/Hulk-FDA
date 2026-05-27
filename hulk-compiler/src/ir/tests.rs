#[cfg(test)]
mod tests_ir {
    use crate::ir::instruction::{IRBinaryOp, IRInstruction, IRInstructionKind, IROperand};
    use crate::ir::lowering::IRBuilder;
    use crate::ir::test_support::{
        IrTestHarness, assert_module_eq, assert_rendered_module_contains,
    };
    use crate::ir::{BasicBlock, IRFunction, IRModule, IRNaming, IRValue, IRValueKind};
    use crate::parser::ast::{BinaryOperator, Expr, Literal, Program};
    use crate::utils::errors::span::Span;

    fn test_span() -> Span {
        Span::new("test".to_string(), 1, 1, 1, 1)
    }

    struct LoweringFixture {
        program: Program,
        expected_fragments: Vec<&'static str>,
    }

    fn arithmetic_fixture() -> LoweringFixture {
        let span = test_span();
        LoweringFixture {
            program: Program::new(
                Vec::new(),
                Some(Expr::binary(
                    Expr::literal(Literal::Number(1.0), span.clone()),
                    BinaryOperator::Add,
                    Expr::literal(Literal::Number(2.0), span.clone()),
                    span.clone(),
                )),
                span,
            ),
            expected_fragments: vec!["+", "return"],
        }
    }

    fn control_flow_fixture() -> LoweringFixture {
        let span = test_span();
        LoweringFixture {
            program: Program::new(
                Vec::new(),
                Some(Expr::block(
                    vec![
                        Expr::if_expr(
                            Expr::literal(Literal::Boolean(true), span.clone()),
                            Expr::literal(Literal::Number(10.0), span.clone()),
                            Vec::new(),
                            Some(Expr::literal(Literal::Number(20.0), span.clone())),
                            span.clone(),
                        ),
                        Expr::while_expr(
                            Expr::literal(Literal::Boolean(true), span.clone()),
                            Expr::literal(Literal::Number(1.0), span.clone()),
                            span.clone(),
                        ),
                    ],
                    span.clone(),
                )),
                span,
            ),
            expected_fragments: vec![
                "if_then",
                "if_else",
                "if_merge",
                "while_cond",
                "while_body",
                "while_exit",
            ],
        }
    }

    fn vector_fixture() -> LoweringFixture {
        let span = test_span();
        LoweringFixture {
            program: Program::new(
                Vec::new(),
                Some(Expr::vector_comprehension(
                    Expr::literal(Literal::Number(2.0), span.clone()),
                    "item".to_string(),
                    Expr::vector_literal(
                        vec![Expr::literal(Literal::Number(1.0), span.clone())],
                        span.clone(),
                    ),
                    span.clone(),
                )),
                span,
            ),
            expected_fragments: vec!["call vector_literal", "call vector_comprehension"],
        }
    }

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
    fn value_constructors_set_expected_kinds() {
        let temp = crate::ir::IRValue::temporary("%t0");
        let parameter = crate::ir::IRValue::parameter("x");
        let constant = crate::ir::IRValue::constant("c0");
        let named = crate::ir::IRValue::named("field");
        let phi = crate::ir::IRValue::phi("%p0");

        assert!(temp.is_temporary());
        assert!(parameter.is_parameter());
        assert!(constant.is_constant());
        assert!(named.kind == IRValueKind::Named);
        assert!(phi.is_phi());
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
    fn lowers_arithmetic_expression_and_literals_into_ir_structure() {
        let span = test_span();
        let expr = Expr::binary(
            Expr::literal(Literal::Number(1.0), span.clone()),
            BinaryOperator::Add,
            Expr::literal(Literal::Number(2.0), span.clone()),
            span.clone(),
        );
        let program = Program::new(Vec::new(), Some(expr), span.clone());

        let mut builder = IRBuilder::new("test");
        let module = builder
            .lower_program(&program)
            .expect("lowering must succeed");

        let mut expected = IRModule::new("module::lowered");
        let mut function = IRFunction::new("__entry");
        let mut block = BasicBlock::new("entry");
        block.push_instruction(IRInstruction::new(IRInstructionKind::Assign {
            target: crate::ir::IRValueId::new("%t0"),
            value: IROperand::Float(1.0),
            original: None,
        }));
        block.push_instruction(IRInstruction::new(IRInstructionKind::Assign {
            target: crate::ir::IRValueId::new("%t1"),
            value: IROperand::Float(2.0),
            original: None,
        }));
        block.push_instruction(IRInstruction::new(IRInstructionKind::Binary {
            target: crate::ir::IRValueId::new("%t2"),
            op: IRBinaryOp::Add,
            left: IROperand::Value(crate::ir::IRValueId::new("%t0")),
            right: IROperand::Value(crate::ir::IRValueId::new("%t1")),
        }));
        block.push_instruction(IRInstruction::new(IRInstructionKind::Return(Some(
            IROperand::Value(crate::ir::IRValueId::new("%t2")),
        ))));
        function.add_block(block);
        expected.add_function(function);

        assert_module_eq(&module, &expected);
    }

    #[test]
    fn lowers_variable_references_assignments_and_control_flow() {
        let span = test_span();
        let program = Program::new(
            Vec::new(),
            Some(Expr::block(
                vec![
                    Expr::let_expr(
                        "x".to_string(),
                        None,
                        Some(Expr::literal(Literal::Number(1.0), span.clone())),
                        span.clone(),
                    ),
                    Expr::assignment(
                        Expr::identifier("x".to_string(), span.clone()),
                        Expr::binary(
                            Expr::identifier("x".to_string(), span.clone()),
                            BinaryOperator::Add,
                            Expr::literal(Literal::Number(2.0), span.clone()),
                            span.clone(),
                        ),
                        span.clone(),
                    ),
                    Expr::identifier("x".to_string(), span.clone()),
                ],
                span.clone(),
            )),
            span.clone(),
        );

        let mut builder = IRBuilder::new("test");
        let module = builder
            .lower_program(&program)
            .expect("lowering must succeed");

        let entry = module
            .function("__entry")
            .expect("entry function must exist");
        assert_eq!(entry.block_count(), 1);
        assert!(entry.blocks[0].instruction_count() >= 5);

        let rendered = module.fmt_display();
        assert!(rendered.contains("__entry"));
        assert!(rendered.contains("= 1"));
        assert!(rendered.contains("+"));
        assert!(rendered.contains("return"));
    }

    #[test]
    fn lowers_if_and_loop_blocks_into_cfg_shape() {
        let span = test_span();
        let program = Program::new(
            Vec::new(),
            Some(Expr::block(
                vec![
                    Expr::if_expr(
                        Expr::literal(Literal::Boolean(true), span.clone()),
                        Expr::literal(Literal::Number(10.0), span.clone()),
                        Vec::new(),
                        Some(Expr::literal(Literal::Number(20.0), span.clone())),
                        span.clone(),
                    ),
                    Expr::while_expr(
                        Expr::literal(Literal::Boolean(true), span.clone()),
                        Expr::literal(Literal::Number(1.0), span.clone()),
                        span.clone(),
                    ),
                ],
                span.clone(),
            )),
            span.clone(),
        );

        let mut builder = IRBuilder::new("test");
        let module = builder
            .lower_program(&program)
            .expect("lowering must succeed");

        let rendered = module.fmt_display();
        assert!(rendered.contains("if_then"));
        assert!(rendered.contains("if_else"));
        assert!(rendered.contains("if_merge"));
        assert!(rendered.contains("while_cond"));
        assert!(rendered.contains("while_body"));
        assert!(rendered.contains("while_exit"));
        assert!(rendered.contains("phi"));
        assert!(rendered.contains("br"));
        assert!(rendered.contains("jump"));
    }

    #[test]
    fn lowering_fixtures_cover_expected_ir_fragments() {
        let fixtures = vec![
            arithmetic_fixture(),
            control_flow_fixture(),
            vector_fixture(),
        ];

        for fixture in fixtures {
            let mut builder = IRBuilder::new("test");
            let module = builder
                .lower_program(&fixture.program)
                .expect("lowering must succeed");

            let rendered = module.fmt_display();
            for fragment in fixture.expected_fragments {
                assert!(
                    rendered.contains(fragment),
                    "expected fragment `{}` not found in rendered module:\n{}",
                    fragment,
                    rendered
                );
            }
        }
    }

    #[test]
    fn lowers_vector_literal_as_intrinsic_call() {
        let span = test_span();
        let program = Program::new(
            Vec::new(),
            Some(Expr::vector_literal(
                vec![
                    Expr::literal(Literal::Number(1.0), span.clone()),
                    Expr::literal(Literal::Number(2.0), span.clone()),
                ],
                span.clone(),
            )),
            span.clone(),
        );

        let mut builder = IRBuilder::new("test");
        let module = builder
            .lower_program(&program)
            .expect("lowering must succeed");

        let rendered = module.fmt_display();
        assert!(rendered.contains("call vector_literal"));
    }

    #[test]
    fn lowers_vector_comprehension_as_intrinsic_call() {
        let span = test_span();
        let program = Program::new(
            Vec::new(),
            Some(Expr::vector_comprehension(
                Expr::literal(Literal::Number(2.0), span.clone()),
                "item".to_string(),
                Expr::vector_literal(
                    vec![Expr::literal(Literal::Number(1.0), span.clone())],
                    span.clone(),
                ),
                span.clone(),
            )),
            span.clone(),
        );

        let mut builder = IRBuilder::new("test");
        let module = builder
            .lower_program(&program)
            .expect("lowering must succeed");

        let rendered = module.fmt_display();
        assert!(rendered.contains("call vector_comprehension"));
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

        assert_eq!(cfg.successors(&b1).unwrap(), std::slice::from_ref(&b2));
        assert_eq!(cfg.predecessors(&b2).unwrap(), std::slice::from_ref(&b1));
        assert_eq!(cfg.successors(&b2).unwrap(), std::slice::from_ref(&b3));
        assert_eq!(cfg.predecessors(&b3).unwrap(), std::slice::from_ref(&b2));
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
        assert_eq!(then_succs, std::slice::from_ref(&merge));

        let else_succs = cfg.successors(&else_b).unwrap();
        assert_eq!(else_succs, std::slice::from_ref(&merge));
    }

    #[test]
    fn cfg_build_loop() {
        use crate::ir::block::ControlFlowGraph;
        let mut cfg = ControlFlowGraph::new();

        let head = cfg.create_block("head");
        let (cond, body, exit) = cfg.build_loop(&head, "cond", "body", "exit");

        assert!(cfg.validate().is_ok());

        assert_eq!(cfg.successors(&head).unwrap(), std::slice::from_ref(&cond));

        let cond_succs = cfg.successors(&cond).unwrap();
        assert!(cond_succs.contains(&body));
        assert!(cond_succs.contains(&exit));

        assert_eq!(cfg.successors(&body).unwrap(), std::slice::from_ref(&cond));
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

    #[test]
    fn cfg_validate_rejects_unreachable_blocks() {
        use crate::ir::block::ControlFlowGraph;
        let mut cfg = ControlFlowGraph::new();

        let entry = cfg.create_block("entry");
        let _orphan = cfg.create_block("orphan");

        let errors = cfg
            .validate()
            .expect_err("unreachable blocks must fail validation");
        assert!(errors.iter().any(|error| error.contains("unreachable")));
        assert_eq!(cfg.entry, Some(entry));
    }

    #[test]
    fn module_validate_accepts_well_formed_ir() {
        let mut module = IRModule::new("sample");
        let mut function = IRFunction::new("main");
        let mut block = BasicBlock::new("entry");

        block.push_instruction(IRInstruction::new(IRInstructionKind::Assign {
            target: crate::ir::IRValueId::new("%t0"),
            value: IROperand::Integer(1),
            original: None,
        }));
        block.push_instruction(IRInstruction::new(IRInstructionKind::Return(Some(
            IROperand::Value(crate::ir::IRValueId::new("%t0")),
        ))));

        function.add_block(block);
        module.add_function(function);

        assert!(module.validate().is_ok());
    }

    #[test]
    fn module_validate_rejects_duplicate_ssa_definitions() {
        let mut module = IRModule::new("sample");
        let mut function = IRFunction::new("main");
        let mut block = BasicBlock::new("entry");

        block.push_instruction(IRInstruction::new(IRInstructionKind::Assign {
            target: crate::ir::IRValueId::new("%t0"),
            value: IROperand::Integer(1),
            original: None,
        }));
        block.push_instruction(IRInstruction::new(IRInstructionKind::Assign {
            target: crate::ir::IRValueId::new("%t0"),
            value: IROperand::Integer(2),
            original: None,
        }));
        block.push_instruction(IRInstruction::new(IRInstructionKind::Return(Some(
            IROperand::Value(crate::ir::IRValueId::new("%t0")),
        ))));

        function.add_block(block);
        module.add_function(function);

        let errors = module
            .validate()
            .expect_err("duplicate SSA definitions must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("defines SSA value '%t0' more than once"))
        );
    }
}
