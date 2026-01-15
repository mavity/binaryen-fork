use crate::expression::{ExprRef, ExpressionKind};
use crate::ops::{BinaryOp, UnaryOp};
use binaryen_core::Literal;
use std::collections::HashMap;

/// Captured environment during pattern matching
/// Maps variable names (from Pattern::Var) to the matched sub-expressions
pub type Env<'a> = HashMap<&'static str, ExprRef<'a>>;

/// Pattern matching DSL for simplification rules
#[derive(Debug, Clone)]
pub enum Pattern {
    /// Match any expression (wildcard)
    Any,
    /// Match specific constant value
    Const(Literal),
    /// Match any constant
    AnyConst,
    /// Match and capture expression into a variable
    Var(&'static str),
    /// Match binary operation
    Binary {
        op: BinaryOp,
        left: Box<Pattern>,
        right: Box<Pattern>,
    },
    /// Match unary operation
    Unary {
        op: UnaryOp,
        value: Box<Pattern>,
    },
}

impl Pattern {
    /// Helper to create a binary pattern
    pub fn binary(op: BinaryOp, left: Pattern, right: Pattern) -> Self {
        Pattern::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Helper to create a unary pattern
    pub fn unary(op: UnaryOp, value: Pattern) -> Self {
        Pattern::Unary {
            op,
            value: Box::new(value),
        }
    }

    /// Try to match expression against this pattern
    pub fn matches<'a>(&self, expr: ExprRef<'a>, env: &mut Env<'a>) -> bool {
        match (self, &expr.kind) {
            (Pattern::Any, _) => true,
            
            (Pattern::Const(target), ExpressionKind::Const(actual)) => {
                target == actual
            }
            
            (Pattern::AnyConst, ExpressionKind::Const(_)) => true,
            
            (Pattern::Var(name), _) => {
                if let Some(existing) = env.get(name) {
                    // If variable already captured, must match structural equality?
                    // For now, simpler implementation: just check pointer equality or assume strict binding order
                    // In many pattern matchers, repeated vars enforce equality. 
                    // Let's implement reference equality check for now (safe/fast), 
                    // but TODO: implement deep structural equality if needed.
                    existing.as_ptr() == expr.as_ptr()
                } else {
                    env.insert(name, expr);
                    true
                }
            }
            
            (Pattern::Binary { op: p_op, left: p_left, right: p_right }, 
             ExpressionKind::Binary { op: e_op, left: e_left, right: e_right }) => {
                p_op == e_op && 
                p_left.matches(*e_left, env) && 
                p_right.matches(*e_right, env)
            }
            
            (Pattern::Unary { op: p_op, value: p_value }, 
             ExpressionKind::Unary { op: e_op, value: e_value }) => {
                p_op == e_op && 
                p_value.matches(*e_value, env)
            }
            
            _ => false,
        }
    }
}

/// A simplification rule
pub struct Rule {
    pub pattern: Pattern,
    // The replacement function takes an Env and returns a new expression
    // It must be valid for any lifetime 'a (the arena lifetime)
    pub replacement: Box<dyn for<'a> Fn(&Env<'a>) -> Option<ExprRef<'a>> + Sync + Send>,
}

/// Engine to apply simplification rules
pub struct PatternMatcher {
    rules: Vec<Rule>,
}

