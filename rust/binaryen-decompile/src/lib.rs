pub mod c_printer;
pub mod lifter;
pub mod passes;
pub mod rust_printer;

pub use c_printer::CPrinter;
pub use lifter::Lifter;
pub use rust_printer::RustPrinter;

#[cfg(test)]
mod tests {
    use super::*;
    use binaryen_core::{Literal, Type};
    use binaryen_ir::expression::IrBuilder;
    use binaryen_ir::module::Function;
    use bumpalo::Bump;

    #[test]
    fn test_identify_if_else() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // (block $L
        //   (br_if $L (i32.eq (local.get 0) (i32.const 0)))
        //   (call $log (local.get 0))
        // )

        let label = "L";
        let cond = builder.binary(
            binaryen_ir::BinaryOp::EqInt32,
            builder.local_get(0, Type::I32),
            builder.const_(Literal::I32(0)),
            Type::I32,
        );
        let br_if = builder.break_(label, Some(cond), None, Type::NONE);

        let mut call_ops = bumpalo::collections::Vec::new_in(&bump);
        call_ops.push(builder.local_get(0, Type::I32));
        let call = builder.call("log", call_ops, Type::NONE, false);

        let mut list = bumpalo::collections::Vec::new_in(&bump);
        list.push(br_if);
        list.push(call);

        let block = builder.block(Some(label), list, Type::NONE);

        let mut module = binaryen_ir::Module::new(&bump);
        module.functions.push(Function::new(
            "test".to_string(),
            Type::NONE,
            Type::NONE,
            vec![Type::I32],
            Some(block),
        ));

        // 1. Run the lifter
        let mut lifter = Lifter::new();
        lifter.run(&mut module);

        // 2. Print using a specific printer
        let mut printer = CPrinter::new(&module);
        let code = printer.print();

        println!("{}", code);
        assert!(code.contains("if (!"));
        assert!(code.contains("log"));
    }

    #[test]
    fn test_type_names_printing() {
        use binaryen_ir::pass::Pass;
        use binaryen_ir::passes::metadata::NameTypes;

        let bump = Bump::new();
        let mut module = binaryen_ir::Module::new(&bump);

        // Add a type: (i32) -> (i32)
        module.add_type(Type::I32, Type::I32);

        // Populate type names
        let mut name_types = NameTypes;
        name_types.run(&mut module);

        let mut func = Function::new("test_func".to_string(), Type::I32, Type::I32, vec![], None);
        func.type_idx = Some(0);
        module.functions.push(func);

        // Test C Printer
        let mut c_printer = CPrinter::new(&module);
        let c_code = c_printer.print();
        println!("C Output:\n{}", c_code);
        assert!(c_code.contains("// Types:"));
        assert!(c_code.contains("//   type$0 : (int32_t) -> (int32_t)"));
        assert!(c_code.contains("// type: type$0"));

        // Test Rust Printer
        let mut rust_printer = RustPrinter::new(&module);
        let rust_code = rust_printer.print();
        println!("Rust Output:\n{}", rust_code);
        assert!(rust_code.contains("// Types:"));
        assert!(rust_code.contains("//   type$0 : (i32) -> (i32)"));
        assert!(rust_code.contains("// type: type$0"));
    }

    #[test]
    fn test_call_indirect_printing() {
        use binaryen_core::type_store::intern_signature;
        use binaryen_ir::pass::Pass;
        use binaryen_ir::passes::metadata::NameTypes;
        use bumpalo::collections::Vec as BumpVec;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = binaryen_ir::Module::new(&bump);

        // 1. Create a module with a type (i32) -> (i32)
        module.add_type(Type::I32, Type::I32);
        let sig_type = intern_signature(Type::I32, Type::I32);

        // 2. Populate type names using NameTypes
        let mut name_types = NameTypes;
        name_types.run(&mut module);

        // 3. Create a function "caller" that uses CallIndirect with that type
        let target = builder.const_(Literal::I32(0)); // function index 0 (the target)
        let mut operands = BumpVec::new_in(&bump);
        operands.push(builder.const_(Literal::I32(42)));

        let call_indirect = builder.call_indirect(
            "0", // table index 0
            target,
            operands,
            sig_type,
            Type::I32, // return type
        );

        let caller_func = Function::new(
            "caller".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(call_indirect),
        );
        module.functions.push(caller_func);

        // Test C Printer
        let mut c_printer = CPrinter::new(&module);
        let c_code = c_printer.print();
        println!("C Output:\n{}", c_code);
        assert!(c_code.contains("call_indirect<type$0>"));

        // Test Rust Printer
        let mut rust_printer = RustPrinter::new(&module);
        let rust_code = rust_printer.print();
        println!("Rust Output:\n{}", rust_code);
        assert!(rust_code.contains("call_indirect<type$0>"));
    }
}
