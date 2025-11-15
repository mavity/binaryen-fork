# Binaryen to Rust Conversion Plan

## Executive Summary

This document provides a systematic, component-by-component plan for converting Binaryen from C++ to Rust in a safe, tested manner. The conversion is designed to be incremental, with each phase bookended by validation tests to ensure correctness and maintain backward compatibility.

**Current State:**
- ~150,000 lines of C++ code
- CMake-based build system
- Comprehensive test suite (lit tests, check.py, unit tests)
- Multiple tools (wasm-opt, wasm-as, wasm-dis, wasm2js, etc.)
- C API and JavaScript bindings
- Wide adoption in production toolchains (Emscripten, wasm-pack, etc.)

**Goals:**
1. Maintain 100% API compatibility during transition
2. Preserve all existing functionality
3. No performance regressions
4. Incremental conversion with continuous validation
5. Minimize disruption to users and dependent projects

## Conversion Strategy Overview

The conversion will follow a **bottom-up, incremental approach** with the following principles:

1. **Start with leaf components** (no dependencies on other Binaryen code)
2. **Maintain C API compatibility** throughout the process
3. **Use FFI boundaries** to allow C++ and Rust to coexist
4. **Test at every step** before moving to the next component
5. **Establish safety invariants** in Rust that match C++ assumptions
6. **Document each phase** with clear entry/exit criteria

## Phase 0: Infrastructure Setup (Weeks 1-2)

### Objectives
- Set up Rust build system alongside existing CMake
- Establish FFI patterns and best practices
- Create testing harness for Rust components
- Set up CI for Rust code

### Tasks

#### 0.1: Repository Structure
- [ ] Add `rust/` directory at repository root
- [ ] Create `Cargo.toml` workspace configuration
- [ ] Set up `.cargo/config.toml` for build settings
- [ ] Add Rust-specific `.gitignore` entries

#### 0.2: Build Integration
- [ ] Create CMake module to invoke Cargo
- [ ] Add `BUILD_RUST_COMPONENTS` CMake option (default: OFF)
- [ ] Configure Rust to output static libraries
- [ ] Link Rust static libraries into existing binaries

#### 0.3: FFI Foundation
- [ ] Create `rust/binaryen-ffi/` crate for FFI bindings
- [ ] Set up `cbindgen` for generating C headers from Rust
- [ ] Create C++ wrapper templates for Rust components
- [ ] Document FFI patterns and conventions

#### 0.4: Testing Infrastructure
- [ ] Port a sample test to validate Rust integration
- [ ] Add `cargo test` to CI pipeline
- [ ] Set up `cargo clippy` for linting
- [ ] Configure `cargo fmt` for code formatting
- [ ] Add miri for undefined behavior detection

#### 0.5: Documentation
- [ ] Create `rust/README.md` with conversion guidelines
- [ ] Document FFI patterns and safety requirements
- [ ] Create contribution guide for Rust code
- [ ] Set up rustdoc generation

### Exit Criteria
- ✅ Rust builds successfully in CI
- ✅ Sample FFI component works end-to-end
- ✅ All existing tests still pass
- ✅ Documentation is in place

## Phase 1: Utility Components (Weeks 3-6)

Convert standalone utility components that have minimal dependencies.

### 1.1: Basic Data Structures

**Components:**
- `src/support/` - Utility functions and data structures
- String handling utilities
- Arena allocators (critical for performance)
- Hash maps and sets

**Approach:**
```rust
// rust/binaryen-support/src/lib.rs
pub mod arena;
pub mod strings;
pub mod hash;
```

**Testing:**
- Unit tests for each data structure
- Property-based tests with `proptest`
- Benchmarks against C++ implementation
- Memory leak tests with `valgrind`/`miri`

**FFI Strategy:**
- Expose opaque handles to Rust types
- Provide C-compatible constructors/destructors
- Use `#[repr(C)]` for compatible structs

### 1.2: Literal Values

**Component:** `src/literal.h` - WebAssembly literal values (i32, i64, f32, f64)

**Approach:**
```rust
// rust/binaryen-core/src/literal.rs
#[repr(C)]
pub struct Literal {
    type_: Type,
    value: LiteralValue,
}

#[repr(C)]
union LiteralValue {
    i32_: i32,
    i64_: i64,
    f32_: u32, // Bitcast representation
    f64_: u64,
}
```

