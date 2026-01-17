use crate::expression::{ExprRef, Expression, ExpressionKind, IrBuilder};
use crate::module::{Export, ExportKind, Function, MemoryLimits, Module};
use crate::ops::{BinaryOp, UnaryOp};
use binaryen_core::{Literal, Type};
use bumpalo::collections::Vec as BumpVec;
use bumpalo::Bump;
use std::io;

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
    InvalidUtf8,
    InvalidExportKind,
    InvalidImportKind,
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

        let mut module = Module::new(self.bump);
        let mut type_section = Vec::new();
        let mut function_section = Vec::new();
        let mut code_section = Vec::new();

        // Parse sections
        let mut memory_section = None;
        let mut table_section = None;
        let mut global_section = Vec::new();
        let mut export_section = Vec::new();
        let mut element_section = Vec::new();
        let mut data_section = Vec::new();
        let mut start_section = None;

        while self.pos < self.data.len() {
            let section_id = self.read_u8()?;
            let section_size = self.read_leb128_u32()? as usize;
            let section_end = self.pos + section_size;

            match section_id {
                1 => {
                    // Type section
                    type_section = self.parse_type_section()?;
                }
                2 => {
                    // Import section
                    let imports = self.parse_import_section(&type_section)?;
                    for import in imports {
                        module.add_import(import);
                    }
                }
                3 => {
                    // Function section
                    function_section = self.parse_function_section()?;
                }
                4 => {
                    // Table section
                    table_section = self.parse_table_section()?;
                }
                5 => {
                    // Memory section
                    memory_section = self.parse_memory_section()?;
                }
                6 => {
                    // Global section
                    global_section = self.parse_global_section()?;
                }
                7 => {
                    // Export section
                    export_section = self.parse_export_section()?;
                }
                8 => {
                    // Start section
                    start_section = self.parse_start_section()?;
                }
                9 => {
                    // Element section
                    element_section = self.parse_element_section()?;
                }
                10 => {
                    // Code section
                    code_section = self.parse_code_section()?;
                }
                11 => {
                    // Data section
                    data_section = self.parse_data_section()?;
                }
                _ => {
                    // Skip unknown sections
                    self.pos = section_end;
                }
            }

            self.pos = section_end;
        }

        // Add types to module
        for (params, results) in &type_section {
            let params_type = Self::types_to_type(params);
            let results_type = Self::types_to_type(results);
            module.add_type(params_type, results_type);
        }

        // Set memory limits
        if let Some(limits) = memory_section {
            module.set_memory(limits.initial, limits.maximum);
        }

        // Set table limits
        if let Some(table) = table_section {
            module.set_table(table.element_type, table.initial, table.maximum);
        }

        // Add globals
        for global in global_section {
            module.add_global(global);
        }

        // Add exports
        for export in export_section {
            module.add_export(export.name, export.kind, export.index);
        }

        // Add data segments
        for segment in data_section {
            module.add_data_segment(segment);
        }

        // Add element segments
        for segment in element_section {
            module.add_element_segment(segment);
        }

        // Set start function
        if let Some(start_idx) = start_section {
            module.set_start(start_idx);
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
            let params = Self::types_to_type(&func_type.0);
            let results = Self::types_to_type(&func_type.1);

            let func = Function::with_type_idx(
                format!("func_{}", i),
                type_idx,
                params,
                results,
                locals,
                body,
            );
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

    /// Convert a Vec<Type> to a single Type using type interning for multi-value types
    fn types_to_type(types: &[Type]) -> Type {
        match types.len() {
            0 => Type::NONE,
            1 => types[0],
            _ => {
                // For multiple types, intern them as a signature
                // This is a simplification - ideally we'd use tuple types
                // For now, just take the first type
                types[0]
            }
        }
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

    fn parse_global_section(&mut self) -> Result<Vec<crate::module::Global<'a>>> {
        let count = self.read_leb128_u32()?;
        let mut globals = Vec::new();

        for i in 0..count {
            let type_ = self.read_value_type()?;
            let mutability = self.read_u8()?;
            let mutable = mutability == 0x01;

            // Init expression
            let mut label_stack = Vec::new();
            let init_expr = self
                .parse_expression_impl(&mut label_stack)?
                .ok_or(ParseError::UnexpectedEof)?;

            let name = format!("global_{}", i);

            globals.push(crate::module::Global {
                name,
                type_,
                mutable,
                init: init_expr,
            });
        }

        Ok(globals)
    }

    fn parse_memory_section(&mut self) -> Result<Option<MemoryLimits>> {
        let count = self.read_leb128_u32()?;
        if count == 0 {
            return Ok(None);
        }

        // Read first memory (WASM 1.0 supports only one memory)
        let (initial, maximum) = self.read_limits()?;

        // Skip remaining memories if any
        for _ in 1..count {
            let _ = self.read_limits()?;
        }

        Ok(Some(MemoryLimits { initial, maximum }))
    }

    fn parse_export_section(&mut self) -> Result<Vec<Export>> {
        let count = self.read_leb128_u32()?;
        let mut exports = Vec::new();

        for _ in 0..count {
            let name = self.read_name()?;

            let kind_byte = self.read_u8()?;
            let kind = match kind_byte {
                0 => ExportKind::Function,
                1 => ExportKind::Table,
                2 => ExportKind::Memory,
                3 => ExportKind::Global,
                _ => return Err(ParseError::InvalidExportKind),
            };

            let index = self.read_leb128_u32()?;

            exports.push(Export { name, kind, index });
        }

        Ok(exports)
    }

    fn parse_code_section(&mut self) -> Result<Vec<(Vec<Type>, Option<ExprRef<'a>>)>> {
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

    fn parse_expression(&mut self) -> Result<Option<ExprRef<'a>>> {
        self.parse_expression_impl(&mut Vec::new())
    }

    fn parse_expression_impl(
        &mut self,
        label_stack: &mut Vec<Option<String>>,
    ) -> Result<Option<ExprRef<'a>>> {
        let builder = IrBuilder::new(self.bump);
        let mut stack: Vec<ExprRef<'a>> = Vec::new();

        loop {
            let opcode = self.read_u8()?;

            match opcode {
                0x0B => {
                    // end
                    break;
                }
                0x05 => {
                    // else - only valid inside if, so break to let parent handle it
                    break;
                }
                0x00 => {
                    // unreachable
                    let expr =
                        Expression::new(self.bump, ExpressionKind::Unreachable, Type::UNREACHABLE);
                    stack.push(expr);
                }
                0x01 => {
                    // nop
                    stack.push(builder.nop());
                }
                0x02 => {
                    // block
                    let block_type = self.read_u8()?; // Block type (0x40 = void)
                    let _result_type = if block_type == 0x40 {
                        Type::NONE
                    } else {
                        self.read_value_type_from_byte(block_type)?
                    };

                    label_stack.push(None); // Unnamed block

                    // Parse block body as a single expression (which will parse until its matching 0x0B)
                    if let Some(body) = self.parse_expression_impl(label_stack)? {
                        label_stack.pop();
                        let block_expr = builder.block(
                            None,
                            BumpVec::from_iter_in([body], self.bump),
                            Type::NONE,
                        );
                        stack.push(block_expr);
                    } else {
                        label_stack.pop();
                        let block_expr =
                            builder.block(None, BumpVec::new_in(self.bump), Type::NONE);
                        stack.push(block_expr);
                    }
                }
                0x03 => {
                    // loop
                    let block_type = self.read_u8()?;
                    let _result_type = if block_type == 0x40 {
                        Type::NONE
                    } else {
                        self.read_value_type_from_byte(block_type)?
                    };

                    label_stack.push(None);

                    if let Some(body) = self.parse_expression_impl(label_stack)? {
                        label_stack.pop();
                        let loop_expr = builder.loop_(None, body, Type::NONE);
                        stack.push(loop_expr);
                    } else {
                        label_stack.pop();
                        return Err(ParseError::UnexpectedEof);
                    }
                }
                0x04 => {
                    // if
                    let block_type = self.read_u8()?;
                    let _result_type = if block_type == 0x40 {
                        Type::NONE
                    } else {
                        self.read_value_type_from_byte(block_type)?
                    };

                    let condition = stack.pop().ok_or(ParseError::UnexpectedEof)?;

                    label_stack.push(None);
                    let if_true = self
                        .parse_expression_impl(label_stack)?
                        .ok_or(ParseError::UnexpectedEof)?;

                    // Check for else (0x05) - parse_expression_impl will have stopped at it
                    let if_false = if self.pos < self.data.len() && self.data[self.pos] == 0x05 {
                        self.read_u8()?; // Consume else
                        self.parse_expression_impl(label_stack)?
                    } else {
                        None
                    };

                    label_stack.pop();
                    let if_expr = builder.if_(condition, if_true, if_false, Type::NONE);
                    stack.push(if_expr);
                }
                0x0C => {
                    // br (unconditional break)
                    let depth = self.read_leb128_u32()?;

                    // For now, use depth as label name since IR requires a label
                    let label = self.bump.alloc_str(&format!("$depth{}", depth));

                    let value = stack.pop();
                    let break_expr = builder.break_(label, None, value, Type::UNREACHABLE);
                    stack.push(break_expr);
                }
                0x0D => {
                    // br_if (conditional break)
                    let depth = self.read_leb128_u32()?;

                    // For now, use depth as label name since IR requires a label
                    let label = self.bump.alloc_str(&format!("$depth{}", depth));

                    let condition = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let value = stack.pop();
                    let break_expr = builder.break_(label, Some(condition), value, Type::I32);
                    stack.push(break_expr);
                }
                0x0F => {
                    // return
                    let value = stack.pop();
                    let return_expr = Expression::new(
                        self.bump,
                        ExpressionKind::Return { value },
                        Type::UNREACHABLE,
                    );
                    stack.push(return_expr);
                }
                0x1A => {
                    // drop
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let drop_expr =
                        Expression::new(self.bump, ExpressionKind::Drop { value }, Type::NONE);
                    stack.push(drop_expr);
                }
                0x1B => {
                    // select
                    let condition = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let if_false = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let if_true = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let result_type = if_true.type_;
                    let select_expr = Expression::new(
                        self.bump,
                        ExpressionKind::Select {
                            condition,
                            if_true,
                            if_false,
                        },
                        result_type,
                    );
                    stack.push(select_expr);
                }
                0x10 => {
                    // call
                    let func_idx = self.read_leb128_u32()?;

                    // Generate function name in bump allocator
                    let func_name =
                        bumpalo::format!(in self.bump, "func_{}", func_idx).into_bump_str();

                    // Pop operands from stack (number depends on function signature)
                    // For now, we collect all available operands
                    // TODO: Use function signature to determine exact operand count
                    let operands = BumpVec::from_iter_in(stack.drain(..), self.bump);

                    let call_expr = Expression::new(
                        self.bump,
                        ExpressionKind::Call {
                            target: func_name,
                            operands,
                            is_return: false,
                        },
                        Type::I32, // TODO: determine actual return type from function signature
                    );
                    stack.push(call_expr);
                }
                0x12 => {
                    // return_call (tail call)
                    let func_idx = self.read_leb128_u32()?;

                    // Generate function name in bump allocator
                    let func_name =
                        bumpalo::format!(in self.bump, "func_{}", func_idx).into_bump_str();

                    // Pop operands from stack
                    let operands = BumpVec::from_iter_in(stack.drain(..), self.bump);

                    let call_expr = Expression::new(
                        self.bump,
                        ExpressionKind::Call {
                            target: func_name,
                            operands,
                            is_return: true,
                        },
                        Type::UNREACHABLE, // tail call doesn't return to caller
                    );
                    stack.push(call_expr);
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
                0x23 => {
                    // global.get
                    let idx = self.read_leb128_u32()?;
                    stack.push(builder.global_get(idx, Type::I32)); // TODO: Lookup actual global type
                }
                0x24 => {
                    // global.set
                    let idx = self.read_leb128_u32()?;
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.global_set(idx, value));
                }
                0x28 => {
                    // i32.load
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.load(4, false, offset, align, ptr, Type::I32));
                }
                0x29 => {
                    // i64.load
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.load(8, false, offset, align, ptr, Type::I64));
                }
                0x2C => {
                    // i32.load8_s
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.load(1, true, offset, align, ptr, Type::I32));
                }
                0x2D => {
                    // i32.load8_u
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.load(1, false, offset, align, ptr, Type::I32));
                }
                0x2E => {
                    // i32.load16_s
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.load(2, true, offset, align, ptr, Type::I32));
                }
                0x2F => {
                    // i32.load16_u
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.load(2, false, offset, align, ptr, Type::I32));
                }
                0x2A => {
                    // f32.load
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.load(4, false, offset, align, ptr, Type::F32));
                }
                0x2B => {
                    // f64.load
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.load(8, false, offset, align, ptr, Type::F64));
                }
                0x30 => {
                    // i64.load8_s
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.load(1, true, offset, align, ptr, Type::I64));
                }
                0x31 => {
                    // i64.load8_u
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.load(1, false, offset, align, ptr, Type::I64));
                }
                0x32 => {
                    // i64.load16_s
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.load(2, true, offset, align, ptr, Type::I64));
                }
                0x33 => {
                    // i64.load16_u
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.load(2, false, offset, align, ptr, Type::I64));
                }
                0x34 => {
                    // i64.load32_s
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.load(4, true, offset, align, ptr, Type::I64));
                }
                0x35 => {
                    // i64.load32_u
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.load(4, false, offset, align, ptr, Type::I64));
                }
                0x36 => {
                    // i32.store
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.store(4, offset, align, ptr, value));
                }
                0x37 => {
                    // i64.store
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.store(8, offset, align, ptr, value));
                }
                0x3A => {
                    // i32.store8
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.store(1, offset, align, ptr, value));
                }
                0x3B => {
                    // i32.store16
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.store(2, offset, align, ptr, value));
                }
                0x38 => {
                    // f32.store
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.store(4, offset, align, ptr, value));
                }
                0x39 => {
                    // f64.store
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.store(8, offset, align, ptr, value));
                }
                0x3C => {
                    // i64.store8
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.store(1, offset, align, ptr, value));
                }
                0x3D => {
                    // i64.store16
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.store(2, offset, align, ptr, value));
                }
                0x3E => {
                    // i64.store32
                    let align = self.read_leb128_u32()?;
                    let offset = self.read_leb128_u32()?;
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let ptr = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.store(4, offset, align, ptr, value));
                }
                0x3F => {
                    // memory.size
                    let _ = self.read_u8()?; // reserved byte (0x00)
                    stack.push(builder.memory_size());
                }
                0x40 => {
                    // memory.grow
                    let _ = self.read_u8()?; // reserved byte (0x00)
                    let delta = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.memory_grow(delta));
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
                0x6D => {
                    // i32.div_s
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::DivSInt32, left, right, Type::I32));
                }
                0x6E => {
                    // i32.div_u
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::DivUInt32, left, right, Type::I32));
                }
                0x6F => {
                    // i32.rem_s
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::RemSInt32, left, right, Type::I32));
                }
                0x70 => {
                    // i32.rem_u
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::RemUInt32, left, right, Type::I32));
                }
                0x71 => {
                    // i32.and
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::AndInt32, left, right, Type::I32));
                }
                0x72 => {
                    // i32.or
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::OrInt32, left, right, Type::I32));
                }
                0x73 => {
                    // i32.xor
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::XorInt32, left, right, Type::I32));
                }
                0x74 => {
                    // i32.shl
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::ShlInt32, left, right, Type::I32));
                }
                0x75 => {
                    // i32.shr_s
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::ShrSInt32, left, right, Type::I32));
                }
                0x76 => {
                    // i32.shr_u
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::ShrUInt32, left, right, Type::I32));
                }
                0x77 => {
                    // i32.rotl
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::RotLInt32, left, right, Type::I32));
                }
                0x78 => {
                    // i32.rotr
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::RotRInt32, left, right, Type::I32));
                }
                0x45 => {
                    // i32.eqz
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::EqZInt32, value, Type::I32));
                }
                0x46 => {
                    // i32.eq
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::EqInt32, left, right, Type::I32));
                }
                0x47 => {
                    // i32.ne
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::NeInt32, left, right, Type::I32));
                }
                0x48 => {
                    // i32.lt_s
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::LtSInt32, left, right, Type::I32));
                }
                0x49 => {
                    // i32.lt_u
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::LtUInt32, left, right, Type::I32));
                }
                0x4A => {
                    // i32.gt_s
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::GtSInt32, left, right, Type::I32));
                }
                0x4B => {
                    // i32.gt_u
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::GtUInt32, left, right, Type::I32));
                }
                0x4C => {
                    // i32.le_s
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::LeSInt32, left, right, Type::I32));
                }
                0x4D => {
                    // i32.le_u
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::LeUInt32, left, right, Type::I32));
                }
                0x4E => {
                    // i32.ge_s
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::GeSInt32, left, right, Type::I32));
                }
                0x4F => {
                    // i32.ge_u
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::GeUInt32, left, right, Type::I32));
                }
                0x67 => {
                    // i32.clz
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ClzInt32, value, Type::I32));
                }
                0x68 => {
                    // i32.ctz
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::CtzInt32, value, Type::I32));
                }
                0x69 => {
                    // i32.popcnt
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::PopcntInt32, value, Type::I32));
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
                0x7F => {
                    // i64.div_s
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::DivSInt64, left, right, Type::I64));
                }
                0x80 => {
                    // i64.div_u
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::DivUInt64, left, right, Type::I64));
                }
                0x81 => {
                    // i64.rem_s
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::RemSInt64, left, right, Type::I64));
                }
                0x82 => {
                    // i64.rem_u
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::RemUInt64, left, right, Type::I64));
                }
                0x83 => {
                    // i64.and
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::AndInt64, left, right, Type::I64));
                }
                0x84 => {
                    // i64.or
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::OrInt64, left, right, Type::I64));
                }
                0x85 => {
                    // i64.xor
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::XorInt64, left, right, Type::I64));
                }
                0x86 => {
                    // i64.shl
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::ShlInt64, left, right, Type::I64));
                }
                0x87 => {
                    // i64.shr_s
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::ShrSInt64, left, right, Type::I64));
                }
                0x88 => {
                    // i64.shr_u
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::ShrUInt64, left, right, Type::I64));
                }
                0x89 => {
                    // i64.rotl
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::RotLInt64, left, right, Type::I64));
                }
                0x8A => {
                    // i64.rotr
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::RotRInt64, left, right, Type::I64));
                }
                0x50 => {
                    // i64.eqz
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::EqZInt64, value, Type::I32));
                }
                0x51 => {
                    // i64.eq
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::EqInt64, left, right, Type::I32));
                }
                0x52 => {
                    // i64.ne
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::NeInt64, left, right, Type::I32));
                }
                0x53 => {
                    // i64.lt_s
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::LtSInt64, left, right, Type::I32));
                }
                0x54 => {
                    // i64.lt_u
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::LtUInt64, left, right, Type::I32));
                }
                0x55 => {
                    // i64.gt_s
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::GtSInt64, left, right, Type::I32));
                }
                0x56 => {
                    // i64.gt_u
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::GtUInt64, left, right, Type::I32));
                }
                0x57 => {
                    // i64.le_s
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::LeSInt64, left, right, Type::I32));
                }
                0x58 => {
                    // i64.le_u
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::LeUInt64, left, right, Type::I32));
                }
                0x59 => {
                    // i64.ge_s
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::GeSInt64, left, right, Type::I32));
                }
                0x5A => {
                    // i64.ge_u
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::GeUInt64, left, right, Type::I32));
                }
                0x79 => {
                    // i64.clz
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ClzInt64, value, Type::I64));
                }
                0x7A => {
                    // i64.ctz
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::CtzInt64, value, Type::I64));
                }
                0x7B => {
                    // i64.popcnt
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::PopcntInt64, value, Type::I64));
                }
                // f32 binary operations
                0x92 => {
                    // f32.add
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::AddFloat32, left, right, Type::F32));
                }
                0x93 => {
                    // f32.sub
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::SubFloat32, left, right, Type::F32));
                }
                0x94 => {
                    // f32.mul
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::MulFloat32, left, right, Type::F32));
                }
                0x95 => {
                    // f32.div
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::DivFloat32, left, right, Type::F32));
                }
                0x96 => {
                    // f32.min
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::MinFloat32, left, right, Type::F32));
                }
                0x97 => {
                    // f32.max
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::MaxFloat32, left, right, Type::F32));
                }
                0x98 => {
                    // f32.copysign
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::CopySignFloat32, left, right, Type::F32));
                }
                0x5B => {
                    // f32.eq
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::EqFloat32, left, right, Type::I32));
                }
                0x5C => {
                    // f32.ne
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::NeFloat32, left, right, Type::I32));
                }
                0x5D => {
                    // f32.lt
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::LtFloat32, left, right, Type::I32));
                }
                0x5E => {
                    // f32.gt
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::GtFloat32, left, right, Type::I32));
                }
                0x5F => {
                    // f32.le
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::LeFloat32, left, right, Type::I32));
                }
                0x60 => {
                    // f32.ge
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::GeFloat32, left, right, Type::I32));
                }
                // f64 binary operations
                0xA0 => {
                    // f64.add
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::AddFloat64, left, right, Type::F64));
                }
                0xA1 => {
                    // f64.sub
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::SubFloat64, left, right, Type::F64));
                }
                0xA2 => {
                    // f64.mul
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::MulFloat64, left, right, Type::F64));
                }
                0xA3 => {
                    // f64.div
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::DivFloat64, left, right, Type::F64));
                }
                0xA4 => {
                    // f64.min
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::MinFloat64, left, right, Type::F64));
                }
                0xA5 => {
                    // f64.max
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::MaxFloat64, left, right, Type::F64));
                }
                0xA6 => {
                    // f64.copysign
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::CopySignFloat64, left, right, Type::F64));
                }
                0x61 => {
                    // f64.eq
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::EqFloat64, left, right, Type::I32));
                }
                0x62 => {
                    // f64.ne
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::NeFloat64, left, right, Type::I32));
                }
                0x63 => {
                    // f64.lt
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::LtFloat64, left, right, Type::I32));
                }
                0x64 => {
                    // f64.gt
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::GtFloat64, left, right, Type::I32));
                }
                0x65 => {
                    // f64.le
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::LeFloat64, left, right, Type::I32));
                }
                0x66 => {
                    // f64.ge
                    let right = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    let left = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.binary(BinaryOp::GeFloat64, left, right, Type::I32));
                }
                // f32 unary operations
                0x8B => {
                    // f32.abs
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::AbsFloat32, value, Type::F32));
                }
                0x8C => {
                    // f32.neg
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::NegFloat32, value, Type::F32));
                }
                0x8D => {
                    // f32.ceil
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::CeilFloat32, value, Type::F32));
                }
                0x8E => {
                    // f32.floor
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::FloorFloat32, value, Type::F32));
                }
                0x8F => {
                    // f32.trunc
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::TruncFloat32, value, Type::F32));
                }
                0x90 => {
                    // f32.nearest
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::NearestFloat32, value, Type::F32));
                }
                0x91 => {
                    // f32.sqrt
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::SqrtFloat32, value, Type::F32));
                }
                // f64 unary operations
                0x99 => {
                    // f64.abs
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::AbsFloat64, value, Type::F64));
                }
                0x9A => {
                    // f64.neg
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::NegFloat64, value, Type::F64));
                }
                0x9B => {
                    // f64.ceil
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::CeilFloat64, value, Type::F64));
                }
                0x9C => {
                    // f64.floor
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::FloorFloat64, value, Type::F64));
                }
                0x9D => {
                    // f64.trunc
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::TruncFloat64, value, Type::F64));
                }
                0x9E => {
                    // f64.nearest
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::NearestFloat64, value, Type::F64));
                }
                0x9F => {
                    // f64.sqrt
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::SqrtFloat64, value, Type::F64));
                }
                // Conversion operations
                0xB2 => {
                    // i32.convert_s/f32
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ConvertSInt32ToFloat32, value, Type::F32));
                }
                0xB3 => {
                    // i32.convert_u/f32
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ConvertUInt32ToFloat32, value, Type::F32));
                }
                0xB4 => {
                    // i64.convert_s/f32
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ConvertSInt64ToFloat32, value, Type::F32));
                }
                0xB5 => {
                    // i64.convert_u/f32
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ConvertUInt64ToFloat32, value, Type::F32));
                }
                0xB7 => {
                    // i32.convert_s/f64
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ConvertSInt32ToFloat64, value, Type::F64));
                }
                0xB8 => {
                    // i32.convert_u/f64
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ConvertUInt32ToFloat64, value, Type::F64));
                }
                0xB9 => {
                    // i64.convert_s/f64
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ConvertSInt64ToFloat64, value, Type::F64));
                }
                0xBA => {
                    // i64.convert_u/f64
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ConvertUInt64ToFloat64, value, Type::F64));
                }
                0xA8 => {
                    // i32.trunc_s/f32
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::TruncSFloat32ToInt32, value, Type::I32));
                }
                0xA9 => {
                    // i32.trunc_u/f32
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::TruncUFloat32ToInt32, value, Type::I32));
                }
                0xAA => {
                    // i32.trunc_s/f64
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::TruncSFloat64ToInt32, value, Type::I32));
                }
                0xAB => {
                    // i32.trunc_u/f64
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::TruncUFloat64ToInt32, value, Type::I32));
                }
                0xAE => {
                    // i64.trunc_s/f32
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::TruncSFloat32ToInt64, value, Type::I64));
                }
                0xAF => {
                    // i64.trunc_u/f32
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::TruncUFloat32ToInt64, value, Type::I64));
                }
                0xB0 => {
                    // i64.trunc_s/f64
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::TruncSFloat64ToInt64, value, Type::I64));
                }
                0xB1 => {
                    // i64.trunc_u/f64
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::TruncUFloat64ToInt64, value, Type::I64));
                }
                0xA7 => {
                    // i32.wrap_i64
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::WrapInt64, value, Type::I32));
                }
                0xAC => {
                    // i64.extend_i32_s
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ExtendSInt32, value, Type::I64));
                }
                0xAD => {
                    // i64.extend_i32_u
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ExtendUInt32, value, Type::I64));
                }
                0xB6 => {
                    // f32.demote_f64
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::DemoteFloat64, value, Type::F32));
                }
                0xBB => {
                    // f64.promote_f32
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::PromoteFloat32, value, Type::F64));
                }
                0xBC => {
                    // i32.reinterpret_f32
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ReinterpretFloat32, value, Type::I32));
                }
                0xBD => {
                    // i64.reinterpret_f64
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ReinterpretFloat64, value, Type::I64));
                }
                0xBE => {
                    // f32.reinterpret_i32
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ReinterpretInt32, value, Type::F32));
                }
                0xBF => {
                    // f64.reinterpret_i64
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ReinterpretInt64, value, Type::F64));
                }
                0xC0 => {
                    // i32.extend8_s
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ExtendS8Int32, value, Type::I32));
                }
                0xC1 => {
                    // i32.extend16_s
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ExtendS16Int32, value, Type::I32));
                }
                0xC2 => {
                    // i64.extend8_s
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ExtendS8Int64, value, Type::I64));
                }
                0xC3 => {
                    // i64.extend16_s
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ExtendS16Int64, value, Type::I64));
                }
                0xC4 => {
                    // i64.extend32_s
                    let value = stack.pop().ok_or(ParseError::UnexpectedEof)?;
                    stack.push(builder.unary(UnaryOp::ExtendS32Int64, value, Type::I64));
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
        self.read_value_type_from_byte(byte)
    }

    fn read_value_type_from_byte(&mut self, byte: u8) -> Result<Type> {
        match byte {
            0x7F => Ok(Type::I32),
            0x7E => Ok(Type::I64),
            0x7D => Ok(Type::F32),
            0x7C => Ok(Type::F64),
            0x7B => Ok(Type::V128),
            0x70 => Ok(Type::FUNCREF),
            0x6F => Ok(Type::EXTERNREF),
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

    fn read_name(&mut self) -> Result<String> {
        let len = self.read_leb128_u32()? as usize;
        let bytes = self.read_bytes(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| ParseError::InvalidUtf8)
    }

    fn read_limits(&mut self) -> Result<(u32, Option<u32>)> {
        let flags = self.read_u8()?;
        let initial = self.read_leb128_u32()?;
        let maximum = if flags & 0x01 != 0 {
            Some(self.read_leb128_u32()?)
        } else {
            None
        };
        Ok((initial, maximum))
    }

    fn parse_import_section(
        &mut self,
        types: &[(Vec<Type>, Vec<Type>)],
    ) -> Result<Vec<crate::module::Import>> {
        let count = self.read_leb128_u32()?;
        let mut imports = Vec::new();
        for _ in 0..count {
            let module = self.read_name()?;
            let name = self.read_name()?;
            let kind_id = self.read_u8()?;

            let kind = match kind_id {
                0 => {
                    // Function
                    let type_idx = self.read_leb128_u32()? as usize;
                    let func_type = types.get(type_idx).ok_or(ParseError::InvalidType)?;
                    // Simplified: just taking first param/result as Type
                    let p = func_type.0.first().copied().unwrap_or(Type::NONE);
                    let r = func_type.1.first().copied().unwrap_or(Type::NONE);
                    crate::module::ImportKind::Function(p, r)
                }
                1 => {
                    // Table
                    let elem_type = self.read_value_type()?;
                    let (min, max) = self.read_limits()?;
                    crate::module::ImportKind::Table(elem_type, min, max)
                }
                2 => {
                    // Memory
                    let (min, max) = self.read_limits()?;
                    crate::module::ImportKind::Memory(MemoryLimits {
                        initial: min,
                        maximum: max,
                    })
                }
                3 => {
                    // Global
                    let val_type = self.read_value_type()?;
                    let mutable = self.read_u8()? != 0;
                    crate::module::ImportKind::Global(val_type, mutable)
                }
                _ => return Err(ParseError::InvalidImportKind),
            };
            imports.push(crate::module::Import { module, name, kind });
        }
        Ok(imports)
    }

    fn parse_data_section(&mut self) -> Result<Vec<crate::module::DataSegment<'a>>> {
        let count = self.read_leb128_u32()?;
        let mut segments = Vec::new();

        for _ in 0..count {
            // Memory index (u32)
            let memory_index = self.read_leb128_u32()?;

            // Offset expression
            let mut label_stack = Vec::new();
            let offset = self
                .parse_expression_impl(&mut label_stack)?
                .ok_or(ParseError::InvalidSection)?;

            // Data length and bytes
            let data_len = self.read_leb128_u32()? as usize;
            let data_bytes = self.read_bytes(data_len)?;

            segments.push(crate::module::DataSegment {
                memory_index,
                offset,
                data: data_bytes.to_vec(),
            });
        }

        Ok(segments)
    }

    fn parse_start_section(&mut self) -> Result<Option<u32>> {
        let func_index = self.read_leb128_u32()?;
        Ok(Some(func_index))
    }

    fn parse_table_section(&mut self) -> Result<Option<crate::module::TableLimits>> {
        let count = self.read_leb128_u32()?;
        if count == 0 {
            return Ok(None);
        }

        // Read first table (WASM MVP supports only one table)
        let elem_type = self.read_value_type()?;
        let (initial, maximum) = self.read_limits()?;

        // Skip remaining tables if any
        for _ in 1..count {
            let _ = self.read_value_type()?;
            let _ = self.read_limits()?;
        }

        Ok(Some(crate::module::TableLimits {
            element_type: elem_type,
            initial,
            maximum,
        }))
    }

    fn parse_element_section(&mut self) -> Result<Vec<crate::module::ElementSegment<'a>>> {
        let count = self.read_leb128_u32()?;
        let mut segments = Vec::new();

        for _ in 0..count {
            // Table index (u32)
            let table_index = self.read_leb128_u32()?;

            // Offset expression
            let mut label_stack = Vec::new();
            let offset = self
                .parse_expression_impl(&mut label_stack)?
                .ok_or(ParseError::InvalidSection)?;

            // Function indices count and vector
            let func_count = self.read_leb128_u32()?;
            let mut func_indices = Vec::new();
            for _ in 0..func_count {
                func_indices.push(self.read_leb128_u32()?);
            }

            segments.push(crate::module::ElementSegment {
                table_index,
                offset,
                func_indices,
            });
        }

        Ok(segments)
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

    #[test]
    fn test_parse_multi_param_function() {
        let bump = Bump::new();

        // WASM: (module (func (param i32 i32) (result i32) local.get 0))
        let wasm = vec![
            0x00, 0x61, 0x73, 0x6D, // magic
            0x01, 0x00, 0x00, 0x00, // version
            // Type section
            0x01, 0x07, // section 1, size 7
            0x01, // 1 type
            0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7F, // func type: (i32, i32) -> i32
            // Function section
            0x03, 0x02, // section 3, size 2
            0x01, 0x00, // 1 function, type 0
            // Code section
            0x0A, 0x06, // section 10, size 6
            0x01, // 1 code
            0x04, // body size 4
            0x00, // 0 locals
            0x20, 0x00, // local.get 0
            0x0B, // end
        ];

        let mut reader = BinaryReader::new(&bump, wasm);
        let module = reader.parse_module().expect("Failed to parse module");

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];

        // Function should have parsed first parameter (our current limitation)
        assert_eq!(func.params, Type::I32);
        assert_eq!(func.results, Type::I32);
    }

    #[test]
    fn test_parse_no_param_function() {
        let bump = Bump::new();

        // WASM: (module (func (result i32) i32.const 123))
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
            0x41, 0x7B, // i32.const 123
            0x0B, // end
        ];

        let mut reader = BinaryReader::new(&bump, wasm);
        let module = reader.parse_module().expect("Failed to parse module");

        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];
        assert_eq!(func.params, Type::NONE);
        assert_eq!(func.results, Type::I32);
    }
}
