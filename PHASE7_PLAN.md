# Phase 7: Optimization Infrastructure & First Passes

## Goal
Establish the framework for running transformation passes on the IR, implement the first actual validation-safe optimizations, and ensure the pipeline maintains module integrity.

## Step-by-Step Plan

### Step 1: Core Pass Infrastructure
**Objective**: Build the engine that executes passes.
- **Action**: Create `rust/binaryen-ir/src/pass.rs`.
- **Details**:
    - Define `trait Pass` with `fn run(&mut self, module: &mut Module)`.
    - Implement `struct PassRunner` that manages a queue of `Box<dyn Pass>`.
    - Add `run()` method to sequentially execute passes.
- **Verification**: 
    - Create a unit test with a `MockPass` that performs a trivial mutation (e.g., appends "_visited" to function names).
    - Assert the mutation occurred.

### Step 2: Peephole Optimizer (`SimplifyIdentity`)
**Objective**: Implement the first analysis-driven transformation.
- **Action**: Create `rust/binaryen-ir/src/passes/simplify_identity.rs`.
- **Details**:
    - Implement `Pass` for a new `SimplifyIdentity` struct.
    - Use the mutable `Visitor` to traverse expressions.
    - Detect and rewrite algebraic identities:
        - `i32.add(x, 0) -> x`
        - `i32.mul(x, 1) -> x`
        - `i32.sub(x, 0) -> x`
- **Verification**:
    - Construct an IR with `(i32.add (local.get 0) (i32.const 0))`.
    - Run the pass.
    - Assert the result is just `(local.get 0)`.

### Step 3: The Validation Pipeline
**Objective**: Ensure "Safe Points" by integrating the Validator from Phase 6.
- **Action**: Modify `PassRunner` in `pass.rs`.
- **Details**:
    - Add a configuration flag `validate_after_pass: bool`.
    - In the run loop, invoke `Validator::validate()` after every pass execution.
    - Panic or return error if validation fails after a pass (catches compiler bugs early).
- **Verification**:
    - Create a "BrokenPass" that introduces a type error (e.g., changes an `i32` const to `f32` in an `i32` context).
    - Run it via `PassRunner` with validation enabled.
    - Assert that the runner reports a validation error.

### Step 4: Basic Dead Code Elimination (DCE)
**Objective**: Implement a structural optimization handling control flow.
- **Action**: Create `rust/binaryen-ir/src/passes/dce.rs`.
- **Details**:
    - Traverse blocks.
    - If an instruction (`Unreachable`, `Break`, `Return`) diverts control flow, remove all subsequent instructions in that block.
- **Verification**:
    - Input: `(block (return) (call $foo))`
    - Expected Output: `(block (return))`
    - Verify `call $foo` is removed.

## Success Criteria for Phase 7
1. `PassRunner` can execute a chain of passes.
2. `SimplifyIdentity` correctly reduces code size/complexity.
3. The pipeline automatically catches regressions via the integrated Validator.