**Testing:**
- Test all literal operations (add, sub, mul, div, etc.)
- Test conversions between types
- Validate against WebAssembly spec tests
- Fuzzing with random literal combinations

### 1.3: Source Maps

**Component:** `src/source-map.h` - Source map reading/writing

**Approach:**
- Use existing `sourcemap` crate or write custom
- Expose C API for backward compatibility
- Maintain exact output format

**Testing:**
- Test source map generation
- Test source map parsing
- Round-trip tests
- Compare output with C++ version byte-for-byte

### Exit Criteria
- ✅ All utility components pass their test suites
- ✅ Performance is within 5% of C++ implementation
- ✅ No memory leaks detected
- ✅ FFI integration works seamlessly

## Phase 2: Type System (Weeks 7-10)

The WebAssembly type system is fundamental to everything else.

### 2.1: Core Types

**Component:** `src/wasm-type.h` - WebAssembly type definitions

**Approach:**
```rust
// rust/binaryen-core/src/types.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Type {
    None = 0,
    I32 = 1,
    I64 = 2,
    F32 = 3,
    F64 = 4,
    V128 = 5,
    FuncRef = 6,
    ExternRef = 7,
    AnyRef = 8,
    EqRef = 9,
    I31Ref = 10,
    DataRef = 11,
    // ... additional types
}

pub struct HeapType { /* ... */ }
pub struct Signature { /* ... */ }
```

**Testing:**
- Test type equality and subtyping
- Test type serialization/deserialization
- Validate against WebAssembly spec
- Property tests for type operations

### 2.2: Type Utilities

**Component:** 
- `src/wasm-type-printing.h` - Type printing
- `src/wasm-type-ordering.h` - Type ordering
- `src/wasm-type-shape.h` - Type shapes

**Testing:**
- Test all type printing formats
- Test type ordering consistency
- Validate type shapes

### Exit Criteria
- ✅ Type system is complete and tested
- ✅ All type operations match C++ behavior
- ✅ Performance is acceptable
- ✅ Integration tests pass

## Phase 3: IR Core (Weeks 11-16)

The intermediate representation is the heart of Binaryen.

### 3.1: Expression Nodes

**Component:** `src/wasm.h` - Expression definitions

**Approach:**
```rust
// rust/binaryen-ir/src/expression.rs
pub enum Expression {
    Const(ConstExpr),
    LocalGet(LocalGetExpr),
    LocalSet(LocalSetExpr),
    Block(BlockExpr),
    If(IfExpr),
    Loop(LoopExpr),
    Call(CallExpr),
    // ... many more variants
}

// Each expression type as a separate struct
pub struct BlockExpr {
    pub name: Name,
    pub list: Vec<Box<Expression>>,
    pub type_: Type,
}
```

**Key Considerations:**
- Use `Box` for heap allocation (similar to C++ arena)
- Implement `Clone` carefully (may need arena support)
- Ensure expression tree invariants

**Testing:**
- Test each expression type
- Test expression traversal
- Test expression equality
- Validate against WebAssembly spec

### 3.2: Module Structure

**Component:** `src/wasm.h` - Module, Function, Global, etc.

**Approach:**
```rust
// rust/binaryen-ir/src/module.rs
pub struct Module {
    pub memory: Memory,
    pub table: Vec<Table>,
    pub globals: Vec<Global>,
    pub functions: Vec<Function>,
    pub exports: Vec<Export>,
    pub imports: Vec<Import>,
    pub start: Option<FunctionRef>,
}

pub struct Function {
    pub name: Name,
    pub sig: Signature,
    pub vars: Vec<Type>,
    pub body: Box<Expression>,
}
```

**Testing:**
- Test module construction
- Test module validation
- Round-trip test: parse → IR → serialize
- Validate against existing .wasm files

### 3.3: Module Builder

**Component:** `src/wasm-builder.h` - Fluent API for building IR

**Approach:**
```rust
// rust/binaryen-ir/src/builder.rs
pub struct Builder {
    allocator: &'a Allocator,
}

impl Builder {
    pub fn make_const(&self, lit: Literal) -> Box<Expression> { /* ... */ }
    pub fn make_block(&self, name: Name, list: Vec<Box<Expression>>) -> Box<Expression> { /* ... */ }
    // ... etc
}
```

