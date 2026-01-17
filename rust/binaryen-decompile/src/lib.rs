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
}
