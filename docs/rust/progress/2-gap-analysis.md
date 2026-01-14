# Binaryen Rust Port: Comprehensive Gap Analysis
**Date**: January 2026 | **Status**: Work in Progress

---

## Executive Summary

The Rust port is **further along than the task documentation suggests**, with substantial implementations for Types, IR, and Binary Parsing already in place. However, there are significant gaps in:

1. **Testing Strategy**: The promised "parity with C++ testing" is incomplete.
2. **Tools & CLIs**: No command-line interfaces exist yet.
3. **Optimization Passes**: Only 2 of ~50 passes are implemented.
4. **Integration**: Critical missing integration tests and fuzzing.

The code is ahead of the documentation, but the delivery against the project's stated goals (100% C API compatibility, equal performance, comprehensive testing) is behind.

---

## Part 1: Implementation Status vs. Documentation

### Phase 0 & 1: Infrastructure & Support Utilities
**Status**: ✅ **Complete and Verified**

- [x] Cargo workspace setup with `cmake` integration
- [x] `cbindgen` golden header generation and CI checks
- [x] `rust/binaryen-support`: StringInterner, Arena allocators, AHash helpers
- [x] FFI wrappers and smoke tests in `test/rust_consumer/`

**Evidence**:
- `rust/Cargo.toml` lists all crates; `rust/binaryen-support/src/` has `lib.rs`, `strings.rs`, `arena.rs`
- `rust/binaryen-ffi/src/lib.rs` exports FFI symbols with `#[no_mangle]`
- CI job `.github/workflows/rust-ci.yml` runs golden header test and linkage validation

---

### Phase 2: Types & TypeStore
**Status**: ✅ **Implemented** | ⚠️ **Documentation Mismatch** | 🔴 **Testing Gap**

**What the Task List Says**:
```
- [ ] Implement `TypeStore` (interning) and API for interned `Signature` and `HeapType`
- [ ] Add `#[repr(C)]`-safe FFI wrappers for Types in `binaryen-ffi`
- [ ] Add a C++ round-trip smoke test in `test/rust_consumer/test_ffi_type_roundtrip.cpp`
```

**What Actually Exists**:

| Component | File | Lines | Status |
|-----------|------|-------|--------|
| Type enum | `binaryen-core/src/type.rs` | ~400 | ✅ Complete with variants for basic types, references, and signatures |
| Signature & HeapType | `binaryen-core/src/type.rs` | Included | ✅ Defined with `#[repr(C)]` layouts |
| **TypeStore** | `binaryen-core/src/type_store.rs` | 135 | ✅ Thread-safe interning via `RwLock<HashMap>` with `Signature` canonicalization |
| FFI Wrappers | `binaryen-ffi/src/type_ffi.rs` | ~200 | ✅ Exports `binaryen_ffi_type_new()`, `binaryen_ffi_signature_intern()`, etc. |
| Literal Support | `binaryen-core/src/literal.rs` | ~150 | ✅ i32, i64, f32, f64 literals with printing |

**Discrepancies**:
- Task list marks `TypeStore` as "Todo" but implementation exists and is substantial.
- FFI wrappers marked as "Todo" but `type_ffi.rs` is present and functional.
- **Critical Missing**: `test/rust_consumer/test_ffi_type_roundtrip.cpp` **does not exist**. This is the only "hard" deliverable for Phase 2 that is provably missing.

**Recommendation**: 
- [ ] Create the C++ round-trip test immediately to validate the Rust TypeStore against C++ expectations.

---

### Phase 3: IR Core
**Status**: ✅ **Substantially Implemented** | ⚠️ **Not in Original Phase 2 Plan**

The task list anticipated Phase 3 to be later, but implementation has already begun.

**Implemented Components**:

| Component | File | Lines | Status |
|-----------|------|-------|--------|
| Expression enum | `binaryen-ir/src/expression.rs` | 327 | ✅ Complete: Block, Const, Unary, Binary, Call, LocalGet/Set/Tee, GlobalGet/Set, Load, Store, Memory operations |
| Module & Functions | `binaryen-ir/src/module.rs` | ~300 | ✅ Module, Function, Export, Import structures |
| IR Builder | `binaryen-ir/src/expression.rs` | Included | ✅ Builder trait for constructing IR from scratch |
| Visitor Pattern | `binaryen-ir/src/visitor.rs` | ~200 | ✅ Basic traversal with `walk_mut()` and control flow |
| Validation | `binaryen-ir/src/validation.rs` | ~300 | ✅ Type checking and structural validation |

**Verdict**: Phase 3 is ~70% complete. Basic IR is solid; memory management (bumpalo arena) is proven.

---

### Phase 4: Binary Format (Read/Write & Text Parser)
**Status**: ✅ **Reader Implemented** | ⚠️ **Writer Partial** | 🔴 **Text Parser Missing**

**Implementation Status**:

| Component | File | Lines | Status |
|-----------|------|-------|--------|
| Binary Reader | `binaryen-ir/src/binary_reader.rs` | 1,853 | ✅ Nearly complete WebAssembly binary parser (LEB128, sections, opcodes) |
| Binary Writer | `binaryen-ir/src/binary_writer.rs` | ~500 | 🟡 Partial—basic structure exists but untested |
| Text Parser (WAT) | N/A | 0 | 🔴 **Missing** |
| Text Printer | `binaryen-ir/src/*` | Unclear | ⚠️ No dedicated module found |

**Verdict**: Binary reading is feature-complete. Text format (WAT parsing and pretty-printing) is not yet started.

---

## Part 2: Tools & CLIs

**Status**: 🔴 **0% Complete** (Phase 6)

**What the Plan Requires**:
- `wasm-opt`: The primary CLI tool for applying optimization passes
- `wasm-as`: Assembler
- `wasm-dis`: Disassembler
- `wasm2js`: JavaScript generator

**What Exists**:
- **Zero Rust CLIs**. The `rust/Cargo.toml` workspace contains only library crates.
- No `[[bin]]` targets in any crate.
- No `main.rs` files found in `rust/`.

**Consequence**:
The Rust port currently cannot be "run" outside of the library. All testing is unit-test only. No end-to-end pipeline can be exercised.

**Required Work**:
1. [ ] Create `rust/binaryen-tools` crate (or expand `binaryen-ir` with a `bin` target)
2. [ ] Implement `wasm-opt` CLI using `clap` or `structopt`
3. [ ] Expose Rust passes through command-line options
4. [ ] Add `--help` documentation matching C++ tool behavior

---

## Part 3: Optimization Passes

**Status**: 🟡 **5% Complete** (Phase 5)

### Implemented Passes

| Pass | File | Lines | Quality | Notes |
|------|------|-------|---------|-------|
| Code Folding | `binaryen-ir/src/passes/code_folding.rs` | ~100 | ✅ Real | Handles `x+0`, `x*1`, `x-0`; uses Visitor pattern correctly. Includes unit tests. |
| Dead Code Elimination (DCE) | `binaryen-ir/src/passes/dce.rs` | ~80 | ✅ Real | Truncates blocks after `unreachable`; basic but functional. Includes unit tests. |

### Missing Passes

The C++ version has ~50 optimization passes. Critical ones missing in Rust include:

- **Simplification**: More complex rewrite rules (algebraic identities, control flow folding)
- **Inlining**: Function inlining and call site optimization
- **Precompute**: Constant folding and compile-time evaluation
- **Loop Optimization**: Invariant hoisting, strength reduction
- **Memory Optimization**: Load/store fusion, dead store elimination
- **Relooping**: Converting irreducible control flow
- **WASM-specific**: Bulk memory operations, SIMD simplification

**Verdict**: The pass infrastructure (PassRunner, Visitor, validation) is solid. Converting the remaining 48 passes is the bulk of Phase 5 work.

---

## Part 4: Testing Strategy

### A. Unit Tests
**Status**: ✅ **In Place**