**Testing:**
- Test all builder methods
- Test complex expression building
- Compare with C++ builder output

### Exit Criteria
- ✅ Full IR is implemented in Rust
- ✅ All IR tests pass
- ✅ Module parsing and serialization works
- ✅ Performance is within acceptable range

## Phase 4: Binary Format (Weeks 17-20)

WebAssembly binary reading and writing.

### 4.1: Binary Reader

**Component:** `src/wasm-binary.h` - Binary format parsing

**Approach:**
```rust
// rust/binaryen-binary/src/reader.rs
pub struct WasmBinaryReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> WasmBinaryReader<'a> {
    pub fn read_module(&mut self) -> Result<Module, BinaryError> { /* ... */ }
}
```

**Testing:**
- Test against WebAssembly spec test suite
- Test malformed binary handling
- Test all WebAssembly features
- Fuzzing with malformed inputs

### 4.2: Binary Writer

**Component:** `src/wasm-stack.h` - Binary format emission

**Approach:**
```rust
// rust/binaryen-binary/src/writer.rs
pub struct WasmBinaryWriter {
    buffer: Vec<u8>,
}

impl WasmBinaryWriter {
    pub fn write_module(&mut self, module: &Module) -> Result<Vec<u8>, BinaryError> { /* ... */ }
}
```

**Testing:**
- Round-trip tests: read → write → read
- Byte-exact comparison with C++ output
- Test size optimization
- Validate with external tools (wabt, etc.)

### 4.3: Text Format

**Component:** `src/parser/` and `src/passes/Print.cpp` - Text format parsing/printing

**Testing:**
- Test against all .wat files in test suite
- Round-trip: parse → print → parse
- Validate output with external parsers

### Exit Criteria
- ✅ Can read/write all WebAssembly binary formats
- ✅ 100% compatibility with existing files
- ✅ Performance is acceptable
- ✅ All format tests pass

## Phase 5: Optimization Passes (Weeks 21-30)

This is the largest component with ~100+ optimization passes.

### 5.1: Pass Infrastructure

**Component:** `src/pass.h` - Pass registration and execution

**Approach:**
```rust
// rust/binaryen-passes/src/pass.rs
pub trait Pass {
    fn name(&self) -> &str;
    fn run(&mut self, module: &mut Module);
}

pub struct PassRunner {
    passes: Vec<Box<dyn Pass>>,
}
```

### 5.2: Simple Passes (Priority Order)

Start with simpler, self-contained passes:

1. **Vacuum** - Remove unnecessary code
2. **DeadCodeElimination** - Remove dead code
3. **Precompute** - Constant folding
4. **SimplifyLocals** - Local variable optimization
5. **CoalesceLocals** - Register allocation
6. **Inlining** - Function inlining
7. **OptimizeInstructions** - Peephole optimization

**Approach (per pass):**
- Convert pass to Rust
- Ensure identical behavior with C++
- Run pass on entire test suite
- Compare output with C++ version

### 5.3: Complex Passes

These require more careful conversion:

1. **ReReloop** - Control flow restructuring
2. **Asyncify** - Async transformation
3. **MemoryPacking** - Data segment optimization
4. **GlobalDCE** - Link-time optimization

### 5.4: Visitor Pattern

**Component:** `src/wasm-traversal.h` - IR traversal

**Approach:**
```rust
// rust/binaryen-ir/src/visitor.rs
pub trait Visitor {
    fn visit_expression(&mut self, expr: &mut Expression) {
        walk_expression(self, expr);
    }
    
    fn visit_const(&mut self, expr: &mut ConstExpr) {}
    fn visit_block(&mut self, expr: &mut BlockExpr) {}
    // ... etc
}

pub fn walk_expression<V: Visitor>(visitor: &mut V, expr: &mut Expression) {
    match expr {
        Expression::Block(block) => visitor.visit_block(block),
        Expression::If(if_) => visitor.visit_if(if_),
        // ... etc
    }
}
```

### Exit Criteria (per pass)
- ✅ Pass produces identical output to C++
- ✅ All tests pass
- ✅ Performance is acceptable
- ✅ Integration with other passes works

## Phase 6: Tools (Weeks 31-36)

Convert the command-line tools.

### 6.1: wasm-opt

**Component:** `src/tools/wasm-opt.cpp`

