use crate::expression::{Expression, ExpressionKind, IrBuilder};
use crate::module::{Function, Module};
use crate::ops::{BinaryOp, UnaryOp};
use binaryen_core::{Literal, Type};
use bumpalo::collections::Vec as BumpVec;
use bumpalo::Bump;
use std::io::{self, Read};

pub struct BinaryReader<'a> {
    bump: &'a Bump,
    data: Vec<u8>,
    pos: usize,
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedEof,
    InvalidMagic,
    InvalidVersion,
    InvalidSection,
    InvalidOpcode(u8),
    InvalidType,
    Io(io::Error),
}

impl From<io::Error> for ParseError {
    fn from(e: io::Error) -> Self {
        ParseError::Io(e)
    }
}

type Result<T> = std::result::Result<T, ParseError>;

impl<'a> BinaryReader<'a> {
    pub fn new(bump: &'a Bump, data: Vec<u8>) -> Self {
        Self { bump, data, pos: 0 }
    }

    pub fn read_leb128_i32(&mut self) -> Result<i32> {
        let mut result = 0i32;
        let mut shift = 0;
        let mut byte;

        loop {
            byte = self.read_u8()?;
            result |= ((byte & 0x7F) as i32) << shift;
            shift += 7;

            if byte & 0x80 == 0 {
                break;
            }

            if shift >= 32 {
                return Err(ParseError::UnexpectedEof);
            }
        }

        // Sign extend if needed
        if shift < 32 && (byte & 0x40) != 0 {
            result |= !0 << shift;
        }

        Ok(result)
    }

    pub fn parse_module(&mut self) -> Result<Module<'a>> {
        // Magic number: 0x00 0x61 0x73 0x6D (\0asm)
        let magic = self.read_u32()?;
        if magic != 0x6D736100 {
            return Err(ParseError::InvalidMagic);
        }

        // Version: 1
        let version = self.read_u32()?;
        if version != 1 {
            return Err(ParseError::InvalidVersion);
        }

        let mut module = Module::new();
        let mut type_section = Vec::new();
        let mut function_section = Vec::new();
        let mut code_section = Vec::new();

        // Parse sections
        while self.pos < self.data.len() {
            let section_id = self.read_u8()?;
            let section_size = self.read_leb128_u32()? as usize;
            let section_end = self.pos + section_size;

            match section_id {
                1 => {
                    // Type section
                    type_section = self.parse_type_section()?;
                }
                3 => {
                    // Function section
                    function_section = self.parse_function_section()?;
                }
                10 => {
                    // Code section
                    code_section = self.parse_code_section()?;
                }
                _ => {
                    // Skip unknown sections
                    self.pos = section_end;
                }
            }

            self.pos = section_end;
        }

        // Combine function signatures with code
        let mut code_iter = code_section.into_iter();

        for (i, &type_idx) in function_section.iter().enumerate() {
            let Some((locals, body)) = code_iter.next() else {
                break;
            };

            let func_type = type_section
                .get(type_idx as usize)
                .cloned()
                .unwrap_or((vec![], vec![]));

            // For now, use first param/result as Type (simplified for single params)
            let params = func_type.0.first().copied().unwrap_or(Type::NONE);
            let results = func_type.1.first().copied().unwrap_or(Type::NONE);

            let func = Function::new(format!("func_{}", i), params, results, locals, body);
            module.add_function(func);
        }

