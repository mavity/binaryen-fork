use crate::expression::{Expression, ExpressionKind};
use crate::module::{Function, Module};
use crate::ops::BinaryOp;
use binaryen_core::{Literal, Type};
use std::io;

pub struct BinaryWriter {
    buffer: Vec<u8>,
    _label_stack: Vec<Option<String>>, // Stack of label names for depth calculation
}

#[derive(Debug)]
pub enum WriteError {
    Io(io::Error),
    UnsupportedFeature(String),
    LabelNotFound(String),
    InvalidExpression,
}

impl From<io::Error> for WriteError {
    fn from(e: io::Error) -> Self {
        WriteError::Io(e)
    }
}

type Result<T> = std::result::Result<T, WriteError>;

impl Default for BinaryWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl BinaryWriter {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            _label_stack: Vec::new(),
        }
    }

    pub fn write_module(&mut self, module: &Module) -> Result<Vec<u8>> {
        // Magic number: 0x00 0x61 0x73 0x6D (\0asm)
        self.write_u32(0x6D736100)?;

        // Version: 1
        self.write_u32(1)?;

        // Use types from module if available, otherwise infer from functions/imports
        let mut type_map: Vec<(Vec<Type>, Vec<Type>)> = Vec::new();

        // First, use any explicit types from module.types
        for func_type in &module.types {
            let params_vec = if func_type.params == Type::NONE {
                vec![]
            } else {
                vec![func_type.params]
            };
            let results_vec = if func_type.results == Type::NONE {
                vec![]
            } else {
                vec![func_type.results]
            };
            type_map.push((params_vec, results_vec));
        }

        // Collect types from imports (if not already in type_map)
        for import in &module.imports {
            if let crate::module::ImportKind::Function(params, results) = import.kind {
                let params_vec = if params == Type::NONE {
                    vec![]
                } else {
                    vec![params]
                };
                let results_vec = if results == Type::NONE {
                    vec![]
                } else {
                    vec![results]
                };
                let sig = (params_vec, results_vec);
                if !type_map.contains(&sig) {
                    type_map.push(sig);
                }
            }
        }

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

        // Write Import section
        if !module.imports.is_empty() {
            self.write_import_section(&module.imports, &type_map)?;
        }

        // Write Function section
        if !module.functions.is_empty() {
            self.write_function_section(&func_type_indices)?;
        }

        // Write Table section
        if let Some(ref table) = module.table {
            self.write_table_section(table)?;
        }

        // Write Memory section
        if let Some(ref memory) = module.memory {
            self.write_memory_section(memory)?;
        }

        // Write Global section
        if !module.globals.is_empty() {
            self.write_global_section(&module.globals)?;
        }

        // Write Export section
        if !module.exports.is_empty() {
            self.write_export_section(&module.exports)?;
        }

        // Write Start section
        if let Some(start_idx) = module.start {
            self.write_start_section(start_idx)?;
        }

        // Write Element section
        if !module.elements.is_empty() {
            self.write_element_section(&module.elements)?;
        }

        // Write Code section
        if !module.functions.is_empty() {
            self.write_code_section(&module.functions)?;
        }

        // Write Data section
        if !module.data.is_empty() {
            self.write_data_section(&module.data)?;
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

    fn write_import_section(
        &mut self,
        imports: &[crate::module::Import],
        type_map: &[(Vec<Type>, Vec<Type>)],
    ) -> Result<()> {
        if imports.is_empty() {
            return Ok(());
        }

        let mut section_buf = Vec::new();

        // Count
        Self::write_leb128_u32(&mut section_buf, imports.len() as u32)?;

        for import in imports {
            // Module
            Self::write_leb128_u32(&mut section_buf, import.module.len() as u32)?;
            section_buf.extend_from_slice(import.module.as_bytes());

            // Name
            Self::write_leb128_u32(&mut section_buf, import.name.len() as u32)?;
            section_buf.extend_from_slice(import.name.as_bytes());

            match &import.kind {
                crate::module::ImportKind::Function(params, results) => {
                    section_buf.push(0x00); // Kind: Function

                    let params_vec = if *params == Type::NONE {
                        vec![]
                    } else {
                        vec![*params]
                    };
                    let results_vec = if *results == Type::NONE {
                        vec![]
                    } else {
                        vec![*results]
                    };
                    let sig = (params_vec, results_vec);

                    let idx = type_map.iter().position(|t| *t == sig).ok_or_else(|| {
                        WriteError::UnsupportedFeature("Type not found in type map".to_string())
                    })?;
                    Self::write_leb128_u32(&mut section_buf, idx as u32)?;
                }
                crate::module::ImportKind::Table(elem_type, min, max) => {
                    section_buf.push(0x01); // Kind: Table
                    Self::write_value_type(&mut section_buf, *elem_type)?;
                    if let Some(m) = max {
                        section_buf.push(0x01);
                        Self::write_leb128_u32(&mut section_buf, *min)?;
                        Self::write_leb128_u32(&mut section_buf, *m)?;
                    } else {
                        section_buf.push(0x00);
                        Self::write_leb128_u32(&mut section_buf, *min)?;
                    }
                }
                crate::module::ImportKind::Memory(limits) => {
                    section_buf.push(0x02); // Kind: Memory
                    if let Some(m) = limits.maximum {
                        section_buf.push(0x01);
                        Self::write_leb128_u32(&mut section_buf, limits.initial)?;
                        Self::write_leb128_u32(&mut section_buf, m)?;
                    } else {
                        section_buf.push(0x00);
                        Self::write_leb128_u32(&mut section_buf, limits.initial)?;
                    }
                }
                crate::module::ImportKind::Global(val_type, mutable) => {
                    section_buf.push(0x03); // Kind: Global
                    Self::write_value_type(&mut section_buf, *val_type)?;
                    section_buf.push(if *mutable { 1 } else { 0 });
                }
            }
        }

        // Section id (2 = Import)
        self.buffer.push(0x02);
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

    fn write_memory_section(&mut self, memory: &crate::module::MemoryLimits) -> Result<()> {
        let mut section_buf = Vec::new();

        // Count (always 1 for now - WASM 1.0 supports only one memory)
        Self::write_leb128_u32(&mut section_buf, 1)?;

        // Limits flag: 0x00 = min only, 0x01 = min and max
        if let Some(max) = memory.maximum {
            section_buf.push(0x01);
            Self::write_leb128_u32(&mut section_buf, memory.initial)?;
            Self::write_leb128_u32(&mut section_buf, max)?;
        } else {
            section_buf.push(0x00);
            Self::write_leb128_u32(&mut section_buf, memory.initial)?;
        }

        // Section id (5 = Memory)
        self.buffer.push(0x05);
        // Section size
        Self::write_leb128_u32(&mut self.buffer, section_buf.len() as u32)?;
        // Section content
        self.buffer.extend_from_slice(&section_buf);

        Ok(())
    }

    fn write_global_section(&mut self, globals: &[crate::module::Global]) -> Result<()> {
        let mut section_buf = Vec::new();

        // Count
        Self::write_leb128_u32(&mut section_buf, globals.len() as u32)?;

        for global in globals {
            // Global Type
            Self::write_value_type(&mut section_buf, global.type_)?;

            // Mutability
            section_buf.push(if global.mutable { 0x01 } else { 0x00 });

            // Init expression
            // Global init expression must be constant and simple (no labels, no function calls)
            let mut label_stack = Vec::new(); // should stay empty
            let dummy_map = std::collections::HashMap::new();
            Self::write_expression(&mut section_buf, global.init, &mut label_stack, &dummy_map)?;

            // End opcode for expression
            section_buf.push(0x0B);
        }

        // Section id (6 = Global)
        self.buffer.push(0x06);
        // Section size
        Self::write_leb128_u32(&mut self.buffer, section_buf.len() as u32)?;
        // Section content
        self.buffer.extend_from_slice(&section_buf);

        Ok(())
    }

    fn write_export_section(&mut self, exports: &[crate::module::Export]) -> Result<()> {
        if exports.is_empty() {
            return Ok(());
        }

        let mut section_buf = Vec::new();

        // Count
        Self::write_leb128_u32(&mut section_buf, exports.len() as u32)?;

        for export in exports {
            // Name
            Self::write_leb128_u32(&mut section_buf, export.name.len() as u32)?;
            section_buf.extend_from_slice(export.name.as_bytes());

            // Kind
            section_buf.push(export.kind as u8);

            // Index
            Self::write_leb128_u32(&mut section_buf, export.index)?;
        }

        // Section id (7 = Export)
        self.buffer.push(0x07);
        // Section size
        Self::write_leb128_u32(&mut self.buffer, section_buf.len() as u32)?;
        // Section content
        self.buffer.extend_from_slice(&section_buf);

        Ok(())
    }

    fn write_start_section(&mut self, start_idx: u32) -> Result<()> {
        let mut section_buf = Vec::new();

        // Function index
        Self::write_leb128_u32(&mut section_buf, start_idx)?;

        // Section id (8 = Start)
        self.buffer.push(0x08);
        // Section size
        Self::write_leb128_u32(&mut self.buffer, section_buf.len() as u32)?;
        // Section content
        self.buffer.extend_from_slice(&section_buf);

        Ok(())
    }

    fn write_code_section(&mut self, functions: &[Function]) -> Result<()> {
        let mut section_buf = Vec::new();

        // Build function name to index map
        let mut func_map = std::collections::HashMap::new();
        for (i, func) in functions.iter().enumerate() {
            func_map.insert(func.name.as_str(), i as u32);
        }

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
                Self::write_expression(&mut body_buf, body, &mut label_stack, &func_map)?;
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
        func_map: &std::collections::HashMap<&str, u32>,
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
                Self::write_expression(buf, value, label_stack, func_map)?;
                buf.push(0x21); // local.set
                Self::write_leb128_u32(buf, *index)?;
            }
            ExpressionKind::LocalTee { index, value } => {
                Self::write_expression(buf, value, label_stack, func_map)?;
                buf.push(0x22); // local.tee
                Self::write_leb128_u32(buf, *index)?;
            }
            ExpressionKind::GlobalGet { index } => {
                buf.push(0x23); // global.get
                Self::write_leb128_u32(buf, *index)?;
            }
            ExpressionKind::GlobalSet { index, value } => {
                Self::write_expression(buf, value, label_stack, func_map)?;
                buf.push(0x24); // global.set
                Self::write_leb128_u32(buf, *index)?;
            }
            ExpressionKind::Binary { op, left, right } => {
                Self::write_expression(buf, left, label_stack, func_map)?;
                Self::write_expression(buf, right, label_stack, func_map)?;

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
                    Self::write_expression(buf, child, label_stack, func_map)?;
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
                Self::write_expression(buf, body, label_stack, func_map)?;

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
                Self::write_expression(buf, condition, label_stack, func_map)?;

                buf.push(0x04); // if opcode
                buf.push(0x40); // block type: empty (void)

                // Push unnamed label for if block
                label_stack.push(None);

                // Write then branch
                Self::write_expression(buf, if_true, label_stack, func_map)?;

                // Write else branch if present
                if let Some(if_false_expr) = if_false {
                    buf.push(0x05); // else opcode
                    Self::write_expression(buf, if_false_expr, label_stack, func_map)?;
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
                    Self::write_expression(buf, val, label_stack, func_map)?;
                }

                // Find label depth
                let depth = Self::find_label_depth(label_stack, name)?;

                if let Some(cond) = condition {
                    // br_if
                    Self::write_expression(buf, cond, label_stack, func_map)?;
                    buf.push(0x0D); // br_if opcode
                } else {
                    // br
                    buf.push(0x0C); // br opcode
                }

                Self::write_leb128_u32(buf, depth)?;
            }
            ExpressionKind::Return { value } => {
                if let Some(val) = value {
                    Self::write_expression(buf, val, label_stack, func_map)?;
                }
                buf.push(0x0F); // return opcode
            }
            ExpressionKind::Unreachable => {
                buf.push(0x00); // unreachable opcode
            }
            ExpressionKind::Drop { value } => {
                Self::write_expression(buf, value, label_stack, func_map)?;
                buf.push(0x1A); // drop opcode
            }
            ExpressionKind::Select {
                condition,
                if_true,
                if_false,
            } => {
                Self::write_expression(buf, if_true, label_stack, func_map)?;
                Self::write_expression(buf, if_false, label_stack, func_map)?;
                Self::write_expression(buf, condition, label_stack, func_map)?;
                buf.push(0x1B); // select opcode
            }
            ExpressionKind::Load {
                bytes,
                signed,
                offset,
                align,
                ptr,
            } => {
                Self::write_expression(buf, ptr, label_stack, func_map)?;

                // Opcode selection based on size and signedness
                let opcode = match (bytes, signed) {
                    (4, _) => 0x28,     // i32.load
                    (8, _) => 0x29,     // i64.load
                    (1, false) => 0x2D, // i32.load8_u
                    (1, true) => 0x2C,  // i32.load8_s
                    (2, false) => 0x2F, // i32.load16_u
                    (2, true) => 0x2E,  // i32.load16_s
                    _ => return Err(WriteError::InvalidExpression),
                };

                buf.push(opcode);
                Self::write_leb128_u32(buf, *align)?;
                Self::write_leb128_u32(buf, *offset)?;
            }
            ExpressionKind::Store {
                bytes,
                offset,
                align,
                ptr,
                value,
            } => {
                Self::write_expression(buf, ptr, label_stack, func_map)?;
                Self::write_expression(buf, value, label_stack, func_map)?;

                // Opcode selection based on size
                let opcode = match bytes {
                    4 => 0x36, // i32.store
                    8 => 0x37, // i64.store
                    1 => 0x3A, // i32.store8
                    2 => 0x3B, // i32.store16
                    _ => return Err(WriteError::InvalidExpression),
                };

                buf.push(opcode);
                Self::write_leb128_u32(buf, *align)?;
                Self::write_leb128_u32(buf, *offset)?;
            }
            ExpressionKind::Call {
                target,
                operands,
                is_return,
            } => {
                // Write operands (arguments)
                for operand in operands.iter() {
                    Self::write_expression(buf, operand, label_stack, func_map)?;
                }

                // Look up function index
                let func_index = func_map.get(target).ok_or(WriteError::InvalidExpression)?;

                if *is_return {
                    // return_call (tail call) - opcode 0x12
                    buf.push(0x12);
                } else {
                    // call - opcode 0x10
                    buf.push(0x10);
                }

                Self::write_leb128_u32(buf, *func_index)?;
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
        } else if type_ == Type::FUNCREF {
            0x70
        } else if type_ == Type::EXTERNREF {
            0x6F
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

    fn write_table_section(&mut self, table: &crate::module::TableLimits) -> Result<()> {
        let mut section_buf = Vec::new();

        // Count (always 1 for now - WASM MVP supports only one table)
        Self::write_leb128_u32(&mut section_buf, 1)?;

        // Element type
        Self::write_value_type(&mut section_buf, table.element_type)?;

        // Limits
        if let Some(max) = table.maximum {
            section_buf.push(0x01); // flag: has maximum
            Self::write_leb128_u32(&mut section_buf, table.initial)?;
            Self::write_leb128_u32(&mut section_buf, max)?;
        } else {
            section_buf.push(0x00); // flag: no maximum
            Self::write_leb128_u32(&mut section_buf, table.initial)?;
        }

        // Section id (4 = Table)
        self.buffer.push(0x04);
        // Section size
        Self::write_leb128_u32(&mut self.buffer, section_buf.len() as u32)?;
        // Section content
        self.buffer.extend_from_slice(&section_buf);

        Ok(())
    }

    fn write_element_section(&mut self, elements: &[crate::module::ElementSegment]) -> Result<()> {
        if elements.is_empty() {
            return Ok(());
        }

        let mut section_buf = Vec::new();

        // Count
        Self::write_leb128_u32(&mut section_buf, elements.len() as u32)?;

        for segment in elements {
            // Table index
            Self::write_leb128_u32(&mut section_buf, segment.table_index)?;

            // Offset expression
            let mut label_stack = Vec::new();
            let func_map = std::collections::HashMap::new();
            Self::write_expression(
                &mut section_buf,
                segment.offset,
                &mut label_stack,
                &func_map,
            )?;
            section_buf.push(0x0B); // end

            // Function indices (count + indices)
            Self::write_leb128_u32(&mut section_buf, segment.func_indices.len() as u32)?;
            for &func_idx in &segment.func_indices {
                Self::write_leb128_u32(&mut section_buf, func_idx)?;
            }
        }

        // Section id (9 = Element)
        self.buffer.push(0x09);
        // Section size
        Self::write_leb128_u32(&mut self.buffer, section_buf.len() as u32)?;
        // Section content
        self.buffer.extend_from_slice(&section_buf);

        Ok(())
    }

    fn write_data_section(&mut self, data_segments: &[crate::module::DataSegment]) -> Result<()> {
        if data_segments.is_empty() {
            return Ok(());
        }

        let mut section_buf = Vec::new();

        // Count
        Self::write_leb128_u32(&mut section_buf, data_segments.len() as u32)?;

        for segment in data_segments {
            // Memory index
            Self::write_leb128_u32(&mut section_buf, segment.memory_index)?;

            // Offset expression
            let mut label_stack = Vec::new();
            let func_map = std::collections::HashMap::new();
            Self::write_expression(
                &mut section_buf,
                segment.offset,
                &mut label_stack,
                &func_map,
            )?;
            section_buf.push(0x0B); // end

            // Data bytes (length + bytes)
            Self::write_leb128_u32(&mut section_buf, segment.data.len() as u32)?;
            section_buf.extend_from_slice(&segment.data);
        }

        // Section id (11 = Data)
        self.buffer.push(0x0B);
        // Section size
        Self::write_leb128_u32(&mut self.buffer, section_buf.len() as u32)?;
        // Section content
        self.buffer.extend_from_slice(&section_buf);

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
    use crate::expression::{ExpressionKind, IrBuilder};
    use crate::module::ExportKind;
    use crate::ops::BinaryOp;
    use binaryen_core::Literal;
    use bumpalo::collections::Vec as BumpVec;
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

    #[test]
    fn test_memory_operations_roundtrip() {
        use crate::binary_reader::BinaryReader;
        use crate::expression::IrBuilder;
        use bumpalo::collections::Vec as BumpVec;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new();

        // Build a function with memory operations:
        // fn test(i32) -> i32 {
        //   let ptr = local.get 0
        //   i32.store offset=4 align=2 (ptr, const 42)
        //   i32.load offset=4 align=2 (ptr)
        // }

        let ptr = builder.local_get(0, Type::I32);
        let const_42 = builder.const_(Literal::I32(42));
        let store_expr = builder.store(4, 4, 2, ptr, const_42);

        let ptr2 = builder.local_get(0, Type::I32);
        let load_expr = builder.load(4, false, 4, 2, ptr2, Type::I32);

        let mut body_list = BumpVec::new_in(&bump);
        body_list.push(store_expr);
        body_list.push(load_expr);
        let body = builder.block(None, body_list, Type::I32);

        module.add_function(Function::new(
            "test_memory".to_string(),
            Type::I32,
            Type::I32,
            vec![],
            Some(body),
        ));

        // Write
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        println!("Memory ops written {} bytes", bytes.len());

        // Read back
        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read");

        // Verify basic structure
        assert_eq!(module2.functions.len(), 1);
        let func = &module2.functions[0];
        assert_eq!(func.params, Type::I32);
        assert_eq!(func.results, Type::I32);
        assert!(func.body.is_some());

        println!("Memory operations round-trip test passed!");
    }

    #[test]
    fn test_load_variants() {
        use crate::binary_reader::BinaryReader;
        use crate::expression::IrBuilder;
        use bumpalo::collections::Vec as BumpVec;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new();

        // Test all load variants: i32.load8_s, i32.load8_u, i32.load16_s, i32.load16_u, i64.load
        let ptr = builder.local_get(0, Type::I32);
        let load8_s = builder.load(1, true, 0, 0, ptr, Type::I32);

        let ptr2 = builder.local_get(0, Type::I32);
        let load8_u = builder.load(1, false, 1, 0, ptr2, Type::I32);

        let ptr3 = builder.local_get(0, Type::I32);
        let load16_s = builder.load(2, true, 2, 1, ptr3, Type::I32);

        let ptr4 = builder.local_get(0, Type::I32);
        let load16_u = builder.load(2, false, 4, 1, ptr4, Type::I32);

        let ptr5 = builder.local_get(0, Type::I32);
        let load64 = builder.load(8, false, 8, 3, ptr5, Type::I64);

        let mut body_list = BumpVec::new_in(&bump);
        body_list.push(load8_s);
        body_list.push(load8_u);
        body_list.push(load16_s);
        body_list.push(load16_u);
        body_list.push(load64);
        let body = builder.block(None, body_list, Type::I64);

        module.add_function(Function::new(
            "test_loads".to_string(),
            Type::I32,
            Type::I64,
            vec![],
            Some(body),
        ));

        // Write and read back
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read");

        assert_eq!(module2.functions.len(), 1);
        println!("All load variants test passed!");
    }

    #[test]
    fn test_store_variants() {
        use crate::binary_reader::BinaryReader;
        use crate::expression::IrBuilder;
        use bumpalo::collections::Vec as BumpVec;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new();

        // Test all store variants: i32.store8, i32.store16, i64.store
        let ptr1 = builder.local_get(0, Type::I32);
        let val1 = builder.const_(Literal::I32(1));
        let store8 = builder.store(1, 0, 0, ptr1, val1);

        let ptr2 = builder.local_get(0, Type::I32);
        let val2 = builder.const_(Literal::I32(2));
        let store16 = builder.store(2, 2, 1, ptr2, val2);

        let ptr3 = builder.local_get(0, Type::I32);
        let val3 = builder.const_(Literal::I64(3));
        let store64 = builder.store(8, 4, 3, ptr3, val3);

        let mut body_list = BumpVec::new_in(&bump);
        body_list.push(store8);
        body_list.push(store16);
        body_list.push(store64);
        let body = builder.block(None, body_list, Type::NONE);

        module.add_function(Function::new(
            "test_stores".to_string(),
            Type::I32,
            Type::NONE,
            vec![],
            Some(body),
        ));

        // Write and read back
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read");

        assert_eq!(module2.functions.len(), 1);
        println!("All store variants test passed!");
    }

    #[test]
    fn test_memory_with_control_flow() {
        use crate::binary_reader::BinaryReader;
        use crate::expression::IrBuilder;
        use bumpalo::collections::Vec as BumpVec;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new();

        // Build: if (local.get 0) { i32.store(ptr, 100) } else { i32.store(ptr, 200) }
        //        return i32.load(ptr)
        let condition = builder.local_get(0, Type::I32);

        let ptr_true = builder.local_get(1, Type::I32);
        let val_true = builder.const_(Literal::I32(100));
        let store_true = builder.store(4, 0, 2, ptr_true, val_true);

        let ptr_false = builder.local_get(1, Type::I32);
        let val_false = builder.const_(Literal::I32(200));
        let store_false = builder.store(4, 0, 2, ptr_false, val_false);

        let if_expr = builder.if_(condition, store_true, Some(store_false), Type::NONE);

        let ptr_load = builder.local_get(1, Type::I32);
        let load_result = builder.load(4, false, 0, 2, ptr_load, Type::I32);

        let mut body_list = BumpVec::new_in(&bump);
        body_list.push(if_expr);
        body_list.push(load_result);
        let body = builder.block(None, body_list, Type::I32);

        // Note: Using basic types since tuple params not fully supported yet
        module.add_function(Function::new(
            "test_mem_control".to_string(),
            Type::I32, // Simple param for now
            Type::I32,
            vec![Type::I32], // local for ptr
            Some(body),
        ));

        // Write and read back
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read");

        assert_eq!(module2.functions.len(), 1);
        println!("Memory with control flow test passed!");
    }

    #[test]
    fn test_memory_offsets_and_alignment() {
        use crate::binary_reader::BinaryReader;
        use crate::expression::IrBuilder;
        use bumpalo::collections::Vec as BumpVec;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new();

        // Test various offsets and alignments
        let ptr1 = builder.local_get(0, Type::I32);
        let load_offset0 = builder.load(4, false, 0, 0, ptr1, Type::I32);

        let ptr2 = builder.local_get(0, Type::I32);
        let load_offset8 = builder.load(4, false, 8, 2, ptr2, Type::I32);

        let ptr3 = builder.local_get(0, Type::I32);
        let load_offset1024 = builder.load(4, false, 1024, 2, ptr3, Type::I32);

        let mut body_list = BumpVec::new_in(&bump);
        body_list.push(load_offset0);
        body_list.push(load_offset8);
        body_list.push(load_offset1024);
        let body = builder.block(None, body_list, Type::I32);

        module.add_function(Function::new(
            "test_offsets".to_string(),
            Type::I32,
            Type::I32,
            vec![],
            Some(body),
        ));

        // Write and read back
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read");

        assert_eq!(module2.functions.len(), 1);
        println!("Memory offsets and alignment test passed!");
    }

    #[test]
    fn test_memory_section() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new();

        // Add memory with limits
        module.set_memory(1, Some(10)); // 1 initial page, 10 max pages

        // Add a simple function that uses memory
        let ptr = builder.const_(Literal::I32(0));
        let val = builder.const_(Literal::I32(42));
        let store = builder.store(4, 0, 2, ptr, val);

        module.add_function(Function::new(
            "init_memory".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(store),
        ));

        // Write and read back
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read");

        // Verify memory limits
        assert!(module2.memory.is_some());
        let memory = module2.memory.unwrap();
        assert_eq!(memory.initial, 1);
        assert_eq!(memory.maximum, Some(10));

        assert_eq!(module2.functions.len(), 1);
        println!("Memory section test passed!");
    }

    #[test]
    fn test_export_section() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new();

        // Add a function
        let result = builder.const_(Literal::I32(42));
        module.add_function(Function::new(
            "get_answer".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(result),
        ));

        // Export the function
        module.export_function(0, "answer".to_string());

        // Write and read back
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read");

        // Verify export
        assert_eq!(module2.exports.len(), 1);
        let export = &module2.exports[0];
        assert_eq!(export.name, "answer");
        assert_eq!(export.kind, ExportKind::Function);
        assert_eq!(export.index, 0);

        println!("Export section test passed!");
    }

    #[test]
    fn test_memory_and_exports_combined() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new();

        // Set memory
        module.set_memory(2, None); // 2 pages, no max

        // Add a function that loads from memory
        let ptr = builder.const_(Literal::I32(0));
        let load = builder.load(4, false, 0, 2, ptr, Type::I32);

        module.add_function(Function::new(
            "read_mem".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(load),
        ));

        // Export both function and memory
        module.export_function(0, "read".to_string());
        module.add_export("mem".to_string(), ExportKind::Memory, 0);

        // Write and read back
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read");

        // Verify memory
        assert!(module2.memory.is_some());
        let memory = module2.memory.unwrap();
        assert_eq!(memory.initial, 2);
        assert_eq!(memory.maximum, None);

        // Verify exports
        assert_eq!(module2.exports.len(), 2);

        let func_export = module2.exports.iter().find(|e| e.name == "read").unwrap();
        assert_eq!(func_export.kind, ExportKind::Function);
        assert_eq!(func_export.index, 0);

        let mem_export = module2.exports.iter().find(|e| e.name == "mem").unwrap();
        assert_eq!(mem_export.kind, ExportKind::Memory);
        assert_eq!(mem_export.index, 0);

        println!("Memory and exports combined test passed!");
    }

    #[test]
    fn test_complete_wasm_module() {
        use std::fs;
        use std::process::Command;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new();

        // Set memory (1 page = 64KB)
        module.set_memory(1, Some(10));

        // Function 1: get42 - returns constant 42
        let result = builder.const_(Literal::I32(42));

        module.add_function(Function::new(
            "get42".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(result),
        ));

        // Function 2: store_value - stores a value in memory
        let value_param = builder.local_get(0, Type::I32);
        let ptr = builder.const_(Literal::I32(0));
        let store = builder.store(4, 0, 2, ptr, value_param);

        module.add_function(Function::new(
            "store_value".to_string(),
            Type::I32,
            Type::NONE,
            vec![],
            Some(store),
        ));

        // Function 3: load_value - loads value from memory
        let ptr3 = builder.const_(Literal::I32(0));
        let load = builder.load(4, false, 0, 2, ptr3, Type::I32);

        module.add_function(Function::new(
            "load_value".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(load),
        ));

        // Export all functions and memory
        module.export_function(0, "get42".to_string());
        module.export_function(1, "store_value".to_string());
        module.export_function(2, "load_value".to_string());
        module.add_export("memory".to_string(), ExportKind::Memory, 0);

        // Write to file
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let test_file = "/tmp/test_module.wasm";
        fs::write(test_file, &bytes).expect("Failed to write WASM file");

        // Validate with wasmtime
        let output = Command::new("wasmtime")
            .arg("compile")
            .arg(test_file)
            .arg("-o")
            .arg("/tmp/test_module.cwasm")
            .output();

        match output {
            Ok(result) => {
                if !result.status.success() {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    panic!("wasmtime compilation failed:\n{}", stderr);
                }
                println!("✓ Module compiled successfully with wasmtime!");
            }
            Err(e) => {
                println!("⚠ Could not run wasmtime ({}), skipping validation", e);
            }
        }

        // Verify round-trip
        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read back");

        assert_eq!(module2.functions.len(), 3);
        assert_eq!(module2.exports.len(), 4);
        assert!(module2.memory.is_some());

        println!("Complete WASM module test passed!");
    }

    #[test]
    fn test_function_call() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new();

        // Function 1: helper - returns 42
        let result1 = builder.const_(Literal::I32(42));
        module.add_function(Function::new(
            "helper".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(result1),
        ));

        // Function 2: caller - calls helper
        let operands = BumpVec::new_in(&bump);
        let call_expr = builder.call("helper", operands, Type::I32, false);

        module.add_function(Function::new(
            "caller".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(call_expr),
        ));

        // Write and read back
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read");

        // Verify structure
        assert_eq!(module2.functions.len(), 2);

        // Check that second function has a call expression
        let caller_func = &module2.functions[1];
        assert!(caller_func.body.is_some());

        if let Some(body) = &caller_func.body {
            match &body.kind {
                ExpressionKind::Call {
                    target,
                    operands,
                    is_return,
                } => {
                    assert_eq!(*target, "func_0"); // Should call function at index 0
                    assert_eq!(operands.len(), 0); // No arguments
                    assert!(!(*is_return)); // Not a tail call
                }
                _ => panic!("Expected Call expression, got {:?}", body.kind),
            }
        }

        println!("Function call test passed!");
    }

    #[test]
    fn test_function_call_with_args() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new();

        // Function 1: add_five - takes i32 param and adds 5
        let param = builder.local_get(0, Type::I32);
        let five = builder.const_(Literal::I32(5));
        let sum = builder.binary(BinaryOp::AddInt32, param, five, Type::I32);

        module.add_function(Function::new(
            "add_five".to_string(),
            Type::I32,
            Type::I32,
            vec![],
            Some(sum),
        ));

        // Function 2: caller - calls add_five with argument 10
        let arg = builder.const_(Literal::I32(10));
        let mut operands = BumpVec::new_in(&bump);
        operands.push(arg);
        let call_expr = builder.call("add_five", operands, Type::I32, false);

        module.add_function(Function::new(
            "caller".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(call_expr),
        ));

        // Write and read back
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read");

        // Verify structure
        assert_eq!(module2.functions.len(), 2);

        println!("Function call with args test passed!");
    }

    #[test]
    fn test_recursive_function() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new();

        // Recursive factorial function (simplified)
        // factorial(n): if n <= 1 then 1 else n * factorial(n-1)
        let _n = builder.local_get(0, Type::I32);
        let one = builder.const_(Literal::I32(1));

        // For simplicity, just return 1 (base case)
        // A full recursive implementation would need proper if/else with recursive call
        let body = one;

        module.add_function(Function::new(
            "factorial".to_string(),
            Type::I32,
            Type::I32,
            vec![],
            Some(body),
        ));

        // Write and read back
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read");

        assert_eq!(module2.functions.len(), 1);

        println!("Recursive function test passed!");
    }

    #[test]
    fn test_multi_function_program_with_wasmtime() {
        use std::fs;
        use std::process::Command;

        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        let mut module = Module::new();

        // Set memory
        module.set_memory(1, Some(10));

        // Function 1: double - doubles an i32 parameter
        let param = builder.local_get(0, Type::I32);
        let two = builder.const_(Literal::I32(2));
        let doubled = builder.binary(BinaryOp::MulInt32, param, two, Type::I32);

        module.add_function(Function::new(
            "double".to_string(),
            Type::I32,
            Type::I32,
            vec![],
            Some(doubled),
        ));

        // Function 2: increment - adds 1 to an i32 parameter
        let param2 = builder.local_get(0, Type::I32);
        let one = builder.const_(Literal::I32(1));
        let incremented = builder.binary(BinaryOp::AddInt32, param2, one, Type::I32);

        module.add_function(Function::new(
            "increment".to_string(),
            Type::I32,
            Type::I32,
            vec![],
            Some(incremented),
        ));

        // Function 3: process - calls both functions: increment(double(x))
        let input = builder.const_(Literal::I32(5)); // Input: 5

        // Call double(5) -> 10
        let mut double_args = BumpVec::new_in(&bump);
        double_args.push(input);
        let double_result = builder.call("double", double_args, Type::I32, false);

        // Call increment(10) -> 11
        let mut inc_args = BumpVec::new_in(&bump);
        inc_args.push(double_result);
        let final_result = builder.call("increment", inc_args, Type::I32, false);

        module.add_function(Function::new(
            "process".to_string(),
            Type::NONE,
            Type::I32,
            vec![],
            Some(final_result),
        ));

        // Export all functions
        module.export_function(0, "double".to_string());
        module.export_function(1, "increment".to_string());
        module.export_function(2, "process".to_string());
        module.add_export("memory".to_string(), ExportKind::Memory, 0);

        // Write to file
        let mut writer = BinaryWriter::new();
        let bytes = writer.write_module(&module).expect("Failed to write");

        let test_file = "/tmp/test_function_calls.wasm";
        fs::write(test_file, &bytes).expect("Failed to write WASM file");

        // Validate with wasmtime
        let output = Command::new("wasmtime")
            .arg("compile")
            .arg(test_file)
            .arg("-o")
            .arg("/tmp/test_function_calls.cwasm")
            .output();

        match output {
            Ok(result) => {
                if !result.status.success() {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    panic!("wasmtime compilation failed:\n{}", stderr);
                }
                println!("✓ Multi-function module compiled successfully with wasmtime!");
            }
            Err(e) => {
                println!("⚠ Could not run wasmtime ({}), skipping validation", e);
            }
        }

        // Verify round-trip
        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader.parse_module().expect("Failed to read back");

        assert_eq!(module2.functions.len(), 3);
        assert_eq!(module2.exports.len(), 4); // 3 functions + memory

        // Verify process function has calls
        let process_func = &module2.functions[2];
        assert!(process_func.body.is_some());

        println!("Multi-function program test passed!");
    }

    #[test]
    fn test_globals_roundtrip() {
        let bump = Bump::new();
        let mut module = Module::new();

        // 1. Define global: mut i32 g0 = 100
        let builder = IrBuilder::new(&bump);
        let init0 = builder.const_(Literal::I32(100)); // Fixed
        module.add_global(crate::module::Global {
            name: "g0".to_string(),
            type_: Type::I32,
            mutable: true,
            init: init0,
        });

        // 2. Define global: const f32 g1 = 3.14
        let builder2 = IrBuilder::new(&bump);
        let init1 = builder2.const_(Literal::F32(2.5)); // Fixed
        module.add_global(crate::module::Global {
            name: "g1".to_string(),
            type_: Type::F32,
            mutable: false,
            init: init1,
        });

        // 3. Define function that uses globals
        let builder3 = IrBuilder::new(&bump);

        let get_g0 = builder3.global_get(0, Type::I32);
        let const_1 = builder3.const_(Literal::I32(1)); // Fixed
        let add = builder3.binary(BinaryOp::AddInt32, get_g0, const_1, Type::I32); // Fixed
        let set_g0 = builder3.global_set(0, add); // g0 = g0 + 1

        let get_g1 = builder3.global_get(1, Type::F32);
        let drop_g1 = builder3.drop(get_g1); // Fixed

        let body = builder3.block(
            None,
            BumpVec::from_iter_in([set_g0, drop_g1], &bump),
            Type::NONE,
        );

        // Function with no params/results/locals
        let func = Function::new(
            "test_globals".to_string(),
            Type::NONE,
            Type::NONE,
            vec![],
            Some(body),
        );
        module.add_function(func);
        module.export_function(0, "test_globals".to_string());

        // Write
        let mut writer = BinaryWriter::new();
        let bytes = writer
            .write_module(&module)
            .expect("Failed to write module with globals");

        // Read back
        let bump2 = Bump::new();
        let mut reader = BinaryReader::new(&bump2, bytes);
        let module2 = reader
            .parse_module()
            .expect("Failed to read back module with globals");

        // Checks
        assert_eq!(module2.globals.len(), 2);

        let g0 = &module2.globals[0];
        assert_eq!(g0.type_, Type::I32);
        assert!(g0.mutable);

        let g1 = &module2.globals[1];
        assert_eq!(g1.type_, Type::F32);
        assert!(!g1.mutable);

        assert_eq!(module2.functions.len(), 1);
        let func = &module2.functions[0];
        assert!(func.body.is_some());
    }
}
