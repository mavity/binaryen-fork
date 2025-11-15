# Binaryen Rust Conversion - Phase Checklists

This document provides detailed checklists for each phase of the conversion process.

## Phase 0: Infrastructure Setup ✅

### Build System Integration
- [ ] Create `rust/` directory structure
- [ ] Add `Cargo.toml` workspace configuration
- [ ] Create CMake module to invoke Cargo
- [ ] Add `BUILD_RUST_COMPONENTS` CMake option
- [ ] Configure static library linking
- [ ] Test build integration works

### FFI Foundation
- [ ] Create `rust/binaryen-ffi/` crate
- [ ] Set up `cbindgen` configuration
- [ ] Create C++ wrapper templates
- [ ] Document FFI patterns
- [ ] Create example FFI component

### Testing Infrastructure
- [ ] Add `cargo test` to CI
- [ ] Set up `cargo clippy` linting
- [ ] Configure `cargo fmt` formatting
- [ ] Add miri for UB detection
- [ ] Create test harness for Rust components

### Documentation
- [ ] Create `rust/README.md`
- [ ] Document FFI patterns
- [ ] Create contribution guidelines
- [ ] Set up rustdoc generation

### Exit Criteria
- [ ] Rust builds in CI
- [ ] Sample FFI component works end-to-end
- [ ] All existing C++ tests pass
- [ ] Documentation complete

---

## Phase 1: Utility Components

### 1.1: Support Library
- [ ] Convert string utilities
- [ ] Convert arena allocators
- [ ] Convert hash map utilities
- [ ] Add unit tests for each utility
- [ ] Benchmark against C++ version
- [ ] Document performance characteristics

### 1.2: Literal Values
- [ ] Define `Literal` type in Rust
- [ ] Implement literal operations
- [ ] Add conversion functions
- [ ] Test against WebAssembly spec
- [ ] Add fuzzing tests
- [ ] Validate with property tests

### 1.3: Source Maps
- [ ] Implement source map reader
- [ ] Implement source map writer
- [ ] Add round-trip tests
- [ ] Compare output with C++ version
- [ ] Test with real source maps

### Exit Criteria
- [ ] All utility tests pass
- [ ] Performance within 5% of C++
- [ ] No memory leaks detected
- [ ] FFI integration verified

---

## Phase 2: Type System

### 2.1: Core Types
- [ ] Define `Type` enum
- [ ] Implement `HeapType`
- [ ] Implement `Signature`
- [ ] Add type equality checks
- [ ] Implement subtyping
- [ ] Test against WebAssembly spec

### 2.2: Type Utilities
- [ ] Implement type printing
- [ ] Implement type ordering
- [ ] Implement type shapes
- [ ] Add comprehensive tests
- [ ] Validate output format

### Exit Criteria
- [ ] Type system complete
- [ ] All type operations match C++
- [ ] Performance acceptable
- [ ] Integration tests pass

---

## Phase 3: IR Core

### 3.1: Expression Nodes
- [ ] Define `Expression` enum
- [ ] Implement all expression types
- [ ] Add expression builder
- [ ] Implement traversal
- [ ] Add validation
- [ ] Test each expression type

### 3.2: Module Structure
- [ ] Implement `Module` struct
- [ ] Implement `Function` struct
- [ ] Implement `Global`, `Table`, `Memory`
- [ ] Add imports and exports
- [ ] Test module construction
- [ ] Add validation

### 3.3: Module Builder
- [ ] Create fluent builder API
- [ ] Test all builder methods
- [ ] Compare output with C++
- [ ] Add documentation examples

### Exit Criteria
- [ ] Full IR implemented
- [ ] All IR tests pass
- [ ] Parsing/serialization works
- [ ] Performance acceptable

---

## Phase 4: Binary Format

### 4.1: Binary Reader
- [ ] Implement WebAssembly binary parser
- [ ] Handle all section types
- [ ] Test against spec suite
- [ ] Test malformed input handling
- [ ] Add fuzzing tests

### 4.2: Binary Writer
- [ ] Implement binary emission
- [ ] Test round-trip (read → write → read)
- [ ] Compare output byte-for-byte
- [ ] Test size optimization
- [ ] Validate with external tools

### 4.3: Text Format
- [ ] Implement WAT parser
- [ ] Implement WAT printer
- [ ] Test against all .wat files
- [ ] Test round-trip parsing
- [ ] Validate with external parsers

### Exit Criteria
- [ ] Can read/write all formats
- [ ] 100% compatibility
- [ ] Performance acceptable
- [ ] All format tests pass

---

## Phase 5: Optimization Passes

### Pass Conversion Priority

