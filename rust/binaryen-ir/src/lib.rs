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
            imports: vec![],
            functions,
            globals: Vec::new(),
            memory: None,
            start: None,
            exports: Vec::new(),
            data: Vec::new(),
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
                imports: vec![],
                functions: vec![func],
                globals: create_globals(),
                memory: None,
                start: None,
                exports: vec![],
                data: Vec::new(),
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
                imports: vec![],
                functions: vec![func],
                globals: create_globals(),
                memory: None,
                start: None,
                exports: vec![],
                data: Vec::new(),
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
                imports: vec![],
                functions: vec![func],
                globals: create_globals(),
                memory: None,
                start: None,
                exports: vec![],
                data: Vec::new(),
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
                imports: vec![],
                functions: vec![func_valid],  // f0 is index 0
                globals: vec![global],        // g0 is index 0
                memory: Some(memory.clone()), // memory is index 0
                start: None,
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
                data: Vec::new(),
            };

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(valid, "Valid exports failed: {:?}", errors);
        }

        // 2. Duplicate export name
        {
            let module = Module {
                imports: vec![],
                functions: vec![],
                globals: vec![],
                memory: None,
                start: None,
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
                data: Vec::new(),
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
                imports: vec![],
                functions: vec![func_valid],
                globals: vec![],
                memory: None,
                start: None,
                exports: vec![Export {
                    name: "f1".to_string(),
                    kind: ExportKind::Function,
                    index: 1,
                }],
                data: Vec::new(),
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
                imports: vec![],
                functions: vec![],
                globals: vec![],
                memory: None,
                start: None,
                exports: vec![Export {
                    name: "g0".to_string(),
                    kind: ExportKind::Global,
                    index: 0,
                }],
                data: Vec::new(),
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
                imports: vec![],
                functions: vec![],
                globals: vec![],
                memory: None,
                start: None,
                exports: vec![Export {
                    name: "m0".to_string(),
                    kind: ExportKind::Memory,
                    index: 0,
                }],
                data: Vec::new(),
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
            imports: vec![],
            functions: vec![func],
            globals: vec![global],
            memory: Some(memory),
            start: None,
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
            data: Vec::new(),
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

    #[test]
    fn test_imports_roundtrip() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;
        use crate::module::{Import, ImportKind, MemoryLimits};

        let _bump = Bump::new();

        let mut module = Module::new();

        // 1. Function Import (param: i32, result: none)
        module.add_import(Import {
            module: "env".to_string(),
            name: "log".to_string(),
            kind: ImportKind::Function(Type::I32, Type::NONE),
        });

        // 2. Global Import (i32, immutable)
        module.add_import(Import {
            module: "env".to_string(),
            name: "limit".to_string(),
            kind: ImportKind::Global(Type::I32, false),
        });

        // 3. Memory Import
        module.add_import(Import {
            module: "env".to_string(),
            name: "memory".to_string(),
            kind: ImportKind::Memory(MemoryLimits {
                initial: 1,
                maximum: Some(2),
            }),
        });

        // Write
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("write failed");

        // Read
        let read_bump = Bump::new();
        let mut reader = BinaryReader::new(&read_bump, bytes);
        let read_module = reader.parse_module().expect("parse failed");

        assert_eq!(read_module.imports.len(), 3);

        let log_import = &read_module.imports[0];
        assert_eq!(log_import.module, "env");
        assert_eq!(log_import.name, "log");
        if let ImportKind::Function(p, r) = log_import.kind {
            assert_eq!(p, Type::I32);
            assert_eq!(r, Type::NONE);
        } else {
            panic!("Expected function import");
        }

        let global_import = &read_module.imports[1];
        assert_eq!(global_import.name, "limit");
        if let ImportKind::Global(ty, mut_) = global_import.kind {
            assert_eq!(ty, Type::I32);
            assert_eq!(mut_, false);
        } else {
            panic!("Expected global import");
        }
    }

    #[test]
    fn test_import_validation() {
        use crate::module::{Global, Import, ImportKind};

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        let mut module = Module::new();

        // Import Global 0: i32, immutable
        module.add_import(Import {
            module: "env".to_string(),
            name: "g_imp".to_string(),
            kind: ImportKind::Global(Type::I32, false),
        });

        // Define Global 1: f32, mutable
        let init_expr = builder.const_(Literal::F32(1.0));
        module.add_global(Global {
            name: "g_def".to_string(),
            type_: Type::F32,
            mutable: true,
            init: init_expr,
        });

        // Test 1: Get imported global (index 0) - Valid
        {
            let get_imp = builder.global_get(0, Type::I32);
            let func = Function::new(
                "test1".to_string(),
                Type::NONE,
                Type::I32,
                vec![],
                Some(get_imp),
            );
            module.functions.push(func);

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(valid, "Get imported global failed: {:?}", errors);
            module.functions.pop();
        }

        // Test 2: Get defined global (index 1) - Valid
        {
            let get_def = builder.global_get(1, Type::F32);
            let func = Function::new(
                "test2".to_string(),
                Type::NONE,
                Type::F32,
                vec![],
                Some(get_def),
            );
            module.functions.push(func);

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(valid, "Get defined global failed: {:?}", errors);
            module.functions.pop();
        }

        // Test 3: Get OOB global (index 2) - Invalid
        {
            let get_oob = builder.global_get(2, Type::I32);
            let func = Function::new(
                "test3".to_string(),
                Type::NONE,
                Type::I32,
                vec![],
                Some(get_oob),
            );
            module.functions.push(func);

            let validator = Validator::new(&module);
            let (valid, _) = validator.validate();
            assert!(!valid, "OOB global get should fail");
            module.functions.pop();
        }
    }

    #[test]
    fn test_data_section_roundtrip() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;
        use crate::module::{DataSegment, MemoryLimits};

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        let mut module = Module::new();

        // Add memory
        module.set_memory(1, Some(10));

        // Data segment 1: "Hello" at offset 0
        let offset1 = builder.const_(Literal::I32(0));
        module.add_data_segment(DataSegment {
            memory_index: 0,
            offset: offset1,
            data: b"Hello".to_vec(),
        });

        // Data segment 2: "World" at offset 100
        let offset2 = builder.const_(Literal::I32(100));
        module.add_data_segment(DataSegment {
            memory_index: 0,
            offset: offset2,
            data: b"World".to_vec(),
        });

        // Write
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("write failed");

        // Read
        let read_bump = Bump::new();
        let mut reader = BinaryReader::new(&read_bump, bytes);
        let read_module = reader.parse_module().expect("parse failed");

        // Verify
        assert_eq!(read_module.data.len(), 2);

        let seg1 = &read_module.data[0];
        assert_eq!(seg1.memory_index, 0);
        assert_eq!(seg1.data, b"Hello");
        if let ExpressionKind::Const(Literal::I32(val)) = seg1.offset.kind {
            assert_eq!(val, 0);
        } else {
            panic!("Expected const offset");
        }

        let seg2 = &read_module.data[1];
        assert_eq!(seg2.memory_index, 0);
        assert_eq!(seg2.data, b"World");
        if let ExpressionKind::Const(Literal::I32(val)) = seg2.offset.kind {
            assert_eq!(val, 100);
        } else {
            panic!("Expected const offset");
        }
    }

    #[test]
    fn test_data_section_validation() {
        use crate::module::{DataSegment, MemoryLimits};

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test 1: Valid data segment
        {
            let mut module = Module::new();
            module.set_memory(1, None);

            let offset = builder.const_(Literal::I32(0));
            module.add_data_segment(DataSegment {
                memory_index: 0,
                offset,
                data: b"test".to_vec(),
            });

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(valid, "Valid data segment failed: {:?}", errors);
        }

        // Test 2: Data segment without memory
        {
            let mut module = Module::new();
            // No memory defined

            let offset = builder.const_(Literal::I32(0));
            module.add_data_segment(DataSegment {
                memory_index: 0,
                offset,
                data: b"test".to_vec(),
            });

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid, "Data without memory should fail");
            assert!(errors.iter().any(|e| e.contains("no memory exists")));
        }

        // Test 3: Invalid memory index
        {
            let mut module = Module::new();
            module.set_memory(1, None);

            let offset = builder.const_(Literal::I32(0));
            module.add_data_segment(DataSegment {
                memory_index: 1, // Invalid: only 0 allowed
                offset,
                data: b"test".to_vec(),
            });

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid, "Invalid memory index should fail");
            assert!(errors.iter().any(|e| e.contains("invalid memory index")));
        }
    }

    #[test]
    fn test_start_section_roundtrip() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        let mut module = Module::new();

        // Add a function to be the start function
        let body = builder.const_(Literal::I32(42));
        let func = Function::new(
            "init".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(body),
        );
        module.add_function(func);

        // Set as start function (index 0)
        module.set_start(0);

        // Write
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("write failed");

        // Read
        let read_bump = Bump::new();
        let mut reader = BinaryReader::new(&read_bump, bytes);
        let read_module = reader.parse_module().expect("parse failed");

        // Verify
        assert_eq!(read_module.start, Some(0));
    }

    #[test]
    fn test_start_section_validation() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test 1: Valid start function (no params, no results)
        {
            let mut module = Module::new();

            let const_expr = builder.const_(Literal::I32(0));
            let body = builder.drop(const_expr); // Drop the i32 to get Type::NONE
            let func = Function::new(
                "start".to_string(),
                Type::NONE,
                Type::NONE,
                vec![],
                Some(body),
            );
            module.add_function(func);
            module.set_start(0);

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(valid, "Valid start function failed: {:?}", errors);
        }

        // Test 2: Start function out of bounds
        {
            let mut module = Module::new();
            module.set_start(99); // No functions exist

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid, "OOB start function should fail");
            assert!(errors.iter().any(|e| e.contains("out of bounds")));
        }

        // Test 3: Start function with parameters (invalid)
        {
            let mut module = Module::new();

            let body = builder.local_get(0, Type::I32);
            let func = Function::new(
                "bad_start".to_string(),
                Type::I32, // Has parameter
                Type::NONE,
                vec![],
                Some(body),
            );
            module.add_function(func);
            module.set_start(0);

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid, "Start with params should fail");
            assert!(errors.iter().any(|e| e.contains("no parameters")));
        }

        // Test 4: Start function with results (invalid)
        {
            let mut module = Module::new();

            let body = builder.const_(Literal::I32(42));
            let func = Function::new(
                "bad_start".to_string(),
                Type::NONE,
                Type::I32, // Has result
                vec![],
                Some(body),
            );
            module.add_function(func);
            module.set_start(0);

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid, "Start with results should fail");
            assert!(errors.iter().any(|e| e.contains("no results")));
        }
    }
}