**Approach:**
- Main entry point in Rust
- Use `clap` for argument parsing
- Call into Rust modules
- Maintain exact CLI compatibility

### 6.2: Other Tools

Convert in order of complexity:
1. **wasm-as** - Assembler
2. **wasm-dis** - Disassembler  
3. **wasm-shell** - Interpreter shell
4. **wasm2js** - WebAssembly to JavaScript
5. **wasm-merge** - Module merger
6. **wasm-reduce** - Test case reducer
7. **wasm-metadce** - Meta-DCE
8. **wasm-ctor-eval** - Constructor evaluation
9. **wasm-emscripten-finalize** - Emscripten finalization

### Exit Criteria
- ✅ All tools produce identical output
- ✅ All command-line options work
- ✅ Performance is acceptable
- ✅ Integration tests pass

## Phase 7: APIs (Weeks 37-40)

### 7.1: C API

**Component:** `src/binaryen-c.h` and `src/binaryen-c.cpp`

**Approach:**
```rust
// rust/binaryen-c-api/src/lib.rs
#[no_mangle]
pub extern "C" fn BinaryenModuleCreate() -> *mut BinaryenModule {
    let module = Box::new(Module::new());
    Box::into_raw(module) as *mut BinaryenModule
}

#[no_mangle]
pub extern "C" fn BinaryenModuleDispose(module: *mut BinaryenModule) {
    if !module.is_null() {
        unsafe { Box::from_raw(module as *mut Module) };
    }
}
```

**Testing:**
- Run all C API tests
- Test with external consumers (if available)
- Verify ABI compatibility

### 7.2: JavaScript API

**Component:** `src/js/binaryen.js-post.js`

**Approach:**
- Use `wasm-bindgen` for Wasm bindings
- Maintain API compatibility
- Update build process

**Testing:**
- Run all JavaScript tests
- Test with AssemblyScript and other consumers
- Verify performance

### Exit Criteria
- ✅ C API is 100% compatible
- ✅ JavaScript API works
- ✅ All API tests pass
- ✅ External consumers work

## Phase 8: Finalization (Weeks 41-44)

### 8.1: Performance Optimization

- Profile Rust code
- Optimize hot paths
- Reduce allocations
- Improve parallelism

### 8.2: Documentation

- Complete rustdoc for all public APIs
- Update main README
- Create migration guide
- Document differences from C++

### 8.3: Deprecation of C++

- Mark C++ code as deprecated
- Create transition timeline
- Support both versions in parallel
- Plan for eventual C++ removal

### Exit Criteria
- ✅ Performance meets or exceeds C++
- ✅ Documentation is complete
- ✅ All tests pass
- ✅ Ready for production use

## Testing Strategy

### Continuous Validation

At every phase:

1. **Unit Tests**: Test individual components in isolation
2. **Integration Tests**: Test component interactions
3. **Regression Tests**: Ensure no existing functionality breaks
4. **Performance Tests**: Benchmark against C++ version
5. **Fuzzing**: Find edge cases and undefined behavior
6. **Property Tests**: Validate invariants with `proptest`

### Test Execution

```bash
# Run all tests
./check.py --rust

# Run Rust unit tests
cargo test --all

# Run Rust benchmarks
cargo bench

# Run fuzzing
cargo fuzz run <target>

# Check for undefined behavior
cargo miri test
```

### Validation Criteria

Before moving to the next phase:
- ✅ All tests pass (C++ and Rust)
- ✅ No performance regressions > 5%
- ✅ No memory leaks (valgrind/miri)
- ✅ Code review completed
- ✅ Documentation updated

## Risk Mitigation

### Technical Risks

| Risk | Mitigation |
|------|------------|
| Performance regression | Benchmark continuously, optimize hot paths |
| Memory usage increase | Profile memory, use arena allocators |
| API compatibility breaks | Maintain FFI layer, version compatibility |
| Unsafe code bugs | Minimize unsafe, use miri, code review |
| Build complexity | Maintain simple CMake integration |

### Process Risks

| Risk | Mitigation |
|------|------------|
| Timeline overruns | Start with high-value components, adjust scope |
| Resource constraints | Parallelize independent components |
| External dependency issues | Fork/vendor critical dependencies |
| Team knowledge gaps | Training, pair programming, documentation |

### Rollback Strategy

For each phase:
1. Work on feature branch
2. Maintain C++ fallback via CMake option
3. Can disable Rust components at any time
4. Full rollback possible until C++ removal

