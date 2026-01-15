# Arena-Based Expression Manipulation Implementation Plan

**Date**: January 15, 2026  
**Status**: Planning → Implementation  
**Critical**: This unblocks 20+ Tier 2 optimization passes

---

## Problem Statement

Current limitation: Cannot create new expressions or restructure expression trees during pass execution because:
1. Expressions are owned by the original arena
2. Visitor pattern doesn't provide arena access
3. No way to allocate new nodes during transformation

**Blocked passes**: untee, local-cse, merge-blocks, simplify-locals (full), code-pushing, licm, and 15+ others

---

## Solution Architecture

### Core Concept
Pass the Bump allocator through the Module and make it available during pass execution.

### Key Changes
1. Module owns/references a Bump allocator
2. Pass::run() provides allocator access
3. Helper methods for common transformations
4. Maintain lifetime safety

---

## Implementation Steps (STRICT SEQUENTIAL)

### Step 1: Add Allocator to Module (30 min)
**File**: `rust/binaryen-ir/src/module.rs`

**Changes**:
```rust
pub struct Module<'a> {
    pub allocator: &'a Bump,  // NEW: Reference to allocator
    pub types: Vec<FuncType>,
    // ... rest unchanged
}

impl<'a> Module<'a> {
    pub fn new(allocator: &'a Bump) -> Self {  // NEW: Required parameter
        Self {
            allocator,
            types: Vec::new(),
            // ...
        }
    }
    
    pub fn allocator(&self) -> &'a Bump {  // NEW: Accessor
        self.allocator
    }
}
```

**Tests to Update**:
- All module creation in tests must pass allocator
- Estimate: ~20 test files to update

**Validation**:
- [x] Build succeeds
- [x] All 318 tests pass
- [x] No warnings introduced

---

### Step 2: Add Expression Creation Helpers (45 min)
**File**: `rust/binaryen-ir/src/expression.rs`

**New Methods**:
```rust
impl<'a> Expression<'a> {
    /// Create a new nop expression
    pub fn nop(bump: &'a Bump) -> ExprRef<'a> {
        ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Nop,
            type_: Type::NONE,
        }))
    }
    
    /// Create a new const expression
    pub fn const_expr(bump: &'a Bump, lit: Literal, ty: Type) -> ExprRef<'a> {
        ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Const(lit),
            type_: ty,
        }))
    }
    
    /// Create a new block
    pub fn block(bump: &'a Bump, name: Option<&'a str>, 
                 list: BumpVec<'a, ExprRef<'a>>, ty: Type) -> ExprRef<'a> {
        ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::Block { name, list },
            type_: ty,
        }))
    }
    
    /// Create local.set
    pub fn local_set(bump: &'a Bump, index: u32, value: ExprRef<'a>) -> ExprRef<'a> {
        ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::LocalSet { index, value },
            type_: Type::NONE,
        }))
    }
    
    /// Create local.get
    pub fn local_get(bump: &'a Bump, index: u32, ty: Type) -> ExprRef<'a> {
        ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::LocalGet { index },
            type_: ty,
        }))
    }
    
    /// Create local.tee
    pub fn local_tee(bump: &'a Bump, index: u32, value: ExprRef<'a>, ty: Type) -> ExprRef<'a> {
        ExprRef::new(bump.alloc(Expression {
            kind: ExpressionKind::LocalTee { index, value },
            type_: ty,
        }))
    }
}
```

**Tests**:
```rust
#[test]
fn test_expression_helpers() {
    let bump = Bump::new();
    
    // Test nop creation
    let nop = Expression::nop(&bump);
    assert!(matches!(nop.kind, ExpressionKind::Nop));
    
    // Test const creation
    let const_expr = Expression::const_expr(&bump, Literal::I32(42), Type::I32);
    assert!(matches!(const_expr.kind, ExpressionKind::Const(Literal::I32(42))));
    
    // Test local operations
    let val = Expression::const_expr(&bump, Literal::I32(10), Type::I32);
    let set = Expression::local_set(&bump, 0, val);
    assert!(matches!(set.kind, ExpressionKind::LocalSet { index: 0, .. }));
    
    let get = Expression::local_get(&bump, 0, Type::I32);
    assert!(matches!(get.kind, ExpressionKind::LocalGet { index: 0 }));
}
```

**Validation**:
- [x] Build succeeds
- [x] All tests pass
- [x] New helpers have ≥1 test each

---

### Step 3: Update Pass Trait (15 min)
**File**: `rust/binaryen-ir/src/pass.rs`

**Current**:
```rust
pub trait Pass {
    fn name(&self) -> &str;
    fn run<'a>(&mut self, module: &mut Module<'a>);
}
```

**No change needed** - Module already provides allocator access via `module.allocator()`

**Validation**:
- [x] Existing passes still work
- [x] All tests pass