        Ok(module)
    }

    fn parse_type_section(&mut self) -> Result<Vec<(Vec<Type>, Vec<Type>)>> {
        let count = self.read_leb128_u32()?;
        let mut types = Vec::new();

        for _ in 0..count {
            let form = self.read_u8()?;
            if form != 0x60 {
                // func type
                return Err(ParseError::InvalidType);
            }

            let param_count = self.read_leb128_u32()?;
            let mut params = Vec::new();
            for _ in 0..param_count {
                params.push(self.read_value_type()?);
            }

            let result_count = self.read_leb128_u32()?;
            let mut results = Vec::new();
            for _ in 0..result_count {
                results.push(self.read_value_type()?);
            }

            types.push((params, results));
        }

        Ok(types)
    }

    fn parse_function_section(&mut self) -> Result<Vec<u32>> {
        let count = self.read_leb128_u32()?;
        let mut funcs = Vec::new();

        for _ in 0..count {
            let type_idx = self.read_leb128_u32()?;
            funcs.push(type_idx);
        }

        Ok(funcs)
    }

    fn parse_code_section(&mut self) -> Result<Vec<(Vec<Type>, Option<&'a mut Expression<'a>>)>> {
        let count = self.read_leb128_u32()?;
        let mut codes = Vec::new();

        for _ in 0..count {
            let body_size = self.read_leb128_u32()?;
            let _body_end = self.pos + body_size as usize;

            // Parse locals
            let local_count = self.read_leb128_u32()?;
            let mut locals = Vec::new();
            for _ in 0..local_count {
                let count = self.read_leb128_u32()?;
                let type_ = self.read_value_type()?;
                for _ in 0..count {
                    locals.push(type_);
                }
            }

            // Parse expression
            let body = self.parse_expression()?;

            codes.push((locals, body));
        }

        Ok(codes)
    }

    fn parse_expression(&mut self) -> Result<Option<&'a mut Expression<'a>>> {
        let builder = IrBuilder::new(self.bump);
        let mut stack: Vec<&'a mut Expression<'a>> = Vec::new();

        loop {
            let opcode = self.read_u8()?;

            match opcode {
                0x0B => {
                    // end
                    break;
                }
                0x41 => {
                    // i32.const
                    let value = self.read_leb128_i32()?;
                    stack.push(builder.const_(Literal::I32(value)));
                }
                0x42 => {
                    // i64.const
                    let value = self.read_leb128_i64()?;
                    stack.push(builder.const_(Literal::I64(value)));
                }
                0x43 => {
                    // f32.const
                    let bytes = self.read_bytes(4)?;
                    let value = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    stack.push(builder.const_(Literal::F32(value)));
                }
                0x44 => {
                    // f64.const
                    let bytes = self.read_bytes(8)?;
                    let value = f64::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]);
                    stack.push(builder.const_(Literal::F64(value)));
                }
                0x20 => {
                    // local.get
                    let idx = self.read_leb128_u32()?;
                    stack.push(builder.local_get(idx, Type::I32)); // TODO: track actual type
                }
                0x21 => {
                    // local.set
                    let idx = self.read_leb128_u32()?;
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.local_set(idx, value));
                }
                0x22 => {
                    // local.tee
                    let idx = self.read_leb128_u32()?;
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let value_type = value.type_;
                    stack.push(builder.local_tee(idx, value, value_type));
                }
                0x6A => {
                    // i32.add
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::AddInt32, left, right, Type::I32));
                }
                0x6B => {
                    // i32.sub
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::SubInt32, left, right, Type::I32));
                }
                0x6C => {
                    // i32.mul
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::MulInt32, left, right, Type::I32));
                }
                0x7C => {
                    // i64.add
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::AddInt64, left, right, Type::I64));
                }
                0x7D => {
                    // i64.sub
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::SubInt64, left, right, Type::I64));
                }
                0x7E => {
                    // i64.mul
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::MulInt64, left, right, Type::I64));
                }
                0x01 => {
                    // nop
                    stack.push(builder.nop());
                }
                _ => {
                    return Err(ParseError::InvalidOpcode(opcode));
                }
            }
        }

        Ok(stack.pop())
    }

    fn read_value_type(&mut self) -> Result<Type> {
        let byte = self.read_u8()?;
        match byte {
            0x7F => Ok(Type::I32),
            0x7E => Ok(Type::I64),
            0x7D => Ok(Type::F32),
            0x7C => Ok(Type::F64),
            0x7B => Ok(Type::V128),
            _ => Err(ParseError::InvalidType),
        }
    }

    fn read_u8(&mut self) -> Result<u8> {
        if self.pos >= self.data.len() {
            return Err(ParseError::UnexpectedEof);
        }
        let byte = self.data[self.pos];
        self.pos += 1;
        Ok(byte)
    }

    fn read_u32(&mut self) -> Result<u32> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_bytes(&mut self, count: usize) -> Result<&[u8]> {
        if self.pos + count > self.data.len() {
            return Err(ParseError::UnexpectedEof);
        }
        let bytes = &self.data[self.pos..self.pos + count];
        self.pos += count;
        Ok(bytes)
    }

    fn read_leb128_u32(&mut self) -> Result<u32> {
        let mut result = 0u32;
        let mut shift = 0;

        loop {
            let byte = self.read_u8()?;
            result |= ((byte & 0x7F) as u32) << shift;

            if byte & 0x80 == 0 {
                break;
            }

            shift += 7;
            if shift >= 32 {
                return Err(ParseError::UnexpectedEof);
            }
        }

        Ok(result)
    }

    fn read_leb128_i64(&mut self) -> Result<i64> {
        let mut result = 0i64;
        let mut shift = 0;
        let mut byte;

        loop {
            byte = self.read_u8()?;
            result |= ((byte & 0x7F) as i64) << shift;
            shift += 7;

            if byte & 0x80 == 0 {
                break;
            }

            if shift >= 64 {
                return Err(ParseError::UnexpectedEof);
            }
        }

        // Sign extend if needed
        if shift < 64 && (byte & 0x40) != 0 {
            result |= !0 << shift;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_module() {
        let bump = Bump::new();

        // Minimal valid WASM: (module (func (result i32) i32.const 42))
        let wasm = vec![
            0x00, 0x61, 0x73, 0x6D, // magic
            0x01, 0x00, 0x00, 0x00, // version
            // Type section
            0x01, 0x05, // section 1, size 5
            0x01, // 1 type
            0x60, 0x00, 0x01, 0x7F, // func type: () -> i32
            // Function section
            0x03, 0x02, // section 3, size 2
            0x01, 0x00, // 1 function, type 0
            // Code section
            0x0A, 0x06, // section 10, size 6
            0x01, // 1 code
            0x04, // body size 4
            0x00, // 0 locals
            0x41, 0x2A, // i32.const 42
            0x0B, // end
        ];

        let mut reader = BinaryReader::new(&bump, wasm);
        let module = reader.parse_module().expect("Failed to parse module");

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];
        assert_eq!(func.results, Type::I32);

        if let Some(body) = &func.body {
            match &body.kind {
                ExpressionKind::Const(Literal::I32(42)) => {} // Success
                _ => panic!("Expected i32.const 42, got {:?}", body.kind),
            }
        } else {
            panic!("Function body is None");
        }
    }

    #[test]
    fn test_leb128_decode() {
        let bump = Bump::new();
        let data = vec![0xE5, 0x8E, 0x26]; // 624485 in LEB128
        let mut reader = BinaryReader::new(&bump, data);
        let value = reader.read_leb128_u32().unwrap();
        assert_eq!(value, 624485);
    }
}
