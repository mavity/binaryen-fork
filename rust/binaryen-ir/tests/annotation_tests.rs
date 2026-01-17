use binaryen_core::{Literal, Type};
use binaryen_ir::{Annotation, Function, HighLevelType, IrBuilder, LoopType, Module, VariableRole};
use bumpalo::Bump;

#[test]
fn test_basic_annotation_lifecycle() {
    let bump = Bump::new();
    let mut module = Module::new(&bump);
    let builder = IrBuilder::new(&bump);

    let expr = builder.const_(Literal::I32(1));

    // Test initial state
    assert!(module.get_annotations(expr).is_none());

    // Test setting and getting
    module.set_annotation(expr, Annotation::Type(HighLevelType::Bool));
    let anns = module
        .get_annotations(expr)
        .expect("Should have annotations");
    assert_eq!(anns.high_level_type, Some(HighLevelType::Bool));

    // Test overwriting
    module.set_annotation(expr, Annotation::Variable(VariableRole::LoopIndex));
    let anns2 = module
        .get_annotations(expr)
        .expect("Should have updated annotations");
    assert_eq!(anns2.role, Some(VariableRole::LoopIndex));
    // Verify it didn't clear the type (multi-annotation behavior)
    assert_eq!(anns2.high_level_type, Some(HighLevelType::Bool));
}

#[test]
fn test_all_annotation_variants() {
    let bump = Bump::new();
    let mut module = Module::new(&bump);
    let builder = IrBuilder::new(&bump);

    // Loop
    let l = builder.loop_(None, builder.nop(), Type::NONE);
    module.set_annotation(l, Annotation::Loop(LoopType::While));
    assert_eq!(
        module.get_annotations(l).map(|a| a.loop_type),
        Some(Some(LoopType::While))
    );

    module.set_annotation(l, Annotation::Loop(LoopType::For));
    assert_eq!(
        module.get_annotations(l).map(|a| a.loop_type),
        Some(Some(LoopType::For))
    );

    module.set_annotation(l, Annotation::Loop(LoopType::DoWhile));
    assert_eq!(
        module.get_annotations(l).map(|a| a.loop_type),
        Some(Some(LoopType::DoWhile))
    );

    // Type
    let t = builder.const_(Literal::I32(0));
    module.set_annotation(t, Annotation::Type(HighLevelType::Pointer));
    assert_eq!(
        module.get_annotations(t).map(|a| a.high_level_type),
        Some(Some(HighLevelType::Pointer))
    );

    // Variable
    let v = builder.local_get(0, Type::I32);
    module.set_annotation(v, Annotation::Variable(VariableRole::BasePointer));
    assert_eq!(
        module.get_annotations(v).map(|a| a.role),
        Some(Some(VariableRole::BasePointer))
    );
}

#[test]
fn test_multiple_annotations_in_module() {
    let bump = Bump::new();
    let mut module = Module::new(&bump);
    let builder = IrBuilder::new(&bump);

    let mut exprs = Vec::new();
    for i in 0..100 {
        let e = builder.const_(Literal::I32(i));
        module.set_annotation(e, Annotation::Type(HighLevelType::Bool));
        exprs.push(e);
    }

    // Verify all preserved
    for e in exprs {
        assert_eq!(
            module.get_annotations(e).map(|a| a.high_level_type),
            Some(Some(HighLevelType::Bool))
        );
    }
}

#[test]
fn test_complex_ir_tagging() {
    let bump = Bump::new();
    let mut module = Module::new(&bump);
    let builder = IrBuilder::new(&bump);

    // for (i=0; i < 10; i++) { ... }
    // In Wasm this is often:
    // (block $break
    //   (local.set $i (i32.const 0))
    //   (loop $top
    //     (br_if $break (i32.ge_s (local.get $i) (i32.const 10)))
    //     ...body...
    //     (local.set $i (i32.add (local.get $i) (i32.const 1)))
    //     (br $top)
    //   )
    // )

    let i_init = builder.local_set(0, builder.const_(Literal::I32(0)));
    let condition = builder.binary(
        binaryen_ir::BinaryOp::GeSInt32,
        builder.local_get(0, Type::I32),
        builder.const_(Literal::I32(10)),
        Type::I32,
    );
    let br_if = builder.break_("break", Some(condition), None, Type::NONE);
    let i_inc = builder.local_set(
        0,
        builder.binary(
            binaryen_ir::BinaryOp::AddInt32,
            builder.local_get(0, Type::I32),
            builder.const_(Literal::I32(1)),
            Type::I32,
        ),
    );
    let br_top = builder.break_("top", None, None, Type::NONE);

    let mut loop_body_vec = bumpalo::collections::Vec::new_in(&bump);
    loop_body_vec.push(br_if);
    loop_body_vec.push(i_inc);
    loop_body_vec.push(br_top);

    let loop_body = builder.block(Some("loop_body"), loop_body_vec, Type::NONE);
    let loop_expr = builder.loop_(Some("top"), loop_body, Type::NONE);

    // Tag the patterns
    module.set_annotation(loop_expr, Annotation::Loop(LoopType::For));
    module.set_annotation(i_init, Annotation::Variable(VariableRole::LoopIndex));

    // Verify retrieval in "lifting" simulation
    let retrieved_loop = module.get_annotations(loop_expr).unwrap();
    assert_eq!(retrieved_loop.loop_type, Some(LoopType::For));

    let retrieved_var = module.get_annotations(i_init).unwrap();
    assert_eq!(retrieved_var.role, Some(VariableRole::LoopIndex));
}

