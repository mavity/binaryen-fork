/// Semantic traits used to infer variable names based on usage patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TraitType {
    Index = 0,       // i32.add(v, 1), loop conditions
    Buffer = 1,      // Base of load/store
    Offset = 2,      // ptr + const or indexing
    Boolean = 3,     // eqz, select, if input
    Length = 4,      // Upper bound comparison
    Bitmask = 5,     // & or | with masks
    Handle = 6,      // WASI handles
    Accumulator = 7, // Repeated mutation
}

/// Compact representation of variable usage and hints.
/// Size: 16 bytes. Optimized for L1/L2 cache locality.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VariableStats {
    /// Semantic scores for each TraitType.
    pub trait_scores: [u8; 8],

    /// Global string table ID for the current best name hint.
    pub name_hint_id: u32,

    /// Confidence level (0-255) of the name hint.
    pub hint_confidence: u8,

    /// Reserved for alignment.
    _padding: [u8; 3],
}

impl Default for VariableStats {
    fn default() -> Self {
        Self {
            trait_scores: [0; 8],
            name_hint_id: 0,
            hint_confidence: 0,
            _padding: [0; 3],
        }
    }
}

/// A synthesized identity for a variable after all passes.
#[derive(Debug, Clone)]
pub struct SemanticID {
    pub base_hint_id: u32,
    pub primary_trait: TraitType,
    pub confidence: u8,
}

pub trait NameFormatter {
    fn format(&self, id: &SemanticID, local_idx: u32) -> String;
}

pub struct HumanFormatter;

impl NameFormatter for HumanFormatter {
    fn format(&self, id: &SemanticID, _local_idx: u32) -> String {
        let trait_name = match id.primary_trait {
            TraitType::Index => "idx",
            TraitType::Buffer => "ptr",
            TraitType::Offset => "offset",
            TraitType::Boolean => "flag",
            TraitType::Length => "len",
            TraitType::Bitmask => "mask",
            TraitType::Handle => "h",
            TraitType::Accumulator => "acc",
        };

        if id.base_hint_id != 0 {
            // TODO: Lookup name hint from string table
            trait_name.to_string()
        } else {
            trait_name.to_string()
        }
    }
}

use binaryen_ir::{
    expression::{ExprRef, ExpressionKind},
    ops::{BinaryOp, UnaryOp},
    visitor::Visitor,
    Module,
};

pub struct NameInferencePass;

impl NameInferencePass {
    pub fn new() -> Self {
        Self
    }

    pub fn run<'a>(&mut self, module: &mut Module<'a>) {
        use std::collections::HashMap;

        for func_idx in 0..module.functions.len() {
            let total_locals = {
                let func = &module.functions[func_idx];
                func.params.tuple_len() + func.vars.len()
            };

            let mut stats = vec![VariableStats::default(); total_locals];
            let mut visitor_names = Vec::new();

            if let Some(mut body) = module.functions[func_idx].body {
                let mut visitor = InferenceVisitor {
                    stats: &mut stats,
                    names: Vec::new(),
                    _phantom: std::marker::PhantomData,
                };
                visitor.visit(&mut body);
                visitor_names = visitor.names;
            }

            // --- Phase 4: Synthesis ---
            let formatter = HumanFormatter;
            let mut local_names = vec![String::new(); total_locals];
            let mut name_counts = HashMap::new();

            for (i, stat) in stats.iter().enumerate() {
                let mut best_trait = TraitType::Index;
                let mut max_score = 0;
                for (t_idx, &score) in stat.trait_scores.iter().enumerate() {
                    if score > max_score {
                        max_score = score;
                        best_trait = match t_idx {
                            0 => TraitType::Index,
                            1 => TraitType::Buffer,
                            2 => TraitType::Offset,
                            3 => TraitType::Boolean,
                            4 => TraitType::Length,
                            5 => TraitType::Bitmask,
                            6 => TraitType::Handle,
                            7 => TraitType::Accumulator,
                            _ => TraitType::Index,
                        };
                    }
                }

                let base_name = if max_score > 0 {
                    let id = SemanticID {
                        base_hint_id: stat.name_hint_id,
                        primary_trait: best_trait,
                        confidence: max_score.max(stat.hint_confidence),
                    };
                    formatter.format(&id, i as u32)
                } else {
                    "v".to_string()
                };

                local_names[i] = base_name.clone();
                *name_counts.entry(base_name).or_insert(0) += 1;
            }

            // Disambiguate names
            let mut current_counts = HashMap::new();
            for i in 0..total_locals {
                let base = local_names[i].clone();
                if *name_counts.get(&base).unwrap() > 1 {
                    let count = current_counts.entry(base.clone()).or_insert(0);
                    local_names[i] = format!("{}{}", base, count);
                    *count += 1;
                }
            }

            // Apply to function metadata
            module.functions[func_idx].local_names = local_names.clone();

            // Apply names to all usage points (ExprRef annotations)
            for (idx, expr_ref) in visitor_names {
                if (idx as usize) < local_names.len() {
                    let name = &local_names[idx as usize];
                    // Box::leak is safe enough here as these names are small and live for the module lifetime
                    let leaked_name: &'a str = Box::leak(name.clone().into_boxed_str());
                    module
                        .set_annotation(expr_ref, binaryen_ir::Annotation::LocalName(leaked_name));
                }
            }
        }
    }
}

