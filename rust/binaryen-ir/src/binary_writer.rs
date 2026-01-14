use crate::expression::{Expression, ExpressionKind};
use crate::module::{Function, Module};
use crate::ops::BinaryOp;
use binaryen_core::{Literal, Type};
use std::io::{self, Write};

pub struct BinaryWriter {
    buffer: Vec<u8>,
}

#[derive(Debug)]
pub enum WriteError {
    Io(io::Error),
    UnsupportedFeature(String),
}

impl From<io::Error> for WriteError {
    fn from(e: io::Error) -> Self {
        WriteError::Io(e)
    }
}

type Result<T> = std::result::Result<T, WriteError>;

impl BinaryWriter {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
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
                Self::write_expression(&mut body_buf, body)?;
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

    fn write_expression(buf: &mut Vec<u8>, expr: &Expression) -> Result<()> {
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
                Self::write_expression(buf, value)?;
                buf.push(0x21); // local.set
                Self::write_leb128_u32(buf, *index)?;
            }
            ExpressionKind::LocalTee { index, value } => {
                Self::write_expression(buf, value)?;
                buf.push(0x22); // local.tee
                Self::write_leb128_u32(buf, *index)?;
            }
            ExpressionKind::Binary { op, left, right } => {
                Self::write_expression(buf, left)?;
                Self::write_expression(buf, right)?;

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
            ExpressionKind::Block { list, .. } => {
                for child in list.iter() {
                    Self::write_expression(buf, child)?;
                }
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

        // Verify by decoding
        let bump = Bump::new();
        let mut reader = BinaryReader::new(&bump, buf);
        let decoded = reader.read_leb128_i32().unwrap();
        assert_eq!(decoded, -123456);
    }
}