- Each crate has `#[test]` functions embedded in source.
- `cargo test` passes (verified in terminal history).
- Coverage: Support libs, Type system, basic Passes.

**Verdict**: Unit tests are good; they validate individual components in isolation.

---

### B. File-Based Tests (Lit Integration)
**Status**: 🔴 **Missing**

**The Standard**: C++ Binaryen uses `lit` (LLVM Integrated Tester) to run hundreds of `.wast` and `.wasm` test files.

**What Should Exist**:
- A `rust-lit-adapter` binary that can be invoked by `lit` to:
  - Parse a `.wast` file
  - Run it through the Rust pipeline
  - Compare output to expected results

**What Exists**:
- **Nothing**. There is no integration between `lit` and the Rust port.
- Test files in `test/` are C++-centric.

**Consequence**: 
You cannot validate that the Rust `dce` pass behaves identically to the C++ `dce` pass on standard test vectors.

**Required Work**:
1. [ ] Create a `rust/tools/lit-adapter.rs` binary
2. [ ] Implement `.wast` file reading and module construction
3. [ ] Call pass infrastructure and serialize output
4. [ ] Integrate into `lit` workflow

---

### C. Integration Tests (Rust Consumer)
**Status**: 🟡 **Partial**

**What Exists**:
- Directory: `test/rust_consumer/`
- Files: `test_ffi_smoke.c`, `pipeline_demo.cpp`

**What Is Missing**:
- [ ] `test_ffi_type_roundtrip.cpp` — **Critical**. Validates TypeStore ABI.
- [ ] Comprehensive API Stress Tests — Check memory safety under adversarial C++ calls.
- [ ] Round-trip tests for IR modules — Create in Rust, serialize, deserialize, verify equality.

**Verdict**: Smoke tests exist but are minimal. The promised "round-trip" test for Phase 2 is not implemented.

---

### D. Fuzzing
**Status**: 🟡 **Partial**

**What Exists**:
- Directory: `rust/fuzz/fuzz_targets/`
- Files: `ahash.rs`, `arena.rs`, `fastmap.rs`, `interner.rs`
- These are **component-level fuzzers** testing data structures in isolation.

**What Is Missing**:
- [ ] IR Fuzzing — Generate random valid WebAssembly modules and run passes on them.
- [ ] Binary Fuzzing — Feed malformed `.wasm` files to the binary reader and verify graceful error handling.
- [ ] Pass Fuzzing — Run optimization passes on random IR and verify validity before/after.
- [ ] Differential Fuzzing — Compare Rust and C++ pass outputs on the same input.

**Consequence**: 
Crashes and undefined behavior in the Rust port are not being systematically discovered during CI.

**Required Work**:
1. [ ] Create IR fuzzer target that generates `Module` + `Expression` trees
2. [ ] Create binary fuzzer that feeds corrupted `.wasm` data to `BinaryReader`
3. [ ] Integrate into `cargo fuzz` CI job

---

### E. Regression Testing
**Status**: ⚠️ **Unclear**

**Question**: Does `./check.py --rust` work? The plan mentions running this in CI.

**Finding**: 
- `scripts/check.py` exists but is C++-centric.
- No evidence of a Rust-specific test runner.

**Required Work**:
1. [ ] Clarify if `check.py` has Rust hooks (search for `--rust` usage)
2. [ ] Create a `rust/check.sh` or extend `check.py` to run Rust tests systematically

---

## Summary Table: Completed vs. Outstanding Work

