use binaryen_core::{Literal, Type};
use binaryen_ir::ops::BinaryOp;
use binaryen_ir::passes::precompute::Precompute;
use binaryen_ir::passes::simplify_identity::SimplifyIdentity;
use binaryen_ir::passes::untee::Untee;
use binaryen_ir::{Expression, ExpressionKind, Function, Module, Pass};
use bumpalo::Bump;

#[test]
fn test_untee_then_simplify() {
    let bump = Bump::new();

    // Create: (i32.add (local.tee $0 (i32.const 5)) (i32.const 0))
    // After untee: (i32.add (block (local.set $0 (i32.const 5)) (local.get $0)) (i32.const 0))
    // After simplify-identity: (block (local.set $0 (i32.const 5)) (local.get $0))

    let five = Expression::const_expr(&bump, Literal::I32(5), Type::I32);
    let tee = Expression::local_tee(&bump, 0, five, Type::I32);
    let zero = Expression::const_expr(&bump, Literal::I32(0), Type::I32);
    let add = Expression::new(
        &bump,
        ExpressionKind::Binary {
            op: BinaryOp::AddInt32,
            left: tee,
            right: zero,
        },
        Type::I32,
    );

    let func = Function::new(
        "test".to_string(),
        Type::NONE,
        Type::I32,
        vec![Type::I32],
        Some(add),
    );

    let mut module = Module::new(&bump);
    module.add_function(func);

    // Run untee pass
    let mut untee_pass = Untee;
    untee_pass.run(&mut module);

    // Verify tee was converted
    let body = module.functions[0].body.as_ref().unwrap();
    if let ExpressionKind::Binary { left, .. } = &body.kind {
        // Left should now be a block
        assert!(matches!(left.kind, ExpressionKind::Block { .. }));
    }

    // Run simplify-identity pass
    let mut simplify_pass = SimplifyIdentity;
    simplify_pass.run(&mut module);

    // After simplify, the add with 0 should be gone
    let body = module.functions[0].body.as_ref().unwrap();
    // Should be just the block now (x + 0 simplified to x)
    assert!(matches!(body.kind, ExpressionKind::Block { .. }));
}

#[test]
fn test_precompute_after_untee() {
    let bump = Bump::new();

    // Create: (i32.add (i32.const 10) (local.tee $0 (i32.const 20)))
    // After untee: (i32.add (i32.const 10) (block (local.set $0 (i32.const 20)) (local.get $0)))
    // Precompute won't change this (can't fold through block)

    let ten = Expression::const_expr(&bump, Literal::I32(10), Type::I32);
    let twenty = Expression::const_expr(&bump, Literal::I32(20), Type::I32);
    let tee = Expression::local_tee(&bump, 0, twenty, Type::I32);
    let add = Expression::new(
        &bump,
        ExpressionKind::Binary {
            op: BinaryOp::AddInt32,
            left: ten,
            right: tee,
        },
        Type::I32,
    );

    let func = Function::new(
        "test".to_string(),
        Type::NONE,
        Type::I32,
        vec![Type::I32],
        Some(add),
    );

    let mut module = Module::new(&bump);
    module.add_function(func);

    // Run pipeline
    let mut untee_pass = Untee;
    untee_pass.run(&mut module);

    let mut precompute_pass = Precompute;
    precompute_pass.run(&mut module);

    // Verify structure is maintained
    let body = module.functions[0].body.as_ref().unwrap();
    assert!(matches!(body.kind, ExpressionKind::Binary { .. }));
}

#[test]
fn test_multiple_passes_composability() {
    let bump = Bump::new();

    // Build a complex expression with multiple optimization opportunities
    let const1 = Expression::const_expr(&bump, Literal::I32(1), Type::I32);
    let const2 = Expression::const_expr(&bump, Literal::I32(2), Type::I32);

    // (i32.add (i32.const 1) (i32.const 2)) => should fold to 3
    let add = Expression::new(
        &bump,
        ExpressionKind::Binary {
            op: BinaryOp::AddInt32,
            left: const1,
            right: const2,
        },
        Type::I32,
    );

    let func = Function::new("test".to_string(), Type::NONE, Type::I32, vec![], Some(add));

    let mut module = Module::new(&bump);
    module.add_function(func);

    // Run precompute
    let mut pass = Precompute;
    pass.run(&mut module);

    // Should be folded to const 3
    let body = module.functions[0].body.as_ref().unwrap();
    assert!(matches!(body.kind, ExpressionKind::Const(Literal::I32(3))));
}
