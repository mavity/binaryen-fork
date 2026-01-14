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
        let _module_name = "test_module";

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

        let functions = vec![Function {
            name: "bad_func".to_string(),
            type_idx: None,
            params: Type::NONE,
            results: Type::I32,
            vars: Vec::new(),
            body: Some(binary_expr),
        }];

        let module = Module {
            types: vec![],
            imports: vec![],
            functions,
            globals: Vec::new(),
            table: None,
            memory: None,
            start: None,
            exports: Vec::new(),
            elements: Vec::new(),
            data: Vec::new(),
        };

        let validator = Validator::new(&module);
        let (valid, errors) = validator.validate();

        assert!(!valid, "Validation should fail for mismatched types");
        assert!(!errors.is_empty());
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
                types: vec![],
                imports: vec![],
                functions: vec![func],
                globals: create_globals(),
                memory: None,
                start: None,
                table: None,
                elements: Vec::new(),
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
                types: vec![],
                imports: vec![],
                functions: vec![func],
                globals: create_globals(),
                memory: None,
                start: None,
                table: None,
                elements: Vec::new(),
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
                types: vec![],
                imports: vec![],
                functions: vec![func],
                globals: create_globals(),
                memory: None,
                start: None,
                table: None,
                elements: Vec::new(),
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
        let _func = Function::new("f0".to_string(), Type::NONE, Type::NONE, vec![], None);

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
                types: vec![],
                imports: vec![],
                functions: vec![func_valid],  // f0 is index 0
                globals: vec![global],        // g0 is index 0
                memory: Some(memory.clone()), // memory is index 0
                start: None,
                table: None,
                elements: Vec::new(),
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
                types: vec![],
                imports: vec![],
                functions: vec![],
                globals: vec![],
                memory: None,
                start: None,
                table: None,
                elements: Vec::new(),
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
                types: vec![],
                imports: vec![],
                functions: vec![func_valid],
                globals: vec![],
                memory: None,
                start: None,
                table: None,
                elements: Vec::new(),
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
                types: vec![],
                imports: vec![],
                functions: vec![],
                globals: vec![],
                memory: None,
                start: None,
                table: None,
                elements: Vec::new(),
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
                types: vec![],
                imports: vec![],
                functions: vec![],
                globals: vec![],
                memory: None,
                start: None,
                table: None,
                elements: Vec::new(),
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
            types: vec![],
            imports: vec![],
            functions: vec![func],
            globals: vec![global],
            memory: Some(memory),
            start: None,
            table: None,
            elements: Vec::new(),
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
            assert!(!mut_);
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
        use crate::module::DataSegment;

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
        use crate::module::DataSegment;

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

    #[test]
    fn test_table_and_element_roundtrip() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;
        use crate::module::ElementSegment;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        let mut module = Module::new();

        // Add functions that will be referenced in the element segment
        let func1 = Function::new("f0".to_string(), Type::NONE, Type::NONE, vec![], None);
        let func2 = Function::new("f1".to_string(), Type::NONE, Type::NONE, vec![], None);
        module.add_function(func1);
        module.add_function(func2);

        // Add table
        module.set_table(Type::FUNCREF, 10, Some(20));

        // Add element segment: initialize table[0..2] with functions [0, 1]
        let offset = builder.const_(Literal::I32(0));
        module.add_element_segment(ElementSegment {
            table_index: 0,
            offset,
            func_indices: vec![0, 1],
        });

        // Write
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("write failed");

        // Read
        let read_bump = Bump::new();
        let mut reader = BinaryReader::new(&read_bump, bytes);
        let read_module = reader.parse_module().expect("parse failed");

        // Verify table
        assert!(read_module.table.is_some());
        let table = read_module.table.as_ref().unwrap();
        assert_eq!(table.element_type, Type::FUNCREF);
        assert_eq!(table.initial, 10);
        assert_eq!(table.maximum, Some(20));

        // Verify element segments
        assert_eq!(read_module.elements.len(), 1);
        let elem = &read_module.elements[0];
        assert_eq!(elem.table_index, 0);
        assert_eq!(elem.func_indices, vec![0, 1]);
        if let ExpressionKind::Const(Literal::I32(val)) = elem.offset.kind {
            assert_eq!(val, 0);
        } else {
            panic!("Expected const offset");
        }
    }

    #[test]
    fn test_table_and_element_validation() {
        use crate::module::ElementSegment;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test 1: Valid table and element segment
        {
            let mut module = Module::new();

            let func = Function::new("f0".to_string(), Type::NONE, Type::NONE, vec![], None);
            module.add_function(func);

            module.set_table(Type::FUNCREF, 5, None);

            let offset = builder.const_(Literal::I32(0));
            module.add_element_segment(ElementSegment {
                table_index: 0,
                offset,
                func_indices: vec![0],
            });

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(valid, "Valid table/element failed: {:?}", errors);
        }

        // Test 2: Element segment without table
        {
            let mut module = Module::new();

            let func = Function::new("f0".to_string(), Type::NONE, Type::NONE, vec![], None);
            module.add_function(func);

            let offset = builder.const_(Literal::I32(0));
            module.add_element_segment(ElementSegment {
                table_index: 0,
                offset,
                func_indices: vec![0],
            });

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid, "Element without table should fail");
            assert!(errors.iter().any(|e| e.contains("no table exists")));
        }

        // Test 3: Element with invalid function index
        {
            let mut module = Module::new();

            module.set_table(Type::FUNCREF, 5, None);

            let offset = builder.const_(Literal::I32(0));
            module.add_element_segment(ElementSegment {
                table_index: 0,
                offset,
                func_indices: vec![99], // Out of bounds
            });

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid, "Element with OOB function should fail");
            assert!(errors.iter().any(|e| e.contains("out of bounds")));
        }

        // Test 4: Invalid table index in element segment
        {
            let mut module = Module::new();

            let func = Function::new("f0".to_string(), Type::NONE, Type::NONE, vec![], None);
            module.add_function(func);

            module.set_table(Type::FUNCREF, 5, None);

            let offset = builder.const_(Literal::I32(0));
            module.add_element_segment(ElementSegment {
                table_index: 1, // Invalid: only 0 allowed
                offset,
                func_indices: vec![0],
            });

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid, "Invalid table index should fail");
            assert!(errors.iter().any(|e| e.contains("invalid table index")));
        }
    }

    #[test]
    fn test_type_section_roundtrip() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();

        // Test 1: Empty types
        {
            let module = Module::new();

            let mut writer = BinaryWriter::new();
            let bytes = writer.write_module(&module).expect("Failed to write");

            let mut reader = BinaryReader::new(&bump, bytes);
            let parsed = reader.parse_module().expect("Failed to parse");

            assert_eq!(parsed.types.len(), 0);
        }

        // Test 2: Single type (i32) -> (i32)
        {
            let mut module = Module::new();
            module.add_type(Type::I32, Type::I32);

            let mut writer = BinaryWriter::new();
            let bytes = writer.write_module(&module).expect("Failed to write");

            let mut reader = BinaryReader::new(&bump, bytes);
            let parsed = reader.parse_module().expect("Failed to parse");

            assert_eq!(parsed.types.len(), 1);
            assert_eq!(parsed.types[0].params, Type::I32);
            assert_eq!(parsed.types[0].results, Type::I32);
        }

        // Test 3: Multiple types
        {
            let mut module = Module::new();
            module.add_type(Type::I32, Type::I32);
            module.add_type(Type::I32, Type::NONE);
            module.add_type(Type::NONE, Type::F64);

            let mut writer = BinaryWriter::new();
            let bytes = writer.write_module(&module).expect("Failed to write");

            let mut reader = BinaryReader::new(&bump, bytes);
            let parsed = reader.parse_module().expect("Failed to parse");

            assert_eq!(parsed.types.len(), 3);
            assert_eq!(parsed.types[0].params, Type::I32);
            assert_eq!(parsed.types[0].results, Type::I32);
            assert_eq!(parsed.types[1].params, Type::I32);
            assert_eq!(parsed.types[1].results, Type::NONE);
            assert_eq!(parsed.types[2].params, Type::NONE);
            assert_eq!(parsed.types[2].results, Type::F64);
        }
    }

    #[test]
    fn test_type_section_all_basic_types() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();

        // Test all basic WebAssembly value types
        let test_types = [
            (Type::I32, "i32"),
            (Type::I64, "i64"),
            (Type::F32, "f32"),
            (Type::F64, "f64"),
            (Type::V128, "v128"),
        ];

        for (param_type, param_name) in &test_types {
            for (result_type, result_name) in &test_types {
                let mut module = Module::new();
                module.add_type(*param_type, *result_type);

                let mut writer = BinaryWriter::new();
                let bytes = writer.write_module(&module).unwrap_or_else(|_| {
                    panic!("Failed to write {} -> {}", param_name, result_name)
                });

                let mut reader = BinaryReader::new(&bump, bytes);
                let parsed = reader.parse_module().unwrap_or_else(|_| {
                    panic!("Failed to parse {} -> {}", param_name, result_name)
                });

                assert_eq!(
                    parsed.types.len(),
                    1,
                    "Wrong type count for {} -> {}",
                    param_name,
                    result_name
                );
                assert_eq!(
                    parsed.types[0].params, *param_type,
                    "Wrong param type for {} -> {}",
                    param_name, result_name
                );
                assert_eq!(
                    parsed.types[0].results, *result_type,
                    "Wrong result type for {} -> {}",
                    param_name, result_name
                );
            }
        }
    }

    #[test]
    fn test_type_section_empty_signatures() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();

        // Test 1: () -> ()
        {
            let mut module = Module::new();
            module.add_type(Type::NONE, Type::NONE);

            let mut writer = BinaryWriter::new();
            let bytes = writer.write_module(&module).expect("Failed to write");

            let mut reader = BinaryReader::new(&bump, bytes);
            let parsed = reader.parse_module().expect("Failed to parse");

            assert_eq!(parsed.types.len(), 1);
            assert_eq!(parsed.types[0].params, Type::NONE);
            assert_eq!(parsed.types[0].results, Type::NONE);
        }

        // Test 2: () -> (i32)
        {
            let mut module = Module::new();
            module.add_type(Type::NONE, Type::I32);

            let mut writer = BinaryWriter::new();
            let bytes = writer.write_module(&module).expect("Failed to write");

            let mut reader = BinaryReader::new(&bump, bytes);
            let parsed = reader.parse_module().expect("Failed to parse");

            assert_eq!(parsed.types.len(), 1);
            assert_eq!(parsed.types[0].params, Type::NONE);
            assert_eq!(parsed.types[0].results, Type::I32);
        }

        // Test 3: (f64) -> ()
        {
            let mut module = Module::new();
            module.add_type(Type::F64, Type::NONE);

            let mut writer = BinaryWriter::new();
            let bytes = writer.write_module(&module).expect("Failed to write");

            let mut reader = BinaryReader::new(&bump, bytes);
            let parsed = reader.parse_module().expect("Failed to parse");

            assert_eq!(parsed.types.len(), 1);
            assert_eq!(parsed.types[0].params, Type::F64);
            assert_eq!(parsed.types[0].results, Type::NONE);
        }
    }

    #[test]
    fn test_type_section_reference_types() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();

        // Test funcref and externref
        let ref_types = [(Type::FUNCREF, "funcref"), (Type::EXTERNREF, "externref")];

        for (ref_type, name) in &ref_types {
            // Test: (ref) -> ()
            {
                let mut module = Module::new();
                module.add_type(*ref_type, Type::NONE);

                let mut writer = BinaryWriter::new();
                let bytes = writer
                    .write_module(&module)
                    .unwrap_or_else(|_| panic!("Failed to write {} -> ()", name));

                let mut reader = BinaryReader::new(&bump, bytes);
                let parsed = reader
                    .parse_module()
                    .unwrap_or_else(|_| panic!("Failed to parse {} -> ()", name));

                assert_eq!(parsed.types.len(), 1);
                assert_eq!(parsed.types[0].params, *ref_type);
                assert_eq!(parsed.types[0].results, Type::NONE);
            }

            // Test: () -> (ref)
            {
                let mut module = Module::new();
                module.add_type(Type::NONE, *ref_type);

                let mut writer = BinaryWriter::new();
                let bytes = writer
                    .write_module(&module)
                    .unwrap_or_else(|_| panic!("Failed to write () -> {}", name));

                let mut reader = BinaryReader::new(&bump, bytes);
                let parsed = reader
                    .parse_module()
                    .unwrap_or_else(|_| panic!("Failed to parse () -> {}", name));

                assert_eq!(parsed.types.len(), 1);
                assert_eq!(parsed.types[0].params, Type::NONE);
                assert_eq!(parsed.types[0].results, *ref_type);
            }
        }
    }

    #[test]
    fn test_type_section_many_types() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();

        // Test with 100 types to ensure proper LEB128 encoding and no limit issues
        let mut module = Module::new();
        for i in 0..100 {
            let param_type = match i % 5 {
                0 => Type::I32,
                1 => Type::I64,
                2 => Type::F32,
                3 => Type::F64,
                _ => Type::NONE,
            };
            let result_type = match i % 4 {
                0 => Type::I32,
                1 => Type::F64,
                2 => Type::NONE,
                _ => Type::I64,
            };
            module.add_type(param_type, result_type);
        }

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        assert_eq!(parsed.types.len(), 100, "Should have 100 types");

        // Verify each type matches
        for i in 0..100 {
            let expected_param = match i % 5 {
                0 => Type::I32,
                1 => Type::I64,
                2 => Type::F32,
                3 => Type::F64,
                _ => Type::NONE,
            };
            let expected_result = match i % 4 {
                0 => Type::I32,
                1 => Type::F64,
                2 => Type::NONE,
                _ => Type::I64,
            };
            assert_eq!(
                parsed.types[i].params, expected_param,
                "Type {} param mismatch",
                i
            );
            assert_eq!(
                parsed.types[i].results, expected_result,
                "Type {} result mismatch",
                i
            );
        }
    }

    #[test]
    fn test_type_section_with_functions() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test that types are properly associated with functions
        let mut module = Module::new();

        // Add explicit types
        module.add_type(Type::I32, Type::I32); // type 0
        module.add_type(Type::F64, Type::F64); // type 1

        // Add functions that should reference these types
        let body1 = builder.const_(Literal::I32(42));
        let func1 = Function::new("f1".to_string(), Type::I32, Type::I32, vec![], Some(body1));
        module.add_function(func1);

        let body2 = builder.const_(Literal::F64(1.23));
        let func2 = Function::new("f2".to_string(), Type::F64, Type::F64, vec![], Some(body2));
        module.add_function(func2);

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        // Should have at least the 2 explicit types (writer may add more if needed)
        assert!(parsed.types.len() >= 2, "Should have at least 2 types");

        // Functions should be parsed
        assert_eq!(parsed.functions.len(), 2, "Should have 2 functions");
    }

    #[test]
    fn test_type_section_with_imports() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;
        use crate::module::{Import, ImportKind};

        let bump = Bump::new();

        let mut module = Module::new();

        // Add explicit types
        module.add_type(Type::I32, Type::I32); // type 0
        module.add_type(Type::F32, Type::F32); // type 1

        // Add imports that use these types
        module.add_import(Import {
            module: "env".to_string(),
            name: "add".to_string(),
            kind: ImportKind::Function(Type::I32, Type::I32),
        });

        module.add_import(Import {
            module: "env".to_string(),
            name: "sqrt".to_string(),
            kind: ImportKind::Function(Type::F32, Type::F32),
        });

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        // Should have the explicit types
        assert_eq!(parsed.types.len(), 2, "Should have 2 types");

        // Imports should be parsed
        assert_eq!(parsed.imports.len(), 2, "Should have 2 imports");
    }

    #[test]
    fn test_type_section_deduplication() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test that writer deduplicates identical type signatures
        let mut module = Module::new();

        // Add three functions with the same signature
        for i in 0..3 {
            let body = builder.const_(Literal::I32(i));
            let func = Function::new(format!("f{}", i), Type::I32, Type::I32, vec![], Some(body));
            module.add_function(func);
        }

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        // Should only have 1 type (deduplicated)
        assert_eq!(
            parsed.types.len(),
            1,
            "Should deduplicate identical signatures"
        );
        assert_eq!(parsed.types[0].params, Type::I32);
        assert_eq!(parsed.types[0].results, Type::I32);

        // Should still have 3 functions
        assert_eq!(parsed.functions.len(), 3);
    }

    #[test]
    fn test_function_section_explicit_type_indices() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test explicit type_idx in functions
        let mut module = Module::new();

        // Add explicit types
        module.add_type(Type::I32, Type::I32); // type 0
        module.add_type(Type::F64, Type::F64); // type 1
        module.add_type(Type::NONE, Type::I64); // type 2

        // Create functions with explicit type indices
        let body1 = builder.const_(Literal::I32(42));
        let func1 = Function::with_type_idx(
            "f0".to_string(),
            0,
            Type::I32,
            Type::I32,
            vec![],
            Some(body1),
        );
        module.add_function(func1);

        let body2 = builder.const_(Literal::F64(3.14));
        let func2 = Function::with_type_idx(
            "f1".to_string(),
            1,
            Type::F64,
            Type::F64,
            vec![],
            Some(body2),
        );
        module.add_function(func2);

        let body3 = builder.const_(Literal::I64(99));
        let func3 = Function::with_type_idx(
            "f2".to_string(),
            2,
            Type::NONE,
            Type::I64,
            vec![],
            Some(body3),
        );
        module.add_function(func3);

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        assert_eq!(parsed.types.len(), 3);
        assert_eq!(parsed.functions.len(), 3);

        // Verify type indices are preserved
        assert_eq!(parsed.functions[0].type_idx, Some(0));
        assert_eq!(parsed.functions[1].type_idx, Some(1));
        assert_eq!(parsed.functions[2].type_idx, Some(2));
    }

    #[test]
    fn test_function_section_mixed_type_specification() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test mixing explicit type_idx and inferred types
        let mut module = Module::new();

        module.add_type(Type::I32, Type::I32); // type 0

        // Function with explicit type_idx
        let body1 = builder.const_(Literal::I32(1));
        let func1 = Function::with_type_idx(
            "explicit".to_string(),
            0,
            Type::I32,
            Type::I32,
            vec![],
            Some(body1),
        );
        module.add_function(func1);

        // Function without type_idx (will be inferred)
        let body2 = builder.const_(Literal::F64(2.0));
        let func2 = Function::new(
            "inferred".to_string(),
            Type::F64,
            Type::F64,
            vec![],
            Some(body2),
        );
        module.add_function(func2);

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        assert_eq!(parsed.functions.len(), 2);
        // Both functions should have type_idx after round-trip
        assert!(parsed.functions[0].type_idx.is_some());
        assert!(parsed.functions[1].type_idx.is_some());
    }

    #[test]
    fn test_function_section_many_functions() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test with 50 functions
        let mut module = Module::new();

        for i in 0..50 {
            let (param_type, result_type) = match i % 4 {
                0 => (Type::I32, Type::I32),
                1 => (Type::F64, Type::F64),
                2 => (Type::NONE, Type::I32),
                _ => (Type::I64, Type::NONE),
            };

            let body = match result_type {
                t if t == Type::I32 => builder.const_(Literal::I32(i)),
                t if t == Type::I64 => builder.const_(Literal::I64(i as i64)),
                t if t == Type::F64 => builder.const_(Literal::F64(i as f64)),
                _ => builder.nop(),
            };

            let func = Function::new(
                format!("func_{}", i),
                param_type,
                result_type,
                vec![],
                Some(body),
            );
            module.add_function(func);
        }

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        assert_eq!(parsed.functions.len(), 50, "Should have 50 functions");
        // All functions should have type indices
        for (i, func) in parsed.functions.iter().enumerate() {
            assert!(
                func.type_idx.is_some(),
                "Function {} should have type_idx",
                i
            );
        }
    }

    #[test]
    fn test_function_section_validation_type_idx_bounds() {
        use crate::validation::Validator;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test 1: Valid type_idx
        {
            let mut module = Module::new();
            module.add_type(Type::I32, Type::I32);

            let body = builder.const_(Literal::I32(42));
            let func = Function::with_type_idx(
                "valid".to_string(),
                0,
                Type::I32,
                Type::I32,
                vec![],
                Some(body),
            );
            module.add_function(func);

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(valid, "Valid type_idx should pass: {:?}", errors);
        }

        // Test 2: Out of bounds type_idx
        {
            let mut module = Module::new();
            module.add_type(Type::I32, Type::I32);

            let body = builder.const_(Literal::I32(42));
            let func = Function::with_type_idx(
                "oob".to_string(),
                99, // Out of bounds
                Type::I32,
                Type::I32,
                vec![],
                Some(body),
            );
            module.add_function(func);

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid, "Out of bounds type_idx should fail");
            assert!(errors.iter().any(|e| e.contains("out of bounds")));
        }

        // Test 3: Type signature mismatch
        {
            let mut module = Module::new();
            module.add_type(Type::I32, Type::I32);

            let body = builder.const_(Literal::F64(3.14));
            let func = Function::with_type_idx(
                "mismatch".to_string(),
                0,
                Type::F64, // Doesn't match type 0
                Type::F64,
                vec![],
                Some(body),
            );
            module.add_function(func);

            let validator = Validator::new(&module);
            let (valid, errors) = validator.validate();
            assert!(!valid, "Signature mismatch should fail");
            assert!(errors.iter().any(|e| e.contains("Signature mismatch")));
        }
    }

    #[test]
    fn test_function_section_no_types() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test that functions work even without explicit types in module
        let mut module = Module::new();

        let body = builder.const_(Literal::I32(42));
        let func = Function::new("f".to_string(), Type::I32, Type::I32, vec![], Some(body));
        module.add_function(func);

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        // Writer should have inferred and created a type
        assert!(parsed.types.len() > 0, "Should have inferred types");
        assert_eq!(parsed.functions.len(), 1);
    }

    #[test]
    fn test_function_section_all_value_type_combinations() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        let value_types = [
            (Type::I32, Literal::I32(0)),
            (Type::I64, Literal::I64(0)),
            (Type::F32, Literal::F32(0.0)),
            (Type::F64, Literal::F64(0.0)),
        ];

        for (i, (param_type, _)) in value_types.iter().enumerate() {
            for (j, (result_type, lit)) in value_types.iter().enumerate() {
                let mut module = Module::new();

                let body = builder.const_(lit.clone());
                let func = Function::new(
                    format!("f_{}_{}", i, j),
                    *param_type,
                    *result_type,
                    vec![],
                    Some(body),
                );
                module.add_function(func);

                let mut writer = BinaryWriter::new();
                let bytes = writer
                    .write_module(&module)
                    .expect(&format!("Failed to write param {} result {}", i, j));

                let mut reader = BinaryReader::new(&bump, bytes);
                let parsed = reader
                    .parse_module()
                    .expect(&format!("Failed to parse param {} result {}", i, j));

                assert_eq!(parsed.functions.len(), 1);
                assert_eq!(parsed.functions[0].params, *param_type);
                assert_eq!(parsed.functions[0].results, *result_type);
            }
        }
    }

    #[test]
    fn test_function_section_empty() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();

        // Test module with types but no functions
        let mut module = Module::new();
        module.add_type(Type::I32, Type::I32);
        module.add_type(Type::F64, Type::F64);

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        assert_eq!(parsed.types.len(), 2);
        assert_eq!(parsed.functions.len(), 0);
    }

    #[test]
    fn test_function_section_with_locals() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test functions with various local variable counts
        let test_cases = [
            (vec![], "no locals"),
            (vec![Type::I32], "1 i32 local"),
            (vec![Type::I32, Type::I64], "2 locals"),
            (
                vec![Type::I32, Type::I64, Type::F32, Type::F64],
                "4 different locals",
            ),
            (vec![Type::I32; 10], "10 i32 locals"),
        ];

        for (locals, desc) in &test_cases {
            let mut module = Module::new();

            let body = builder.const_(Literal::I32(42));
            let func = Function::new(
                format!("f_{}", desc),
                Type::NONE,
                Type::I32,
                locals.clone(),
                Some(body),
            );
            module.add_function(func);

            let mut writer = BinaryWriter::new();
            let bytes = writer
                .write_module(&module)
                .expect(&format!("Failed to write: {}", desc));

            let mut reader = BinaryReader::new(&bump, bytes);
            let parsed = reader
                .parse_module()
                .expect(&format!("Failed to parse: {}", desc));

            assert_eq!(
                parsed.functions.len(),
                1,
                "Function count mismatch: {}",
                desc
            );
            assert_eq!(
                parsed.functions[0].vars.len(),
                locals.len(),
                "Local count mismatch: {}",
                desc
            );
        }
    }

    #[test]
    fn test_function_section_shared_type_indices() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test multiple functions sharing the same type
        let mut module = Module::new();
        module.add_type(Type::I32, Type::I32); // type 0

        // Create 5 functions all using type 0
        for i in 0..5 {
            let body = builder.const_(Literal::I32(i));
            let func = Function::with_type_idx(
                format!("shared_{}", i),
                0,
                Type::I32,
                Type::I32,
                vec![],
                Some(body),
            );
            module.add_function(func);
        }

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        assert_eq!(parsed.types.len(), 1, "Should have 1 shared type");
        assert_eq!(parsed.functions.len(), 5);

        // All functions should reference type 0
        for (i, func) in parsed.functions.iter().enumerate() {
            assert_eq!(
                func.type_idx,
                Some(0),
                "Function {} should reference type 0",
                i
            );
        }
    }

    #[test]
    fn test_function_section_type_ordering() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test that type indices are correctly maintained in order
        let mut module = Module::new();

        // Add types in specific order
        module.add_type(Type::I32, Type::I32); // type 0
        module.add_type(Type::I64, Type::I64); // type 1
        module.add_type(Type::F32, Type::F32); // type 2
        module.add_type(Type::F64, Type::F64); // type 3

        // Add functions referencing types in reverse order
        for i in (0..4).rev() {
            let (typ, lit) = match i {
                0 => (Type::I32, Literal::I32(0)),
                1 => (Type::I64, Literal::I64(0)),
                2 => (Type::F32, Literal::F32(0.0)),
                _ => (Type::F64, Literal::F64(0.0)),
            };

            let body = builder.const_(lit);
            let func = Function::with_type_idx(format!("f{}", i), i, typ, typ, vec![], Some(body));
            module.add_function(func);
        }

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        assert_eq!(parsed.functions.len(), 4);

        // Verify type indices are preserved (reverse order: 3, 2, 1, 0)
        assert_eq!(parsed.functions[0].type_idx, Some(3));
        assert_eq!(parsed.functions[1].type_idx, Some(2));
        assert_eq!(parsed.functions[2].type_idx, Some(1));
        assert_eq!(parsed.functions[3].type_idx, Some(0));
    }

    #[test]
    fn test_function_section_with_imports() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;
        use crate::module::{Import, ImportKind};

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test interaction between imported and defined functions
        let mut module = Module::new();

        module.add_type(Type::I32, Type::I32); // type 0
        module.add_type(Type::F64, Type::F64); // type 1

        // Add 2 imported functions
        module.add_import(Import {
            module: "env".to_string(),
            name: "imported1".to_string(),
            kind: ImportKind::Function(Type::I32, Type::I32),
        });

        module.add_import(Import {
            module: "env".to_string(),
            name: "imported2".to_string(),
            kind: ImportKind::Function(Type::F64, Type::F64),
        });

        // Add 2 defined functions
        let body1 = builder.const_(Literal::I32(1));
        let func1 = Function::with_type_idx(
            "local1".to_string(),
            0,
            Type::I32,
            Type::I32,
            vec![],
            Some(body1),
        );
        module.add_function(func1);

        let body2 = builder.const_(Literal::F64(2.0));
        let func2 = Function::with_type_idx(
            "local2".to_string(),
            1,
            Type::F64,
            Type::F64,
            vec![],
            Some(body2),
        );
        module.add_function(func2);

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        // Function space: imports come first, then defined functions
        assert_eq!(parsed.imports.len(), 2);
        assert_eq!(parsed.functions.len(), 2);
        assert_eq!(parsed.types.len(), 2);
    }

    #[test]
    fn test_function_section_no_body() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();

        // Test function with no body (None)
        let mut module = Module::new();
        module.add_type(Type::I32, Type::I32);

        let func =
            Function::with_type_idx("no_body".to_string(), 0, Type::I32, Type::I32, vec![], None);
        module.add_function(func);

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        assert_eq!(parsed.functions.len(), 1);
        // Body parsing might differ - just verify structure is preserved
    }

    #[test]
    fn test_function_section_reference_types() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test functions with reference types
        let mut module = Module::new();

        // funcref parameter
        let body1 = builder.nop();
        let func1 = Function::new(
            "takes_funcref".to_string(),
            Type::FUNCREF,
            Type::NONE,
            vec![],
            Some(body1),
        );
        module.add_function(func1);

        // externref parameter
        let body2 = builder.nop();
        let func2 = Function::new(
            "takes_externref".to_string(),
            Type::EXTERNREF,
            Type::NONE,
            vec![],
            Some(body2),
        );
        module.add_function(func2);

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        assert_eq!(parsed.functions.len(), 2);
        assert_eq!(parsed.functions[0].params, Type::FUNCREF);
        assert_eq!(parsed.functions[1].params, Type::EXTERNREF);
    }

    #[test]
    fn test_function_section_type_inference_deduplication() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test that writer correctly deduplicates inferred types
        let mut module = Module::new();

        // Add 10 functions with same signature (no explicit type_idx)
        for i in 0..10 {
            let body = builder.const_(Literal::I32(i));
            let func = Function::new(
                format!("inferred_{}", i),
                Type::I32,
                Type::I32,
                vec![],
                Some(body),
            );
            module.add_function(func);
        }

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        // Should deduplicate to 1 type
        assert_eq!(
            parsed.types.len(),
            1,
            "Should deduplicate 10 identical inferred types to 1"
        );
        assert_eq!(parsed.functions.len(), 10);

        // All functions should reference the same type
        for (i, func) in parsed.functions.iter().enumerate() {
            assert_eq!(
                func.type_idx,
                Some(0),
                "Function {} should reference type 0",
                i
            );
        }
    }

    #[test]
    fn test_function_section_validation_empty_module() {
        use crate::validation::Validator;

        // Test validation with no functions
        let module = Module::new();
        let validator = Validator::new(&module);
        let (valid, errors) = validator.validate();
        assert!(valid, "Empty module should be valid: {:?}", errors);
    }

    #[test]
    fn test_function_section_validation_no_type_idx() {
        use crate::validation::Validator;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test validation of function without explicit type_idx
        let mut module = Module::new();

        let body = builder.const_(Literal::I32(42));
        let func = Function::new("f".to_string(), Type::I32, Type::I32, vec![], Some(body));
        module.add_function(func);

        let validator = Validator::new(&module);
        let (valid, errors) = validator.validate();
        assert!(
            valid,
            "Function without type_idx should be valid: {:?}",
            errors
        );
    }

    #[test]
    fn test_function_section_mixed_signatures() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test with diverse function signatures
        let mut module = Module::new();

        let signatures = [
            (Type::NONE, Type::NONE, Literal::I32(0), "() -> ()"),
            (Type::I32, Type::I32, Literal::I32(1), "(i32) -> (i32)"),
            (Type::I64, Type::NONE, Literal::I32(2), "(i64) -> ()"),
            (Type::NONE, Type::F32, Literal::F32(3.0), "() -> (f32)"),
            (Type::F64, Type::I64, Literal::I64(4), "(f64) -> (i64)"),
        ];

        for (i, (param, result, lit, _desc)) in signatures.iter().enumerate() {
            let body = builder.const_(lit.clone());
            let func = Function::new(format!("f{}", i), *param, *result, vec![], Some(body));
            module.add_function(func);
        }

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        assert_eq!(parsed.functions.len(), 5);

        // Verify each function has correct signature
        for (i, (param, result, _, _)) in signatures.iter().enumerate() {
            assert_eq!(parsed.functions[i].params, *param);
            assert_eq!(parsed.functions[i].results, *result);
        }
    }

    #[test]
    fn test_function_section_large_type_index() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test with many types to ensure large type indices work
        let mut module = Module::new();

        // Add 200 types
        for i in 0..200 {
            let typ = match i % 4 {
                0 => Type::I32,
                1 => Type::I64,
                2 => Type::F32,
                _ => Type::F64,
            };
            module.add_type(typ, typ);
        }

        // Add function referencing type 199
        let body = builder.const_(Literal::F64(3.14));
        let func = Function::with_type_idx(
            "f199".to_string(),
            199,
            Type::F64,
            Type::F64,
            vec![],
            Some(body),
        );
        module.add_function(func);

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        assert_eq!(parsed.types.len(), 200);
        assert_eq!(parsed.functions.len(), 1);
        assert_eq!(parsed.functions[0].type_idx, Some(199));
    }

    #[test]
    fn test_function_section_locals_all_types() {
        use crate::binary_reader::BinaryReader;
        use crate::binary_writer::BinaryWriter;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);

        // Test function with locals of all basic types
        let mut module = Module::new();

        let locals = vec![Type::I32, Type::I64, Type::F32, Type::F64, Type::V128];

        let body = builder.const_(Literal::I32(0));
        let func = Function::new(
            "all_locals".to_string(),
            Type::NONE,
            Type::I32,
            locals,
            Some(body),
        );
        module.add_function(func);

        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let mut reader = BinaryReader::new(&bump, bytes);
        let parsed = reader.parse_module().expect("Failed to parse");

        assert_eq!(parsed.functions.len(), 1);
        assert_eq!(parsed.functions[0].vars.len(), 5);
        assert_eq!(parsed.functions[0].vars[0], Type::I32);
        assert_eq!(parsed.functions[0].vars[1], Type::I64);
        assert_eq!(parsed.functions[0].vars[2], Type::F32);
        assert_eq!(parsed.functions[0].vars[3], Type::F64);
        assert_eq!(parsed.functions[0].vars[4], Type::V128);
    }
}
