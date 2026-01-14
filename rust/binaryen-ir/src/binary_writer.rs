use crate::expression::{Expression, ExpressionKind};
use crate::module::{Function, Module};
use crate::ops::BinaryOp;
use binaryen_core::{Literal, Type};
use std::io::{self, Write};

pub struct BinaryWriter {
    buffer: Vec<u8>,
    label_stack: Vec<Option<String>>, // Stack of label names for depth calculation
}

#[derive(Debug)]
pub enum WriteError {
    Io(io::Error),
    UnsupportedFeature(String),
    LabelNotFound(String),
}

impl From<io::Error> for WriteError {
    fn from(e: io::Error) -> Self {
        WriteError::Io(e)
    }
}

type Result<T> = std::result::Result<T, WriteError>;

impl BinaryWriter {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            label_stack: Vec::new(),
        }
    }

    pub fn write_module(&mut self, module: &Module) -> Result<Vec<u8>> {
        // Magic number: 0x00 0x61 0x73 0x6D (\0asm)
        self.write_u32(0x6D736100)?;

        // Version: 1
        self.write_u32(1)?;

        // Collect function types
        let mut type_map: Vec<(Vec<Type>, Vec<Type>)> = Vec::new();
        let mut func_type_indices: Vec<usize> = Vec::new();

        for func in &module.functions {
            // Convert single Type to Vec<Type> for type section
            let params_vec = if func.params == Type::NONE {
                vec![]
            } else {
                vec![func.params]
            };
            let results_vec = if func.results == Type::NONE {
                vec![]
            } else {
                vec![func.results]
            };

            let sig = (params_vec, results_vec);
            let idx = if let Some(pos) = type_map.iter().position(|t| *t == sig) {
                pos
            } else {
                let idx = type_map.len();
                type_map.push(sig);
                idx
            };
            func_type_indices.push(idx);
        }

        // Write Type section
        if !type_map.is_empty() {
            self.write_type_section(&type_map)?;
        }

        // Write Function section
        if !module.functions.is_empty() {
            self.write_function_section(&func_type_indices)?;
        }

        // Write Code section
        if !module.functions.is_empty() {
            self.write_code_section(&module.functions)?;
        }

        Ok(self.buffer.clone())
    }

    fn write_type_section(&mut self, types: &[(Vec<Type>, Vec<Type>)]) -> Result<()> {
        let mut section_buf = Vec::new();

        // Count
        Self::write_leb128_u32(&mut section_buf, types.len() as u32)?;

        for (params, results) in types {
            // func type
            section_buf.push(0x60);

            // params
            Self::write_leb128_u32(&mut section_buf, params.len() as u32)?;
            for param_type in params {
                Self::write_value_type(&mut section_buf, *param_type)?;
            }

            // results
            Self::write_leb128_u32(&mut section_buf, results.len() as u32)?;
            for result_type in results {
                Self::write_value_type(&mut section_buf, *result_type)?;
            }
        }

        // Section id (1 = Type)
        self.buffer.push(0x01);
        // Section size
        Self::write_leb128_u32(&mut self.buffer, section_buf.len() as u32)?;
        // Section content
        self.buffer.extend_from_slice(&section_buf);

        Ok(())
    }

    fn write_function_section(&mut self, type_indices: &[usize]) -> Result<()> {
        let mut section_buf = Vec::new();

        // Count
        Self::write_leb128_u32(&mut section_buf, type_indices.len() as u32)?;

        for &idx in type_indices {
            Self::write_leb128_u32(&mut section_buf, idx as u32)?;
        }

        // Section id (3 = Function)
        self.buffer.push(0x03);
        // Section size
        Self::write_leb128_u32(&mut self.buffer, section_buf.len() as u32)?;
        // Section content
        self.buffer.extend_from_slice(&section_buf);

        Ok(())
    }

    fn write_code_section(&mut self, functions: &[Function]) -> Result<()> {
        let mut section_buf = Vec::new();

        // Count
        Self::write_leb128_u32(&mut section_buf, functions.len() as u32)?;

        for func in functions {
            let mut body_buf = Vec::new();

            // Locals
            Self::write_leb128_u32(&mut body_buf, func.vars.len() as u32)?;
            for var_type in &func.vars {
                Self::write_leb128_u32(&mut body_buf, 1)?; // count
                Self::write_value_type(&mut body_buf, *var_type)?;
            }

            // Expression
            if let Some(body) = &func.body {
                let mut label_stack = Vec::new();
                Self::write_expression(&mut body_buf, body, &mut label_stack)?;
            }

            // end
            body_buf.push(0x0B);

            // Body size + body
            Self::write_leb128_u32(&mut section_buf, body_buf.len() as u32)?;
            section_buf.extend_from_slice(&body_buf);
        }

        // Section id (10 = Code)
        self.buffer.push(0x0A);
        // Section size
        Self::write_leb128_u32(&mut self.buffer, section_buf.len() as u32)?;
        // Section content
        self.buffer.extend_from_slice(&section_buf);

        Ok(())
    }

    fn write_expression(
        buf: &mut Vec<u8>,
        expr: &Expression,
        label_stack: &mut Vec<Option<String>>,
    ) -> Result<()> {
        match &expr.kind {
            ExpressionKind::Const(lit) => {
                match lit {
                    Literal::I32(val) => {
                        buf.push(0x41); // i32.const
                        Self::write_leb128_i32(buf, *val)?;
                    }
                    Literal::I64(val) => {
                        buf.push(0x42); // i64.const
                        Self::write_leb128_i64(buf, *val)?;
                    }
                    Literal::F32(val) => {
                        buf.push(0x43); // f32.const
                        buf.extend_from_slice(&val.to_le_bytes());
                    }
                    Literal::F64(val) => {
                        buf.push(0x44); // f64.const
                        buf.extend_from_slice(&val.to_le_bytes());
                    }
                }
            }
            ExpressionKind::LocalGet { index } => {
                buf.push(0x20); // local.get
                Self::write_leb128_u32(buf, *index)?;
            }
            ExpressionKind::LocalSet { index, value } => {
                Self::write_expression(buf, value, label_stack)?;
                buf.push(0x21); // local.set
                Self::write_leb128_u32(buf, *index)?;
            }
            ExpressionKind::LocalTee { index, value } => {
                Self::write_expression(buf, value, label_stack)?;
                buf.push(0x22); // local.tee
                Self::write_leb128_u32(buf, *index)?;
            }
            ExpressionKind::Binary { op, left, right } => {
                Self::write_expression(buf, left, label_stack)?;
                Self::write_expression(buf, right, label_stack)?;

                let opcode = match op {
                    BinaryOp::AddInt32 => 0x6A,
                    BinaryOp::SubInt32 => 0x6B,
                    BinaryOp::MulInt32 => 0x6C,
                    BinaryOp::AddInt64 => 0x7C,
                    BinaryOp::SubInt64 => 0x7D,
                    BinaryOp::MulInt64 => 0x7E,
                    _ => {
                        return Err(WriteError::UnsupportedFeature(format!(
                            "Binary op: {:?}",
                            op
                        )))
                    }
                };
                buf.push(opcode);
            }
            ExpressionKind::Block { name, list } => {
                buf.push(0x02); // block opcode
                buf.push(0x40); // block type: empty (void)

                // Push label onto stack
                label_stack.push(name.map(|s| s.to_string()));

                // Write block body
                for child in list.iter() {
                    Self::write_expression(buf, child, label_stack)?;
                }

                // Pop label
                label_stack.pop();

                buf.push(0x0B); // end opcode
            }
            ExpressionKind::Loop { name, body } => {
                buf.push(0x03); // loop opcode
                buf.push(0x40); // block type: empty (void)

                // Push label onto stack
                label_stack.push(name.map(|s| s.to_string()));

                // Write loop body
                Self::write_expression(buf, body, label_stack)?;

                // Pop label
                label_stack.pop();

                buf.push(0x0B); // end opcode
            }
            ExpressionKind::If {
                condition,
                if_true,
                if_false,
            } => {
                // Write condition
                Self::write_expression(buf, condition, label_stack)?;

                buf.push(0x04); // if opcode
                buf.push(0x40); // block type: empty (void)

                // Push unnamed label for if block
                label_stack.push(None);

                // Write then branch
                Self::write_expression(buf, if_true, label_stack)?;

                // Write else branch if present
                if let Some(if_false_expr) = if_false {
                    buf.push(0x05); // else opcode
                    Self::write_expression(buf, if_false_expr, label_stack)?;
                }

                // Pop label
                label_stack.pop();

                buf.push(0x0B); // end opcode
            }
            ExpressionKind::Break {
                name,
                condition,
                value,
            } => {
                // Write value if present
                if let Some(val) = value {
                    Self::write_expression(buf, val, label_stack)?;
                }

                // Find label depth
                let depth = Self::find_label_depth(label_stack, name)?;

                if let Some(cond) = condition {
                    // br_if
                    Self::write_expression(buf, cond, label_stack)?;
                    buf.push(0x0D); // br_if opcode
                } else {
                    // br
                    buf.push(0x0C); // br opcode
                }

                Self::write_leb128_u32(buf, depth)?;
            }
            ExpressionKind::Return { value } => {
                if let Some(val) = value {
                    Self::write_expression(buf, val, label_stack)?;
                }
                buf.push(0x0F); // return opcode
            }
            ExpressionKind::Unreachable => {
                buf.push(0x00); // unreachable opcode
            }
            ExpressionKind::Drop { value } => {
                Self::write_expression(buf, value, label_stack)?;
                buf.push(0x1A); // drop opcode
            }
            ExpressionKind::Select {
                condition,
                if_true,
                if_false,
            } => {
                Self::write_expression(buf, if_true, label_stack)?;
                Self::write_expression(buf, if_false, label_stack)?;
                Self::write_expression(buf, condition, label_stack)?;
                buf.push(0x1B); // select opcode
            }
            ExpressionKind::Nop => {
                buf.push(0x01); // nop
            }
            _ => {
                return Err(WriteError::UnsupportedFeature(format!(
                    "Expression: {:?}",
                    expr.kind
                )));
            }
        }
        Ok(())
    }

    fn find_label_depth(label_stack: &[Option<String>], target: &str) -> Result<u32> {
        // Search from the end (most recent) to the beginning
        for (i, label) in label_stack.iter().rev().enumerate() {
            if let Some(name) = label {
                if name == target {
                    return Ok(i as u32);
                }
            }
        }
        Err(WriteError::LabelNotFound(target.to_string()))
    }

    fn write_value_type(buf: &mut Vec<u8>, type_: Type) -> Result<()> {
        let byte = if type_ == Type::I32 {
            0x7F
        } else if type_ == Type::I64 {
            0x7E
        } else if type_ == Type::F32 {
            0x7D
        } else if type_ == Type::F64 {
            0x7C
        } else if type_ == Type::V128 {
            0x7B
        } else {
            return Err(WriteError::UnsupportedFeature(format!("Type: {:?}", type_)));
        };
        buf.push(byte);
        Ok(())
    }

    fn write_u32(&mut self, value: u32) -> Result<()> {
        self.buffer.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn write_leb128_u32(buf: &mut Vec<u8>, mut value: u32) -> Result<()> {
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            buf.push(byte);
            if value == 0 {
                break;
            }
        }
        Ok(())
    }

    fn write_leb128_i32(buf: &mut Vec<u8>, mut value: i32) -> Result<()> {
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            let sign_bit_set = (byte & 0x40) != 0;

            if (value == 0 && !sign_bit_set) || (value == -1 && sign_bit_set) {
                buf.push(byte);
                break;
            } else {
                byte |= 0x80;
                buf.push(byte);
            }
        }
        Ok(())
    }

    fn write_leb128_i64(buf: &mut Vec<u8>, mut value: i64) -> Result<()> {
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            let sign_bit_set = (byte & 0x40) != 0;

            if (value == 0 && !sign_bit_set) || (value == -1 && sign_bit_set) {
                buf.push(byte);
                break;
            } else {
                byte |= 0x80;
                buf.push(byte);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_reader::BinaryReader;
    use bumpalo::Bump;

    #[test]
    fn test_write_minimal_module() {
        let mut module = Module::new();

        let bump = Bump::new();
        let body = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(42)),
            type_: Type::I32,
        });

        module.add_function(Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(body),
        ));

        let mut writer = BinaryWriter::new();
        let bytes = writer
            .write_module(&module)
            .expect("Failed to write module");

        // Verify magic and version
        assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]);
        assert_eq!(&bytes[4..8], &[0x01, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_roundtrip() {
        let bump = Bump::new();

        // Original module
        let mut module = Module::new();
        let body = bump.alloc(Expression {
            kind: ExpressionKind::Const(Literal::I32(42)),
            type_: Type::I32,
        });
        module.add_function(Function::new(
            "test".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(body),
        ));

        // Write
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        // Read back
        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read");

        // Verify
        assert_eq!(module2.functions.len(), 1);
        assert_eq!(module2.functions[0].results, Type::I32);

        if let Some(body) = &module2.functions[0].body {
            match &body.kind {
                ExpressionKind::Const(Literal::I32(42)) => {} // Success
                _ => panic!("Expected i32.const 42"),
            }
        }
    }

    #[test]
    fn test_leb128_encode() {
        let mut buf = Vec::new();
        BinaryWriter::write_leb128_u32(&mut buf, 624485).unwrap();
        assert_eq!(buf, vec![0xE5, 0x8E, 0x26]);
    }

    #[test]
    fn test_leb128_signed() {
        let mut buf = Vec::new();
        BinaryWriter::write_leb128_i32(&mut buf, -123456).unwrap();

        // Verify by reading back
        let bump = Bump::new();
        let mut reader = BinaryReader::new(&bump, buf);
        let value = reader.read_leb128_i32().unwrap();
        assert_eq!(value, -123456);
    }

    #[test]
    fn test_write_multi_param_function() {
        let bump = Bump::new();

        // Create a module with a function that has a single parameter
        // (our IR currently stores params as single Type, not Vec<Type>)
        let mut module = Module::new();
        let body = bump.alloc(Expression {
            kind: ExpressionKind::LocalGet { index: 0 },
            type_: Type::I32,
        });

        module.add_function(Function::new(
            "test".to_string(),
            Type::I32, // Single param
            Type::I32,
            vec![],
            Some(body),
        ));

        let mut writer = BinaryWriter::new();
        let bytes = writer
            .write_module(&module)
            .expect("Failed to write module");

        // Read it back
        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to parse");

        assert_eq!(module2.functions.len(), 1);
        assert_eq!(module2.functions[0].params, Type::I32);
        assert_eq!(module2.functions[0].results, Type::I32);
    }

    #[test]
    fn test_roundtrip_with_locals() {
        let bump = Bump::new();

        // Module with function that has locals
        let mut module = Module::new();
        let body = bump.alloc(Expression {
            kind: ExpressionKind::LocalGet { index: 1 }, // Get local variable
            type_: Type::I32,
        });

        module.add_function(Function::new(
            "test".to_string(),
            Type::I32,
            Type::I32,
            vec![Type::I32, Type::I64], // Two local variables
            Some(body),
        ));

        // Write
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        // Read back
        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read");

        // Verify
        assert_eq!(module2.functions.len(), 1);
        let func = &module2.functions[0];
        assert_eq!(func.params, Type::I32);
        assert_eq!(func.results, Type::I32);
        assert_eq!(func.vars.len(), 2);
        assert_eq!(func.vars[0], Type::I32);
        assert_eq!(func.vars[1], Type::I64);
    }

    #[test]
    fn test_control_flow_roundtrip() {
        use crate::binary_reader::BinaryReader;
        use crate::expression::IrBuilder;
        use bumpalo::collections::Vec as BumpVec;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new();

        // Build a function with control flow:
        // fn test(i32) -> i32 {
        //   block {
        //     if (local.get 0) {
        //       return const 1
        //     }
        //     loop {
        //       drop (const 42)
        //       break
        //     }
        //   }
        //   select (const 10, const 20, local.get 0)
        // }

        let local_get = builder.local_get(0, Type::I32);
        let const_1 = builder.const_(Literal::I32(1));
        let return_expr = Expression::new(
            &bump,
            ExpressionKind::Return {
                value: Some(const_1),
            },
            Type::UNREACHABLE,
        );
        let if_body = builder.if_(local_get, return_expr, None, Type::NONE);

        let const_42 = builder.const_(Literal::I32(42));
        let drop_expr =
            Expression::new(&bump, ExpressionKind::Drop { value: const_42 }, Type::NONE);
        let break_expr = builder.break_("$loop", None, None, Type::UNREACHABLE);
        let mut loop_body_list = BumpVec::new_in(&bump);
        loop_body_list.push(drop_expr);
        loop_body_list.push(break_expr);
        let loop_body_block = builder.block(None, loop_body_list, Type::NONE);
        let loop_expr = builder.loop_(Some("$loop"), loop_body_block, Type::NONE);

        let mut block_list = BumpVec::new_in(&bump);
        block_list.push(if_body);
        block_list.push(loop_expr);
        let block = builder.block(None, block_list, Type::NONE);

        let const_10 = builder.const_(Literal::I32(10));
        let const_20 = builder.const_(Literal::I32(20));
        let local_get2 = builder.local_get(0, Type::I32);
        let select_expr = Expression::new(
            &bump,
            ExpressionKind::Select {
                condition: local_get2,
                if_true: const_10,
                if_false: const_20,
            },
            Type::I32,
        );

        let mut final_body_list = BumpVec::new_in(&bump);
        final_body_list.push(block);
        final_body_list.push(select_expr);
        let body = builder.block(None, final_body_list, Type::I32);

        module.add_function(Function::new(
            "test_control_flow".to_string(),
            Type::I32,
            Type::I32,
            vec![],
            Some(body),
        ));

        // Write
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        println!("Written {} bytes", bytes.len());
        println!("Bytes: {:02X?}", &bytes[0..std::cmp::min(100, bytes.len())]);

        // Read back
        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read");

        // Verify basic structure
        assert_eq!(module2.functions.len(), 1);
        let func = &module2.functions[0];
        // Note: function name not preserved without export section
        assert_eq!(func.params, Type::I32);
        assert_eq!(func.results, Type::I32);
        assert!(func.body.is_some());

        println!("Control flow round-trip test passed!");
    }
}
