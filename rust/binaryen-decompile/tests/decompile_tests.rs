use binaryen_core::{Literal, Type};
use binaryen_decompile::{CPrinter, Lifter};
use binaryen_ir::{Annotation, BinaryOp, HighLevelType, IrBuilder, LoopType, Module};
use bumpalo::collections::Vec as BumpVec;
use bumpalo::Bump;

#[test]
fn test_boolean_lifting() {
    let allocator = Bump::new();
    let mut module = Module::new(&allocator);
    let builder = IrBuilder::new(&allocator);

    // Create a function that returns (1 == 2)
    let left = builder.const_(Literal::I32(1));
    let right = builder.const_(Literal::I32(2));
    let eq = builder.binary(BinaryOp::EqInt32, left, right, Type::I32);

    let func =
        binaryen_ir::Function::new("test".to_string(), Type::NONE, Type::I32, vec![], Some(eq));
    module.functions.push(func);

    // 1. Run the lifter
    let mut lifter = Lifter::new();
    lifter.run(&mut module);

    // Should have a Bool annotation now
    let ann = module.get_annotations(eq).expect("Should have annotation");
    assert_eq!(ann.high_level_type, Some(HighLevelType::Bool));

    // 2. Decompile
    let mut printer = CPrinter::new(&module);
    let output = printer.print();
    println!("Output:\n{}", output);

    // Should contain the relational op in a high-level way
    assert!(output.contains(" == "));
}

#[test]
fn test_loop_lifting_do_while() {
    let allocator = Bump::new();
    let mut module = Module::new(&allocator);
    let builder = IrBuilder::new(&allocator);

    // (loop $L (block (br_if $L (i32.const 1))))
    let loop_name = "L";
    let cond = builder.const_(Literal::I32(1));
    let br_if = builder.break_(loop_name, Some(cond), None, Type::NONE);

    let mut list = BumpVec::new_in(&allocator);
    list.push(br_if);
    let body = builder.block(None, list, Type::NONE);

    let loop_expr = builder.loop_(Some(loop_name), body, Type::NONE);

    let func = binaryen_ir::Function::new(
        "test".to_string(),
        Type::NONE,
        Type::NONE,
        vec![],
        Some(loop_expr),
    );
    module.functions.push(func);

    {
        let mut lifter = Lifter::new();
        lifter.run(&mut module);

        let ann = module
            .get_annotations(loop_expr)
            .expect("Should have annotation");
        assert_eq!(ann.loop_type, Some(LoopType::DoWhile));

        let mut printer = CPrinter::new(&module);
        let output = printer.print();
        println!("Output:\n{}", output);
        assert!(output.contains("do-while L"));
    }
}

#[test]
fn test_pointer_lifting() {
    let allocator = Bump::new();
    let mut module = Module::new(&allocator);
    let builder = IrBuilder::new(&allocator);

    // (i32.load (local.get 0))
    let ptr_expr = builder.local_get(0, Type::I32);
    let load = builder.load(4, false, 0, 0, ptr_expr, Type::I32);

    let func =
        binaryen_ir::Function::new("test".to_string(), Type::I32, Type::I32, vec![], Some(load));
    module.functions.push(func);

    {
        let mut lifter = Lifter::new();
        lifter.run(&mut module);

        let ann = module
            .get_annotations(ptr_expr)
            .expect("Should have annotation");
        assert_eq!(ann.high_level_type, Some(HighLevelType::Pointer));

        let mut printer = CPrinter::new(&module);
        let output = printer.print();
        println!("Output:\n{}", output);
        // Pointer lifting should turn Load(p0) into *(p0)
        assert!(output.contains("*(ptr_0)"));
    }
}

#[test]
fn test_expression_recombination() {
    let allocator = Bump::new();
    let mut module = Module::new(&allocator);
    let builder = IrBuilder::new(&allocator);

    // (local.set 1 (i32.add (local.get 0) (i32.const 10)))
    // (local.get 1)
    let add_expr = builder.binary(
        BinaryOp::AddInt32,
        builder.local_get(0, Type::I32),
        builder.const_(Literal::I32(10)),
        Type::I32,
    );
    let set = builder.local_set(1, add_expr);
    let load = builder.load(4, false, 0, 0, builder.local_get(1, Type::I32), Type::I32);

    let mut list = BumpVec::new_in(&allocator);
    list.push(set);
    list.push(load);
    let block = builder.block(None, list, Type::I32);

    let func = binaryen_ir::Function::new(
        "test".to_string(),
        Type::I32,
        Type::I32,
        vec![Type::I32], // local 1
        Some(block),
    );
    module.functions.push(func);

    {
        let mut lifter = Lifter::new();
        lifter.run(&mut module);

        let mut printer = CPrinter::new(&module);
        let output = printer.print();
        println!("Output:\n{}", output);

        // Should NOT contain 'p1 = ...'
        assert!(!output.contains("ptr_1 = "));

        // Should contain Load with inlined addition
        assert!(output.contains("*((i_0 + 10))"));
    }
}
