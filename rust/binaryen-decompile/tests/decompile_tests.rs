use binaryen_ir::{Module, IrBuilder, HighLevelType, Annotation, BinaryOp};
use binaryen_decompile::Decompiler;
use binaryen_core::{Type, Literal};
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
    
    let func = binaryen_ir::Function::new(
        "test".to_string(),
        Type::NONE,
        Type::I32,
        vec![],
        Some(eq),
    );
    module.functions.push(func);
    
    // Use a scope to manage the mutable borrow of 'module' by 'decompiler'
    {
        let mut decompiler = Decompiler::new(&mut module);
        
        // Lift booleans
        decompiler.lift();
        
        // Should have a Bool annotation now
        let ann = decompiler.module.get_annotation(eq).expect("Should have annotation");
        assert_eq!(*ann, Annotation::Type(HighLevelType::Bool));
        
        // Decompile
        let output = decompiler.decompile();
        println!("{}", output);
        
        // Output should contain the annotation (since our placeholder printer shows them)
        assert!(output.contains("/* Type(Bool) */"));
        assert!(output.contains("EqInt32"));
    }
}
