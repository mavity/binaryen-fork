# Phase 6: Validation & Refinement

**Status**: In Progress
**Date**: January 14, 2026

## ✅ Achievements
1.  **Immutable Traversal**: Implemented `ReadOnlyVisitor` trait in `rust/binaryen-ir/src/visitor.rs`. This allows analyzing the IR without requiring mutable access, which is critical for validation and analysis passes.
2.  **Validator Skeleton**: Created `Validator` struct in `rust/binaryen-ir/src/validation.rs`.
3.  **Basic Validation Rules**:
    -   **Binary Operations**: Checks operand type equality (e.g., `i32 + f32` fails).
    -   **Function Return**: Verifies that the body block type matches the function's declared result type.
    -   **Call Integrity**: Checks that the target function exists and argument types match (basic single-value types).
4.  **Integration**:
    -   Exported `Validator` from `binaryen-ir`.
    -   Added regression test `test_validation_failure` which successfully catches invalid IR.

## 🚧 Current Blockers / TODOs
1.  **Local Variable Access**: `LocalGet`/`LocalSet` validation is a placeholder. Needs to verify index against `Function.params` + `Function.vars`.
2.  **Control Flow**: `Block`, `Loop`, `If` validation is minimal. Need to check label types for `Break` targets.
3.  **Type System**: Struct/Tuple support is marked as TODO in validation logic.

## 📅 Next Steps
1.  **Complete Validator**: Fill in the TODOs for Local variable bounds checking and basic control flow verification.
2.  **Phase 7: Pass Management**: Design the `Pass` trait and `PassRunner` to begin supporting optimizations.