| Phase | Component | Status | Completeness | Blockers |
|-------|-----------|--------|--------------|----------|
| **0** | Infra & FFI | ✅ Done | 100% | None |
| **1** | Support Libs | ✅ Done | 100% | None |
| **2** | Types & TypeStore | ✅ Code | 95% | Missing C++ roundtrip test |
| **3** | IR Core | ✅ Code | 70% | Memory model stabilization, more expr nodes |
| **4** | Binary Format | 🟡 Reader done | 50% | WAT parser, text printer missing |
| **5** | Passes | 🟡 2 of 50 | 4% | Bulk of optimization logic |
| **6** | Tools/CLIs | 🔴 Not started | 0% | No CLI infrastructure |
| **7** | APIs (C/JS) | 🟡 C FFI partial | 30% | JS bindings, comprehensive coverage |
| **Testing** | File-based (Lit) | 🔴 Missing | 0% | Requires lit adapter |
| **Testing** | Fuzzing (IR/Binary) | 🔴 Missing | 0% | Requires fuzzer targets |
| **Testing** | Integration | 🟡 Partial | 40% | C++ roundtrip test missing |

---

## Recommended Prioritization

### Tier 1: Unblock the Passing Pipeline (Immediate, Next 2–4 weeks)
1. **Create C++ roundtrip test** (`test_ffi_type_roundtrip.cpp`)
   - Validates Phase 2 completion.
   - Unblocks downstream integration.
   
2. **Stabilize Arena & Memory Model**
   - Lock down lifetime semantics for IR nodes.
   - Document ownership model in FFI spec.

3. **Implement more Expression nodes** (as needed for upcoming passes)
   - Match C++ feature set.

### Tier 2: Implement Core Testing Infrastructure (4–8 weeks)
1. **Lit Adapter**
   - Allow `lit` to invoke Rust pipeline.
   - Enable file-based regression testing.

2. **IR Fuzzer Target**
   - Generate random valid modules.
   - Run passes and verify invariants.

3. **Binary Fuzzer Target**
   - Stress-test `BinaryReader` with malformed input.

### Tier 3: Complete Optimization Passes (8–16 weeks)
1. Prioritize high-impact passes:
   - **Simplification** (most commonly used)
   - **Precompute** (enables other optimizations)
   - **Inlining** (significant perf impact)
   - **Loop Optimization** (common pattern)

2. Build a shared library of reusable IR transformation utilities.

### Tier 4: CLI Tools (16–24 weeks)
1. Wrap the pass infrastructure in `wasm-opt` CLI.
2. Maintain command-line compatibility with C++ version.
3. Integrate into build system and CI.

---

## Documentation Corrections Needed

Update the task list (`docs/rust/progress/1-tasks.md`):

1. **Team IR Section**:
   - Check off TypeStore, Signature/HeapType FFI as ✅ Done.
   - Note that IR Expression enum is ✅ Done (Phase 3 ahead of schedule).
   - Add missing tasks: "Text parser (WAT)" and "Text printer".

2. **Team Parsers Section**:
   - Update: Binary reader ✅ done; binary writer 🟡 partial; text parser 🔴 missing.

3. **Testing Section**:
   - Clarify which tests are missing and why (Lit adapter, fuzzing, integration).

---

## Open Questions & Decisions Needed

1. **Architecture Decision**: Should `binaryen-binary` be a separate crate, or stay within `binaryen-ir`?
   - Current: Merged into `binaryen-ir` (practical but harder to test independently).

2. **WAT Parser Strategy**: Build from scratch or wrap C++ parser via FFI?
   - Current: No decision made; not started.

3. **Pass Conversion Order**: What is the priority order for the remaining 48 passes?
   - Should be driven by: frequency of use, impact on performance, dependencies on other passes.

4. **CLI Strategy**: Will Rust tools be drop-in replacements for C++ binaries, or coexist?
   - Implication: ABI compatibility, feature parity, testing strategy.

---

## Conclusion

**The Rust implementation is structurally sound but functionally incomplete.** Types, IR, and binary parsing are further along than the task list indicates, but:

- **Tools do not exist** (Phase 6 has not started).
- **Testing parity with C++ is not achieved** (Lit, fuzzing, and integration gaps).
- **Only 4% of optimization passes are done** (48 remain).

The next 3–6 months should focus on (1) validating the existing work with integration tests, (2) building the testing infrastructure (Lit, fuzzing), and (3) systematically converting the remaining passes. CLI tools can follow once the core pipeline is proven.