## Dependencies and Prerequisites

### Build Tools
- Rust toolchain (stable, 1.70+)
- Cargo
- CMake 3.16+
- C++17 compiler (for remaining C++ code)

### Rust Crates (anticipated)
- `cbindgen` - Generate C headers
- `cxx` - C++ interop (if needed)
- `clap` - CLI argument parsing
- `proptest` - Property-based testing
- `criterion` - Benchmarking
- `rayon` - Parallelism
- `parking_lot` - Better synchronization primitives
- `bumpalo` - Arena allocation
- `ahash` - Fast hashing

### Development Tools
- `rust-analyzer` - IDE support
- `cargo-fuzz` - Fuzzing
- `cargo-miri` - Undefined behavior detection
- `cargo-flamegraph` - Profiling
- `valgrind` - Memory checking

## Success Metrics

### Technical Metrics
- ✅ 100% test coverage maintained
- ✅ Performance within 5% of C++ (prefer better)
- ✅ Memory usage within 10% of C++
- ✅ Zero undefined behavior (miri clean)
- ✅ No API breaking changes

### Process Metrics
- ✅ Code review completed for each phase
- ✅ Documentation updated continuously
- ✅ CI pipeline always green
- ✅ No major blockers > 1 week

### Adoption Metrics
- ✅ Successful integration with Emscripten
- ✅ Successful integration with wasm-pack
- ✅ Community feedback positive
- ✅ No major bug reports from conversion

## Timeline Summary

| Phase | Duration | Component |
|-------|----------|-----------|
| 0 | 2 weeks | Infrastructure setup |
| 1 | 4 weeks | Utility components |
| 2 | 4 weeks | Type system |
| 3 | 6 weeks | IR core |
| 4 | 4 weeks | Binary format |
| 5 | 10 weeks | Optimization passes |
| 6 | 6 weeks | Tools |
| 7 | 4 weeks | APIs |
| 8 | 4 weeks | Finalization |
| **Total** | **44 weeks** | **~11 months** |

**Note:** Timeline assumes dedicated team. Actual duration may vary based on:
- Team size and experience
- Parallel work streams
- Complexity discoveries
- External dependencies

## Alternative Approaches Considered

### 1. Complete Rewrite
**Pros:** Clean slate, best practices from start
**Cons:** High risk, long time without validation, potential for missing features
**Decision:** Rejected - too risky

### 2. Gradual Type-by-Type
**Pros:** Very incremental
**Cons:** Too slow, hard to validate
**Decision:** Rejected - inefficient

### 3. Top-Down (Tools First)
**Pros:** Early visible progress
**Cons:** Requires full stack, harder to test
**Decision:** Rejected - riskier

### 4. Bottom-Up Component-by-Component (Chosen)
**Pros:** Safe, testable, incremental, can validate continuously
**Cons:** Slower initial progress
**Decision:** Accepted - best balance of safety and progress

## Resources

### Documentation
- [Rust FFI Guide](https://doc.rust-lang.org/nomicon/ffi.html)
- [WebAssembly Specification](https://webassembly.github.io/spec/)
- [Binaryen Wiki](https://github.com/WebAssembly/binaryen/wiki)

### Similar Projects
- [Wasmtime](https://github.com/bytecodealliance/wasmtime) - WebAssembly runtime in Rust
- [wasmer](https://github.com/wasmerio/wasmer) - WebAssembly runtime in Rust
- [walrus](https://github.com/rustwasm/walrus) - WebAssembly transformation library in Rust

### Community
- Rust WebAssembly Working Group
- WebAssembly Community Group
- Binaryen Contributors

## Conclusion

This plan provides a systematic, low-risk approach to converting Binaryen from C++ to Rust. By following a bottom-up, component-by-component strategy with continuous testing and validation, we can achieve a successful conversion while maintaining stability and compatibility for all users.

The key to success is:
1. **Incremental progress** - Small, testable steps
2. **Continuous validation** - Test after every change
3. **Safety first** - Use Rust's type system to prevent bugs
4. **Performance focus** - Match or exceed C++ performance
5. **Community engagement** - Keep users informed and involved

With careful execution, this conversion will result in a more maintainable, safer, and equally performant Binaryen implementation that will serve the WebAssembly ecosystem for years to come.