impl PatternMatcher {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule<F>(&mut self, pattern: Pattern, replacement: F)
    where
        F: for<'a> Fn(&Env<'a>) -> Option<ExprRef<'a>> + 'static + Sync + Send,
    {
        self.rules.push(Rule {
            pattern,
            replacement: Box::new(replacement),
        });
    }

    /// Try to simplify an expression using registered rules.
    /// Returns the new expression if a rule matched and replacement succeeded.
    pub fn simplify<'a>(&self, expr: ExprRef<'a>) -> Option<ExprRef<'a>> {
        let mut env = Env::new();
        
        for rule in &self.rules {
            env.clear();
            if rule.pattern.matches(expr, &mut env) {
                // If match succeeds, try to generate replacement
                // The replacement closure might return None if additional conditions fail
                // (simulating a "guard" or if the replacement generation fails)
                // Note: The replacement usually needs to allocate new nodes in the Bump arena.
                // However, the current signature `Fn(&Env) -> Option<ExprRef>` implies 
                // we might need access to an allocator or builder in the closure context.
                // For this implementation, we assume the closure captures what it needs 
                // or reuses nodes from Env.
                if let Some(new_expr) = (rule.replacement)(&env) {
                    return Some(new_expr);
                }
            }
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{Expression, ExpressionKind, IrBuilder};
    use binaryen_core::Type;
    use bumpalo::Bump;

    #[test]
    fn test_match_const() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        
        let c42 = builder.const_(Literal::I32(42));
        let c100 = builder.const_(Literal::I32(100));
        
        let p42 = Pattern::Const(Literal::I32(42));
        let p_any_const = Pattern::AnyConst;
        
        let mut env = Env::new();
        
        assert!(p42.matches(c42, &mut env));
        env.clear();
        assert!(!p42.matches(c100, &mut env));
        
        env.clear();
        assert!(p_any_const.matches(c42, &mut env));
        env.clear();
        assert!(p_any_const.matches(c100, &mut env));
    }

    #[test]
    fn test_match_var() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        
        let c42 = builder.const_(Literal::I32(42));
        let p_x = Pattern::Var("x");
        
        let mut env = Env::new();
        
        assert!(p_x.matches(c42, &mut env));
        assert_eq!(env.get("x").unwrap().as_ptr(), c42.as_ptr());
    }

    #[test]
    fn test_match_binary_structure() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        
        // (42 + 100)
        let left = builder.const_(Literal::I32(42));
        let right = builder.const_(Literal::I32(100));
        let add = builder.binary(BinaryOp::AddInt32, left, right, Type::I32);
        
        // Pattern: (AnyConst + AnyConst)
        let p = Pattern::binary(
            BinaryOp::AddInt32, 
            Pattern::AnyConst, 
            Pattern::AnyConst
        );
        
        let mut env = Env::new();
        assert!(p.matches(add, &mut env));
        
        // Pattern: (x + y)
        let p_vars = Pattern::binary(
            BinaryOp::AddInt32,
            Pattern::Var("x"),
            Pattern::Var("y")
        );
        
        env.clear();
        assert!(p_vars.matches(add, &mut env));
        assert_eq!(env.get("x").unwrap().as_ptr(), left.as_ptr());
        assert_eq!(env.get("y").unwrap().as_ptr(), right.as_ptr());
    }

    #[test]
    fn test_match_repeated_var() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        
        let c42 = builder.const_(Literal::I32(42));
        let c100 = builder.const_(Literal::I32(100));
        
        // (42 + 42)
        let add_same = builder.binary(BinaryOp::AddInt32, c42, c42, Type::I32);
        // (42 + 100)
        let add_diff = builder.binary(BinaryOp::AddInt32, c42, c100, Type::I32);
        
        // Pattern: x + x
        let p = Pattern::binary(
            BinaryOp::AddInt32,
            Pattern::Var("x"),
            Pattern::Var("x")
        );
        
        let mut env = Env::new();
        assert!(p.matches(add_same, &mut env));
        
        env.clear();
        assert!(!p.matches(add_diff, &mut env));
    }
    
    #[test]
    fn test_simple_replacement() {
        let bump = Bump::new();
        let builder = IrBuilder::new(&bump);
        
        // x + 0 -> x
        let x = builder.const_(Literal::I32(42));
        let zero = builder.const_(Literal::I32(0));
        let add = builder.binary(BinaryOp::AddInt32, x, zero, Type::I32);
        
        let mut matcher = PatternMatcher::new();
        matcher.add_rule(
            Pattern::binary(
                BinaryOp::AddInt32,
                Pattern::Var("x"),
                Pattern::Const(Literal::I32(0))
            ),
            |env| env.get("x").copied()
        );
        
        let simplified = matcher.simplify(add);
        assert!(simplified.is_some());
        assert_eq!(simplified.unwrap().as_ptr(), x.as_ptr());
    }
}