#[test]
fn test_traversal_tagging() {
    use binaryen_ir::{ExpressionKind, Visitor};

    let bump = Bump::new();
    let mut module = Module::new(&bump);
    let builder = IrBuilder::new(&bump);

    // Create a body with some constants we want to tag as booleans
    let c1 = builder.const_(Literal::I32(1));
    let c2 = builder.const_(Literal::I32(0));
    let c3 = builder.const_(Literal::I32(42)); // Not a bool

    let mut body_vec = bumpalo::collections::Vec::new_in(&bump);
    body_vec.push(c1);
    body_vec.push(c2);
    body_vec.push(c3);
    let body = builder.block(None, body_vec, Type::NONE);

    // Simulation of a pass
    struct BoolLiftingPass<'a> {
        to_tag: Vec<binaryen_ir::ExprRef<'a>>,
    }

    impl<'a> Visitor<'a> for BoolLiftingPass<'a> {
        fn visit_expression(&mut self, expr: &mut binaryen_ir::ExprRef<'a>) {
            if let ExpressionKind::Const(Literal::I32(v)) = &expr.kind {
                if *v == 0 || *v == 1 {
                    self.to_tag.push(*expr);
                }
            }
        }
    }

    let mut pass = BoolLiftingPass { to_tag: Vec::new() };
    let mut body_ref = body;
    pass.visit(&mut body_ref);

    for expr in pass.to_tag {
        module.set_annotation(expr, Annotation::Type(HighLevelType::Bool));
    }

    // Verify
    assert_eq!(
        module.get_annotations(c1).map(|a| a.high_level_type),
        Some(Some(HighLevelType::Bool))
    );
    assert_eq!(
        module.get_annotations(c2).map(|a| a.high_level_type),
        Some(Some(HighLevelType::Bool))
    );
    assert!(
        module.get_annotations(c3).is_none()
            || module
                .get_annotations(c3)
                .unwrap()
                .high_level_type
                .is_none()
    );
}

#[test]
fn test_annotation_stress() {
    let bump = Bump::new();
    let mut module = Module::new(&bump);
    let builder = IrBuilder::new(&bump);

    // Create 10,000 unique expressions and tag them
    let mut tagged_ids = std::collections::HashSet::new();
    for i in 0..10000 {
        let e = builder.const_(Literal::I32(i));
        module.set_annotation(e, Annotation::Type(HighLevelType::Pointer));
        tagged_ids.insert(e);
    }

    assert_eq!(module.annotations.len(), 10000);

    // Random check some
    for e in tagged_ids.iter().take(100) {
        assert_eq!(
            module.get_annotations(*e).map(|a| a.high_level_type),
            Some(Some(HighLevelType::Pointer))
        );
    }
}

#[test]
fn test_expr_pointer_uniqueness() {
    let bump = Bump::new();
    let mut module = Module::new(&bump);
    let builder = IrBuilder::new(&bump);

    // Two identical constants at different memory addresses
    let e1 = builder.const_(Literal::I32(0));
    let e2 = builder.const_(Literal::I32(0));

    assert_ne!(e1.as_ptr(), e2.as_ptr());
    assert_ne!(e1, e2); // ExprRef uses pointer equality

    module.set_annotation(e1, Annotation::Type(HighLevelType::Bool));

    assert_eq!(
        module.get_annotations(e1).map(|a| a.high_level_type),
        Some(Some(HighLevelType::Bool))
    );
    assert!(
        module.get_annotations(e2).is_none()
            || module
                .get_annotations(e2)
                .unwrap()
                .high_level_type
                .is_none()
    );
}

#[test]
fn test_pass_integration() {
    use binaryen_ir::{Pass, PassRunner, ReadOnlyVisitor};

    struct BooleanLiftingPass;
    impl Pass for BooleanLiftingPass {
        fn name(&self) -> &str {
            "bool-lifting"
        }
        fn run<'a>(&mut self, module: &mut Module<'a>) {
            let mut to_tag = Vec::new();

            struct Finder<'b, 'a>(&'b mut Vec<binaryen_ir::ExprRef<'a>>);
            impl<'b, 'a> ReadOnlyVisitor<'a> for Finder<'b, 'a> {
                fn visit_expression(&mut self, expr: binaryen_ir::ExprRef<'a>) {
                    if let binaryen_ir::ExpressionKind::Const(Literal::I32(v)) = &expr.kind {
                        if *v == 0 || *v == 1 {
                            self.0.push(expr);
                        }
                    }
                }
            }

            for func in &mut module.functions {
                if let Some(body) = func.body {
                    let mut finder = Finder(&mut to_tag);
                    finder.visit(body);
                }
            }

            for expr in to_tag {
                module.set_annotation(expr, Annotation::Type(HighLevelType::Bool));
            }
        }
    }

    let bump = Bump::new();
    let mut module = Module::new(&bump);
    let builder = IrBuilder::new(&bump);

    let c1 = builder.const_(Literal::I32(1));
    module.add_function(Function::new(
        "test".to_string(),
        Type::NONE,
        Type::NONE,
        vec![],
        Some(c1),
    ));

    let mut runner = PassRunner::new();
    runner.add(BooleanLiftingPass);
    runner.run(&mut module);

    assert_eq!(
        module.get_annotations(c1).map(|a| a.high_level_type),
        Some(Some(HighLevelType::Bool))
    );
}
