pub mod binary_reader;
pub mod binary_writer;
pub mod expression;
pub mod module;
pub mod ops;
pub mod pass;
pub mod passes;
pub mod validation;
pub mod visitor;

pub use binary_reader::BinaryReader;
pub use binary_writer::BinaryWriter;
pub use expression::{ExprRef, Expression, ExpressionKind, IrBuilder};
pub use module::{Function, Module};
pub use ops::{BinaryOp, UnaryOp};
pub use pass::{Pass, PassRunner};
pub use validation::Validator;
pub use visitor::{ReadOnlyVisitor, Visitor};

#[cfg(test)]
mod tests {
    use super::*;
    use binaryen_core::{Literal, Type};
    use bumpalo::collections::Vec as BumpVec;
    use bumpalo::Bump;

    #[test]
    fn test_validation_failure() {
        let bump = Bump::new();
        let module_name = "test_module";

        // Create mismatched binary op: i32 + f32
        let left = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(1)),
            type_: Type::I32,
        });
        let right = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::F32(2.0)),
            type_: Type::F32,
        });

        let binary_expr = bump.alloc(Expression {
            kind: ExpressionKind::Binary {
                op: BinaryOp::AddInt32, // Note: using int32 add
                left,
                right,
            },
            type_: Type::I32, // Result type claimed to be i32
        });

        let mut functions = Vec::new();
        functions.push(Function {
            name: "bad_func".to_string(),
            params: Type::NONE,
            results: Type::I32,
            vars: Vec::new(),
            body: Some(binary_expr),
        });

        let module = Module {
            functions,
            globals: Vec::new(),
            memory: None,
            exports: Vec::new(),
        };

        let validator = Validator::new(&module);
        let (valid, errors) = validator.validate();

        assert!(!valid, "Validation should fail for mismatched types");
        assert!(errors.len() > 0);
        assert!(errors[0].contains("Binary op AddInt32 operands type mismatch"));
    }

    #[test]
    fn test_global_validation() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Helper to create standard globals
        let create_globals = || {
            vec![
                crate::module::Global {
                    name: "g0".to_string(),
                    type_: Type::I32,
                    mutable: false,
                    init: builder.const_(Literal::I32(0)),
                },
                crate::module::Global {
                    name: "g1".to_string(),
                    type_: Type::F32,
                    mutable: true,
                    init: builder.const_(Literal::F32(0.0)),
                },
            ]
        };

        // 1. Test GlobalSet on immutable global (g0)
        {
            let val = builder.const_(Literal::I32(42));
            let set_immutable = builder.global_set(0, val);

            let func = Function::new(
                "fail_immut".to_string(),
                Type::NONE,
                Type::NONE,
                vec![],
                Some(set_immutable),
            );

            let module = Module {
                functions: vec![func],
                globals: create_globals(),
                memory: None,
                exports: vec![],
            };

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid, "Setting immutable global should fail");
            assert!(errors.iter().any(|e| e.contains("is immutable")));
        }

        // 2. Test GlobalSet type mismatch
        {
            let val = builder.const_(Literal::I32(42)); // i32
            let set_mismatch = builder.global_set(1, val); // trying to set to g1 (f32)

            let func = Function::new(
                "fail_type".to_string(),
                Type::NONE,
                Type::NONE,
                vec![],
                Some(set_mismatch),
            );

            let module = Module {
                functions: vec![func],
                globals: create_globals(),
                memory: None,
                exports: vec![],
            };

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid, "GlobalSet type mismatch should fail");
            assert!(errors
                .iter()
                .any(|e| e.contains("Value type i32 does not match global type f32")));
        }

        // 3. Test GlobalGet out of bounds
        {
            let get_oob = builder.global_get(99, Type::I32);

            let func = Function::new(
                "fail_oob".to_string(),
                Type::NONE,
                Type::I32,
                vec![],
                Some(get_oob),
            );

            let module = Module {
                functions: vec![func],
                globals: create_globals(),
                memory: None,
                exports: vec![],
            };

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid, "OOB GlobalGet should fail");
            assert!(errors.iter().any(|e| e.contains("out of bounds")));
        }
    }

    #[test]
    fn test_ir_construction() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        let const_expr = builder.const_(Literal::I32(42));

        match const_expr.kind {
            ExpressionKind::Const(Literal::I32(42)) => (),
            _ => panic!("Expected Const(42)"),
        }
        assert_eq!(const_expr.type_, Type::I32);

        let mut list = BumpVec::new_in(&bump);
        list.push(const_expr);

        let block = builder.block(Some("my_block"), list, Type::I32);

        if let ExpressionKind::Block { name, list } = &block.kind {
            assert_eq!(*name, Some("my_block"));
            assert_eq!(list.len(), 1);
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_binary_op() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        let left = builder.const_(Literal::I32(10));
        let right = builder.const_(Literal::I32(32));

        let add = builder.binary(BinaryOp::AddInt32, left, right, Type::I32);

        if let ExpressionKind::Binary { op, left, right } = &add.kind {
            assert_eq!(*op, BinaryOp::AddInt32);
            assert_eq!(left.type_, Type::I32);
            assert_eq!(right.type_, Type::I32);
        } else {
            panic!("Expected Binary");
        }
    }

    struct CountVisitor {
        count: usize,
    }

    impl<'a> Visitor<'a> for CountVisitor {
        fn visit_expression(&mut self, _expr: &mut Expression<'a>) {
            self.count += 1;
        }
    }

    #[test]
    fn test_visitor() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        let c1 = builder.const_(Literal::I32(1));
        let c2 = builder.const_(Literal::I32(2));
        let add = builder.binary(BinaryOp::AddInt32, c1, c2, Type::I32);

        let mut list = BumpVec::new_in(&bump);
        list.push(add);
        let block = builder.block(None, list, Type::I32);

        let mut v = CountVisitor { count: 0 };
        v.visit(block);

        // Block (1) -> Add (1) -> Const (1) + Const (1) = 4 expressions
        assert_eq!(v.count, 4);
    }

    #[test]
    fn test_module_construction() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // fn add_one(x: i32) -> i32
        let local_get = builder.local_get(0, Type::I32);
        let const_1 = builder.const_(Literal::I32(1));
        let add = builder.binary(BinaryOp::AddInt32, local_get, const_1, Type::I32);

        let func = Function::new(
            "add_one".to_string(),
            Type::I32,
            Type::I32,
            vec![],
            Some(add),
        );

        let mut module = Module::new();
        module.add_function(func);

        assert!(module.get_function("add_one").is_some());

        let f = module.get_function("add_one").unwrap();
        if let Some(body) = &f.body {
            if let ExpressionKind::Binary { op, .. } = body.kind {
                assert_eq!(op, BinaryOp::AddInt32);
            } else {
                panic!("Expected Binary");
            }
        }
    }

    #[test]
    fn test_export_validation() {
        use crate::module::{Export, ExportKind, Global, MemoryLimits};

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Setup a basic valid module components
        let func = Function::new("f0".to_string(), Type::NONE, Type::NONE, vec![], None);

        let global_init = builder.const_(Literal::I32(0));
        let global = Global {
            name: "g0".to_string(),
            type_: Type::I32,
            mutable: false,
            init: global_init,
        };

        let memory = MemoryLimits {
            initial: 1,
            maximum: None,
        };

        // 1. Valid exports
        {
            // func needs to be cloned or recreated because Function doesn't implement Clone
            let func_valid = Function::new("f0".to_string(), Type::NONE, Type::NONE, vec![], None);

            let module = Module {
                functions: vec![func_valid],  // f0 is index 0
                globals: vec![global],        // g0 is index 0
                memory: Some(memory.clone()), // memory is index 0
                exports: vec![
                    Export {
                        name: "exp_func".to_string(),
                        kind: ExportKind::Function,
                        index: 0,
                    },
                    Export {
                        name: "exp_glob".to_string(),
                        kind: ExportKind::Global,
                        index: 0,
                    },
                    Export {
                        name: "exp_mem".to_string(),
                        kind: ExportKind::Memory,
                        index: 0,
                    },
                ],
            };

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(valid, "Valid exports failed: {:?}", errors);
        }

        // 2. Duplicate export name
        {
            let module = Module {
                functions: vec![],
                globals: vec![],
                memory: None,
                exports: vec![
                    Export {
                        name: "same".to_string(),
                        kind: ExportKind::Function,
                        index: 0,
                    },
                    Export {
                        name: "same".to_string(),
                        kind: ExportKind::Function,
                        index: 0,
                    },
                ],
            };
            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid);
            assert!(errors.iter().any(|e| e.contains("Duplicate export name")));
        }

        // 3. Function OOB
        {
            let func_valid = Function::new("f0".to_string(), Type::NONE, Type::NONE, vec![], None);
            let module = Module {
                functions: vec![func_valid],
                globals: vec![],
                memory: None,
                exports: vec![Export {
                    name: "f1".to_string(),
                    kind: ExportKind::Function,
                    index: 1,
                }],
            };
            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid);
            assert!(errors
                .iter()
                .any(|e| e.contains("Exported function index 1 out of bounds")));
        }

        // 4. Global OOB
        {
            let module = Module {
                functions: vec![],
                globals: vec![],
                memory: None,
                exports: vec![Export {
                    name: "g0".to_string(),
                    kind: ExportKind::Global,
                    index: 0,
                }],
            };
            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid);
            assert!(errors
                .iter()
                .any(|e| e.contains("Exported global index 0 out of bounds")));
        }

        // 5. Memory OOB / No Memory
        {
            let module = Module {
                functions: vec![],
                globals: vec![],
                memory: None,
                exports: vec![Export {
                    name: "m0".to_string(),
                    kind: ExportKind::Memory,
                    index: 0,
                }],
            };
            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid);
            assert!(errors
                .iter()
                .any(|e| e.contains("Exported memory but no memory exists")));
        }
    }

    #[test]
    fn test_exports_roundtrip() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;
        use crate::module::{Export, ExportKind, Global, MemoryLimits};

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        let func = Function::new("f0".to_string(), Type::NONE, Type::NONE, vec![], None);
        let global = Global {
            name: "g0".to_string(),
            type_: Type::I32,
            mutable: false,
            init: builder.const_(Literal::I32(123)),
        };
        let memory = MemoryLimits {
            initial: 1,
            maximum: None,
        };

        let module = Module {
            functions: vec![func],
            globals: vec![global],
            memory: Some(memory),
            exports: vec![
                Export {
                    name: "test_func".to_string(),
                    kind: ExportKind::Function,
                    index: 0,
                },
                Export {
                    name: "test_glob".to_string(),
                    kind: ExportKind::Global,
                    index: 0,
                },
                Export {
                    name: "test_mem".to_string(),
                    kind: ExportKind::Memory,
                    index: 0,
                },
            ],
        };

        // Write
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("write failed");

        // Read
        let read_bump = Bump::new();
        let mut reader = BinaryReader::new(&read_bump, bytes);
        let read_module = reader.parse_module().expect("parse failed");

        // Verify
        assert_eq!(read_module.exports.len(), 3);

        let func_exp = read_module
            .exports
            .iter()
            .find(|e| e.name == "test_func")
            .unwrap();
        assert_eq!(func_exp.kind, ExportKind::Function);
        assert_eq!(func_exp.index, 0);

        let glob_exp = read_module
            .exports
            .iter()
            .find(|e| e.name == "test_glob")
            .unwrap();
        assert_eq!(glob_exp.kind, ExportKind::Global);
        assert_eq!(glob_exp.index, 0);

        let mem_exp = read_module
            .exports
            .iter()
            .find(|e| e.name == "test_mem")
            .unwrap();
        assert_eq!(mem_exp.kind, ExportKind::Memory);
        assert_eq!(mem_exp.index, 0);
    }
}