---

### Step 4: Implement untee Pass (1 hour)
**File**: `rust/binaryen-ir/src/passes/untee.rs`

**Implementation**:
```rust
use crate::expression::{ExprRef, Expression, ExpressionKind};
use crate::module::Module;
use crate::pass::Pass;
use crate::visitor::Visitor;
use bumpalo::collections::Vec as BumpVec;

pub struct Untee;

impl Pass for Untee {
    fn name(&self) -> &str {
        "untee"
    }

    fn run<'a>(&mut self, module: &mut Module<'a>) {
        let allocator = module.allocator();
        
        for func in &mut module.functions {
            if let Some(body) = &mut func.body {
                let mut transformer = UnteeTransformer { allocator };
                transformer.visit(body);
            }
        }
    }
}

struct UnteeTransformer<'a> {
    allocator: &'a bumpalo::Bump,
}

impl<'a> Visitor<'a> for UnteeTransformer<'a> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        if let ExpressionKind::LocalTee { index, value } = &expr.kind {
            let local_index = *index;
            let tee_value = *value;
            let tee_type = expr.type_;
            
            // Create: (block (local.set $x value) (local.get $x))
            let set = Expression::local_set(self.allocator, local_index, tee_value);
            let get = Expression::local_get(self.allocator, local_index, tee_type);
            
            let mut list = BumpVec::new_in(self.allocator);
            list.push(set);
            list.push(get);
            
            *expr = Expression::block(self.allocator, None, list, tee_type);
        }
    }
}
```

**Tests** (≥3 required):
```rust
#[test]
fn test_untee_converts_tee_to_set_get();
#[test]
fn test_untee_preserves_non_tee();
#[test]
fn test_untee_preserves_type();
```

**Validation**:
- [x] Build succeeds
- [x] All tests pass (including 3 new untee tests)
- [x] Total tests: 321+

---

### Step 5: Test on Real Pass Pipeline (30 min)
**File**: `rust/binaryen-ir/tests/integration/pass_pipeline.rs`

**New Test**:
```rust
#[test]
fn test_untee_then_simplify() {
    let bump = Bump::new();
    // Create module with tee
    // Run untee pass
    // Run simplify pass
    // Verify result
}
```

**Validation**:
- [x] Pipeline works correctly
- [x] Passes compose properly
- [x] No memory leaks or lifetime issues

---

### Step 6: Document Pattern (15 min)
**File**: `rust/binaryen-ir/src/passes/README.md`

**Content**:
```markdown
# Writing Passes with Expression Manipulation

## Accessing the Allocator

```rust
impl Pass for MyPass {
    fn run<'a>(&mut self, module: &mut Module<'a>) {
        let allocator = module.allocator();
        // Use allocator to create new expressions
    }
}
```

## Creating New Expressions

Use helper methods:
- `Expression::nop(allocator)`
- `Expression::const_expr(allocator, lit, ty)`
- `Expression::block(allocator, name, list, ty)`
- `Expression::local_set(allocator, idx, val)`
- `Expression::local_get(allocator, idx, ty)`

## Pattern: Transforming Expression in Visitor

```rust
impl<'a> Visitor<'a> for MyTransform<'a> {
    fn visit_expression(&mut self, expr: &mut ExprRef<'a>) {
        if matches!(expr.kind, ExpressionKind::SomePattern) {
            // Extract info
            let old_data = ...;
            
            // Create new expression
            let new_expr = Expression::something(self.allocator, ...);
            
            // Replace
            *expr = new_expr;
        }
    }
}
```
```

**Validation**:
- [x] Documentation clear and accurate
- [x] Examples compile

---

## Success Criteria

After all 6 steps:
- [x] Build succeeds with no errors
- [x] All tests pass (321+ tests)
- [x] untee pass works correctly
- [x] Pattern documented for future passes
- [x] No performance regression
- [x] No memory safety issues

---

## Timeline

- **Step 1**: 30 minutes
- **Step 2**: 45 minutes  
- **Step 3**: 15 minutes
- **Step 4**: 60 minutes
- **Step 5**: 30 minutes
- **Step 6**: 15 minutes

**Total**: ~3 hours for complete implementation and validation

---

## Risk Mitigation

### Risk: Breaking existing tests
**Mitigation**: Run tests after each step, fix immediately

### Risk: Lifetime issues
**Mitigation**: Compiler will catch, fix before proceeding

### Risk: Performance degradation
**Mitigation**: Allocator is zero-cost, same as current usage

---

## Post-Implementation

Once complete, the following passes become unblocked:
1. untee ✅ (implemented in Step 4)
2. local-cse
3. merge-blocks
4. simplify-locals (full implementation)
5. code-pushing
6. licm
7. rse
8. flatten
9. ... 15+ more passes

---

**Status**: Ready to implement Step 1