#### Simple Passes (Week 1-3)
- [ ] Vacuum - Remove unnecessary code
- [ ] DeadCodeElimination
- [ ] Precompute - Constant folding
- [ ] Test each pass individually
- [ ] Validate output matches C++

#### Medium Passes (Week 4-6)
- [ ] SimplifyLocals
- [ ] CoalesceLocals
- [ ] OptimizeInstructions
- [ ] Test pass interactions
- [ ] Benchmark performance

#### Complex Passes (Week 7-10)
- [ ] Inlining
- [ ] ReReloop
- [ ] Asyncify
- [ ] MemoryPacking
- [ ] GlobalDCE

### Infrastructure
- [ ] Implement Pass trait
- [ ] Create PassRunner
- [ ] Implement visitor pattern
- [ ] Add pass registration
- [ ] Test pass pipeline

### Exit Criteria (per pass)
- [ ] Output identical to C++
- [ ] All tests pass
- [ ] Performance acceptable
- [ ] Integration verified

---

## Phase 6: Tools

### 6.1: wasm-opt
- [ ] Convert main entry point
- [ ] Use clap for argument parsing
- [ ] Maintain CLI compatibility
- [ ] Test all command options
- [ ] Validate output

### 6.2: Other Tools (in order)
- [ ] wasm-as (Assembler)
- [ ] wasm-dis (Disassembler)
- [ ] wasm-shell (Interpreter)
- [ ] wasm2js
- [ ] wasm-merge
- [ ] wasm-reduce
- [ ] wasm-metadce
- [ ] wasm-ctor-eval
- [ ] wasm-emscripten-finalize

### Exit Criteria
- [ ] All tools produce identical output
- [ ] All CLI options work
- [ ] Performance acceptable
- [ ] Integration tests pass

---

## Phase 7: APIs

### 7.1: C API
- [ ] Convert all C API functions
- [ ] Test ABI compatibility
- [ ] Run all C API tests
- [ ] Test with external consumers
- [ ] Document any changes

### 7.2: JavaScript API
- [ ] Use wasm-bindgen
- [ ] Maintain API compatibility
- [ ] Update build process
- [ ] Run all JS tests
- [ ] Test with AssemblyScript

### Exit Criteria
- [ ] C API 100% compatible
- [ ] JavaScript API works
- [ ] All API tests pass
- [ ] External consumers work

---

## Phase 8: Finalization

### 8.1: Performance Optimization
- [ ] Profile Rust code
- [ ] Optimize hot paths
- [ ] Reduce allocations
- [ ] Improve parallelism
- [ ] Benchmark comprehensive suite

### 8.2: Documentation
- [ ] Complete rustdoc
- [ ] Update main README
- [ ] Create migration guide
- [ ] Document C++ differences
- [ ] Add usage examples

### 8.3: C++ Deprecation
- [ ] Mark C++ as deprecated
- [ ] Create transition timeline
- [ ] Support parallel versions
- [ ] Plan C++ removal

### Exit Criteria
- [ ] Performance meets/exceeds C++
- [ ] Documentation complete
- [ ] All tests pass
- [ ] Production ready

---

## Continuous Integration Checks

For every phase, ensure:

- [ ] `cargo test --all` passes
- [ ] `cargo clippy --all -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo miri test` passes (where applicable)
- [ ] Existing C++ tests still pass
- [ ] No performance regressions > 5%
- [ ] Documentation updated
- [ ] Code review completed

---

## Risk Tracking

### Current Risks

| Risk | Status | Mitigation |
|------|--------|------------|
| Performance regression | 🟡 Monitor | Continuous benchmarking |
| Memory usage increase | 🟡 Monitor | Profile and optimize |
| API breakage | 🟢 Low | FFI layer maintains compatibility |
| Timeline overrun | 🟡 Possible | Adjust scope, prioritize |

**Legend:**
- 🟢 Low risk
- 🟡 Medium risk - monitoring required
- 🔴 High risk - immediate attention needed

---

## Progress Tracking Template

Use this template to track progress:

```markdown
## Week [X]: [Phase Name]

### Completed
- [ ] Item 1
- [ ] Item 2

### In Progress
- [ ] Item 3

### Blocked
- [ ] Item 4 - blocked by [reason]

### Next Week
- [ ] Item 5
- [ ] Item 6

### Notes
[Any important observations or decisions]
```

---

## Component Dependency Map

```
Utilities (Phase 1)
    ↓
Type System (Phase 2)
    ↓
IR Core (Phase 3)
    ↓
Binary Format (Phase 4)
    ↓
Passes (Phase 5) ← Can parallelize individual passes
    ↓
Tools (Phase 6) ← Can parallelize individual tools
    ↓
APIs (Phase 7)
    ↓
Finalization (Phase 8)
```

This shows the critical path. Components at the same level can be worked on in parallel by different team members.