struct InferenceVisitor<'a, 'b> {
    stats: &'a mut [VariableStats],
    names: Vec<(u32, ExprRef<'b>)>,
    _phantom: std::marker::PhantomData<&'b ()>,
}

impl<'a, 'b> Visitor<'b> for InferenceVisitor<'a, 'b> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'b>) {
        match &expr.kind {
            ExpressionKind::LocalGet { index } => {
                let idx = *index as usize;
                if idx < self.stats.len() {
                    self.names.push((*index, *expr));
                }
            }
            ExpressionKind::LocalSet { index, value }
            | ExpressionKind::LocalTee { index, value } => {
                let idx = *index as usize;
                if idx < self.stats.len() {
                    self.names.push((*index, *expr));
                    self.analyze_assignment(idx, value);
                }
            }
            ExpressionKind::Load { ptr, .. } | ExpressionKind::Store { ptr, .. } => {
                self.mark_as_trait(ptr, TraitType::Buffer, 40);
            }
            ExpressionKind::Binary {
                op, left, right, ..
            } => {
                self.analyze_binary(*op, left, right);
            }
            ExpressionKind::Unary { op, value } => {
                self.analyze_unary(*op, value);
            }
            ExpressionKind::If { condition, .. }
            | ExpressionKind::Loop {
                body: condition, ..
            }
            | ExpressionKind::Break {
                condition: Some(condition),
                ..
            } => {
                self.mark_as_trait(condition, TraitType::Boolean, 30);
            }
            _ => {}
        }
    }
}

impl<'a, 'b> InferenceVisitor<'a, 'b> {
    fn mark_as_trait(&mut self, expr: &ExprRef<'b>, trait_type: TraitType, weight: u8) {
        if let ExpressionKind::LocalGet { index } = &expr.kind {
            let idx = *index as usize;
            if idx < self.stats.len() {
                let current = self.stats[idx].trait_scores[trait_type as usize];
                self.stats[idx].trait_scores[trait_type as usize] = current.saturating_add(weight);
            }
        }
    }

    fn analyze_assignment(&mut self, target_idx: usize, value: &ExprRef<'b>) {
        // --- 1. Propagate Hints (Phase 3) ---
        if let ExpressionKind::LocalGet { index: src_idx } = &value.kind {
            let src_idx = *src_idx as usize;
            if src_idx < self.stats.len() {
                let src_conf = self.stats[src_idx].hint_confidence;
                let target_conf = self.stats[target_idx].hint_confidence;

                // Decay Factor: 0.85 (scaled to 255: 255 * 0.85 = 216)
                let decayed_conf = ((src_conf as u16 * 216) >> 8) as u8;

                if decayed_conf > target_conf {
                    self.stats[target_idx].name_hint_id = self.stats[src_idx].name_hint_id;
                    self.stats[target_idx].hint_confidence = decayed_conf;
                }
            }
        }

        // --- 2. Pattern Scoring ---
        match &value.kind {
            ExpressionKind::Binary {
                op, left, right, ..
            } => {
                if let BinaryOp::AddInt32 | BinaryOp::AddInt64 = op {
                    // local = local + 1 => Index/Accumulator
                    if self.is_local_get(left, target_idx as u32)
                        || self.is_local_get(right, target_idx as u32)
                    {
                        self.stats[target_idx].trait_scores[TraitType::Index as usize] =
                            self.stats[target_idx].trait_scores[TraitType::Index as usize]
                                .saturating_add(25);
                    }
                }
            }
            _ => {}
        }
    }

    fn analyze_binary(&mut self, op: BinaryOp, left: &ExprRef<'b>, right: &ExprRef<'b>) {
        match op {
            BinaryOp::AddInt32 | BinaryOp::AddInt64 | BinaryOp::SubInt32 | BinaryOp::SubInt64 => {
                // If one side is a buffer, the result is an offset
                if self.has_high_trait(left, TraitType::Buffer, 30) {
                    self.mark_as_trait(right, TraitType::Offset, 20);
                } else if self.has_high_trait(right, TraitType::Buffer, 30) {
                    self.mark_as_trait(left, TraitType::Offset, 20);
                }
            }
            BinaryOp::AndInt32 | BinaryOp::AndInt64 | BinaryOp::OrInt32 | BinaryOp::OrInt64 => {
                self.mark_as_trait(left, TraitType::Bitmask, 15);
                self.mark_as_trait(right, TraitType::Bitmask, 15);
            }
            BinaryOp::EqInt32
            | BinaryOp::NeInt32
            | BinaryOp::LtSInt32
            | BinaryOp::LtUInt32
            | BinaryOp::LeSInt32
            | BinaryOp::LeUInt32
            | BinaryOp::GtSInt32
            | BinaryOp::GtUInt32
            | BinaryOp::GeSInt32
            | BinaryOp::GeUInt32 => {
                // Comparisons often involve indices and lengths
                self.mark_as_trait(left, TraitType::Index, 10);
                self.mark_as_trait(right, TraitType::Length, 10);
            }
            _ => {}
        }
    }

    fn analyze_unary(&mut self, op: UnaryOp, value: &ExprRef<'b>) {
        if op == UnaryOp::EqZInt32 || op == UnaryOp::EqZInt64 {
            self.mark_as_trait(value, TraitType::Boolean, 40);
        }
    }

    fn is_local_get(&self, expr: &ExprRef<'b>, index: u32) -> bool {
        if let ExpressionKind::LocalGet { index: i } = &expr.kind {
            *i == index
        } else {
            false
        }
    }

    fn has_high_trait(&self, expr: &ExprRef<'b>, trait_type: TraitType, threshold: u8) -> bool {
        if let ExpressionKind::LocalGet { index } = &expr.kind {
            let idx = *index as usize;
            if idx < self.stats.len() {
                return self.stats[idx].trait_scores[trait_type as usize] >= threshold;
            }
        }
        false
    }
}
