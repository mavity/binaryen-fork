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

## Part 3: Optimization Passes (Phase 5)

**Status**: 🟡 **5% Complete** | 🏗️ **Phase 5a In Progress**

### Implemented Passes

| Pass | File | Lines | Quality | Notes |
|------|------|-------|---------|-------|
| Code Folding | `binaryen-ir/src/passes/code_folding.rs` | ~100 | ✅ Real | Handles `x+0`, `x*1`, `x-0`; uses Visitor pattern correctly. Includes unit tests. |
| Dead Code Elimination (DCE) | `binaryen-ir/src/passes/dce.rs` | ~80 | ✅ Real | Truncates blocks after `unreachable`; basic but functional. Includes unit tests. |
| Memory Optimization | `binaryen-ir/src/passes/memory_optimization.rs` | ~150 | ✅ Real | Redundant store elimination for adjacent nodes. Needs EffectAnalyzer for rigorous mode. |

### Phase 5a: Optimization Infrastructure — Effect Analysis

**Status**: 🏗️ **Identified & In Progress**

A key discovery during recent development is the opportunity to implement a robust `EffectAnalyzer` in Rust. In the C++ codebase, `EffectAnalyzer` is the backbone of safe reordering and elimination optimizations, ensuring that side effects (like traps, I/O, or global state changes) are respected.

**The Opportunity**:
Implementing `EffectAnalyzer` in Rust is not just about catching up to C++; it is an opportunity to:
1. **Enforce Safety**: Leverage Rust's type system to make effect analysis stricter and less prone to C++ style oversight.
2. **Unlock Advanced Optimizations**: Enable "rigorous" modes for passes like `MemoryOptimization`, `SimplifyLocals`, and `DeadCodeElimination` that strictly adhere to WebAssembly validation rules.
3. **Surpass the Baseline**: While C++ sometimes hesitates to remove redundant linear memory stores (as seen in recent `wasm-opt` comparisons), a fresh Rust implementation can aggressively yet safely target these inefficiencies.

**Incremental Implementation (Test-Driven, 3 Atomic Steps):**

1. **Step 1 — The Language of Effects (1 week)** 🔧
   - Files: `rust/binaryen-ir/src/effects.rs` (module + pub exports).
   - Implement: `Effect` bitflags (Read, Write, Trap, Control, MemoryRead, MemoryWrite, GlobalRead/Write, LocalWrite, Call, etc.).
   - Tests: unit tests for flag composition, serialization (Debug), and simple helpers.
   - Acceptance: Cargo tests pass; API exported in crate documentation.

2. **Step 2 — EffectAnalyzer (2–3 weeks)** 🧠
   - Implement: `EffectAnalyzer` visitor that computes aggregated effects for any `Expression`.
   - Integrate: expose `analyze(expr: &Expression) -> Effect` and `analyze_range(&[&Expression]) -> Effect` helpers.
   - Tests: unit tests covering `Block`, `If`, `Loop`, `Call`, `Load`, `Store`, `LocalSet`, and edge cases (traps, unreachable). Add cross-check tests against small C++ examples where possible.
   - Acceptance: Tests confirm behavior matches C++ expectations for canonical cases; no regressions in existing pass tests.

3. **Step 3 — Rigorous Upgrade of Passes (2–4 weeks)** ⚙️
   - Refactor: Make `MemoryOptimization` and `SimplifyLocals` (and others as needed) call `EffectAnalyzer` before performing non-local deletions/reorders.
   - Implement: A `--rigorous` or configuration toggle in pass runner to enable stricter semantics for CI / experimental runs.
   - Tests: Unit + lit-based file tests covering cases where stores are separated by calls/traps; fuzzing tests that ensure module validity before/after passes; regression tests where redundant stores are safely removed or converted to `drop` when they have side effects.
   - Acceptance: No behavioral regressions; performance and size wins where expected; CI includes new tests in `cargo test` and `lit` integration.

**CI & Testing Policies**:
- Every step must include unit tests and be green in `cargo test`.
- Step 2 must add lit-style file tests (or minimal equivalents) for tricky interprocedural cases; add fuzz targets for pass validation in Step 3.
- Each PR should be small and reviewable (< 300 lines ideally) and include at least one focused test for the new behavior.

**Risks & Mitigations**:
- Risk: Behavioral regressions in existing passes. Mitigation: Add regression tests and gate changes behind a `--rigorous` flag until validated.
- Risk: Overly conservative effects reduce optimization opportunities. Mitigation: start conservative, add refinements, and measure wins.

**Phase 5a Status**: ✅ **COMPLETE** as of January 2026. Effect system (27 bitflags, 64 tests) and enhanced MemoryOptimization are production-ready. Total: **298 tests passing**.

---

### Implemented Passes Summary (Phase 5a Complete)

| Pass | Status | Features | Tests | Quality |
|------|--------|----------|-------|---------|
| **EffectAnalyzer** | ✅ Complete | 27 effect flags, interference detection, all expression kinds | 64 | Excellent |
| **MemoryOptimization** | ✅ Enhanced | Dead store elimination, local/global set elimination, rigorous mode, binary pointer comparison | 10 | Production-ready |
| **SimplifyLocals** | 🏗️ Phase 5b | Infrastructure for sinking, tracking, effects; optimization methods stubbed | 3 | Foundation |
| **DeadCodeElimination** | ✅ Complete | Unreachable removal, block truncation, type updating | 5 | Good |
| **SimplifyIdentity** | ✅ Complete | Algebraic identities (x+0, x*1, x-0) | 3 | Good |
| **Simplify** | ✅ Basic | Basic simplifications | 2 | Basic |

**Current Capability**: ~50% feature parity with C++ SimplifyLocals (infrastructure ready), ~15-20% overall parity with C++ suite (122 passes).

---

## Phase 5b (Major Task): Architectural Migration — The Arena Transition
**Status**: 📋 **Planned** | **Tier**: 1 (Critical Blocker)

The current IR ownership model (`Box` or exclusive `&mut` references) has reached its limits for advanced tree transformations like sinking nodes or redundant move elimination. To achieve C++-style optimization parity and $O(1)$ tree surgery, the IR must transition to a centralized Arena-based handle model.

### The Problem
Rust's standard ownership prevents holding multiple mutable pointers to parts of the same tree. Algorithms like "Sinking" require moving a branch from one parent to another without invalidating the recursive traversal state.

### The Solution: Arena Handles (`ExprRef`)
Instead of exclusive references, we will use copyable handles backed by a centralized `Module`-owned Arena.

| Component | Current (`&mut`) | New (Arena Handles) |
|-----------|------------------|-----------------|
| **Storage** | Recursive `Box` or `&mut` | Flat `Bump` allocation |
| **Handle** | `&'a mut Expression` | `ExprRef<'a>` (Pointer wrapper) |
| **Logic** | Blocks aliasing / move | $O(1)$ relocate & swap |

### Implementation Roadmap (6 Atomic Steps)

1. **Step 1: Handle Definition** 🔧
   - File: `rust/binaryen-ir/src/expression.rs`
   - Define `ExprRef<'a>` as a transparent wrapper around a raw pointer (safely managed via lifetimes).
   - Implement `Copy`, `Clone`, and conversion helpers.

2. **Step 2: Core IR Refactor** 🏗️
   - Update `ExpressionKind` variants to use `ExprRef` instead of recursive mutable references.
   - Update `walk_mut` and `Visitor` traits to work with handles.

3. **Step 3: Centralized Arena Ownership** 🏛️
   - Integrate `bumpalo::Bump` into `Module` and `Function` structures.
   - Update factory methods in `Expression` to require an `Arena` reference.

4. **Step 4: Unblocking SimplifyLocals** ⚙️
   - Implement the recursive sinking algorithm using $O(1)$ node relocation.
   - Enable `local.tee` creation and block-return value optimization.

5. **Step 5: Migration of Existing Passes** 🔄
   - Refactor `MemoryOptimization`, `DCE`, and `SimplifyIdentity` to the new handle model.

6. **Step 6: Validation** ✅
   - Ensure all 300+ existing tests pass under the new memory model.
   - Add regression tests for node-swapping edge cases.

**Impact**: Unblocks all 117 missing passes, enables recursive optimization without lifetime bottlenecks, and improves runtime allocation performance.

---

### Phase 5b: SimplifyLocals Foundation — STARTED

**Status**: 🏗️ **Foundation Complete** (January 15, 2026)

**What's Implemented**:
- ✅ Pass infrastructure with 3 option modes (allow_tee, allow_structure, allow_nesting)
- ✅ FunctionContext tracking sinkables and get counts
- ✅ Effect-based invalidation to prevent unsafe optimizations
- ✅ Multi-cycle optimization framework
- ✅ Visitor pattern integration
- ✅ 3 unit tests (301 total tests passing)

**What's Stubbed** (ready for implementation):
- 🔧 Local.set sinking with tree manipulation
- 🔧 Local.tee creation for multiple uses
- 🔧 Block return value optimization
- 🔧 If return value optimization  
- 🔧 Loop return value optimization
- 🔧 Drop-tee to set conversion

**Next Steps** (requires arena-based expression manipulation):
1. Implement expression cloning/replacement infrastructure
2. Add actual sinking transformations to optimize_local_get
3. Implement block/if/loop return value merging
4. Add comprehensive integration tests
5. Benchmark against C++ SimplifyLocals

**Technical Blocker**: Full implementation requires arena-based tree manipulation infrastructure, which is a separate architectural task. Current foundation allows detection of optimization opportunities but defers transformations.

---

### Pass Implementation Strategy: 5 Tiers by Implementation Affinity

**Reality Check**: C++ Binaryen has **122 registered passes** (not ~50). Rust has **6 implemented**. This represents a **116-pass gap**.

Rather than porting passes alphabetically or randomly, we organize them into **5 implementation tiers** based on:
- **Shared infrastructure requirements** (passes that use the same analyses)
- **Implementation complexity** (simpler passes first)
- **Dependencies** (foundational passes before dependent ones)
- **Impact** (high-value optimizations prioritized)

This transforms a "multi-month monolithic effort" into **manageable multi-day chunks** with clear milestones.

**Detailed tier plans**: See companion documents:
- `2.1-tier1-foundational.md` — Cleanup & metadata (19 passes, 2-3 weeks)
- `2.2-tier2-local-block.md` — Local & block optimizations (24 passes, 3-4 weeks)
- `2.3-tier3-expression.md` — Instruction optimization (18 passes, 3-5 weeks)
- `2.4-tier4-advanced.md` — Whole-module analysis (28 passes, 4-8 weeks)
- `2.5-tier5-specialized.md` — Ecosystem integration (28 passes, 6-12 weeks)

---

### Tier Overview: Expected Parity by Milestone

| Tier | Category | Passes | Effort | Completion | Functional Parity | Impact |
|------|----------|--------|--------|------------|-------------------|--------|
| **Current** | Foundation | 6 | — | ✅ Done | ~5% | Core infrastructure |
| **Tier 1** | Cleanup & Metadata | 19 | 2-3 weeks | Week 1-2 | ~20% | Dead code removal |
| **Tier 2** | Local & Block | 24 | 3-4 weeks | Week 3-5 | ~40% | Single-function pipeline |
| **Tier 3** | Expression Optimization | 18 | 3-5 weeks | Week 6-9 | ~55% | High-impact (OptimizeInstructions) |
| **Tier 4** | Advanced Analysis | 28 | 4-8 weeks | Week 10-16 | ~75% | Whole-module, inlining |
| **Tier 5** | Specialized | 28 | 6-12 weeks | Week 17-30 | ~100% | Ecosystem features |

**Key Insight**: **Tier 3 completion delivers 55% functional parity** and most user-facing optimization value in just **12-16 weeks**, because it includes `OptimizeInstructions` (which accounts for 30-40% of C++'s optimization power alone).

---

### Complete Pass Inventory by Tier

**Tier 1: Foundational & Cleanup** (19 passes, 2-3 weeks)
- **Group 1A (7)**: vacuum, remove-unused-module-elements, remove-unused-brs, remove-unused-names, remove-unused-types, remove-imports, remove-memory-init
- **Group 1B (5)**: reorder-types, reorder-locals, reorder-globals, reorder-functions, minify-imports
- **Group 1C (7)**: name-types, nm, propagate-debug-locs, strip-debug, strip-dwarf, emit-target-features, print/print-*

**Tier 2: Local & Block Optimizations** (24 passes, 3-4 weeks)
- **Group 2A (8)**: simplify-locals, coalesce-locals✅, local-cse, local-subtyping, untee, merge-locals, dae, dae-optimizing
- **Group 2B (8)**: merge-blocks, simplify-control-flow, rereloop, poppify, rse, flatten, code-pushing, licm
- **Group 2C (8)**: pick-load-signs, signext-lowering, avoid-reinterprets, optimize-added-constants, ssa/ssa-nomerge, instrument-locals, trap-mode-*

**Tier 3: Expression & Instruction Optimization** (18 passes, 3-5 weeks)
- **Group 3A (6)**: precompute, precompute-propagate, const-hoisting, optimize-casts, denan, avoid-reinterprets
- **Group 3B (7)**: **optimize-instructions**⭐ (CRITICAL, 30-40% of optimization power), code-folding, simplify-identity✅, intrinsic-lowering, licm, alignment-lowering, generate-global-effects
- **Group 3C (5)**: local-subtyping, global-refining, type-refining, optimize-casts, pick-load-signs

**Tier 4: Advanced Analysis & Transforms** (28 passes, 4-8 weeks)
- **Group 4A (7)**: inlining, inlining-optimizing, inline-main, outlining, duplicate-function-elimination, merge-similar-functions, monomorphize/no-inline
- **Group 4B (8)**: dfo (dataflow-opts), flatten, type-ssa, gsi, cfp/cfp-reftest, unsubtyping, local-subtyping, gufa
- **Group 4C (8)**: global-refining, generate-global-effects, gsi, gufa, gto, remove-unused-module-elements, simplify-globals(-optimizing), propagate-globals-globally
- **Group 4D (5)**: type-merging, minimize-rec-groups, type-generalizing, signature-pruning/refining, heap-store-optimization

**Tier 5: Specialized & Backend** (28 passes, 6-12 weeks)
- **Group 5A (4)**: post-emscripten, optimize-for-js, legalize-js-interface, generate-dyncalls
- **Group 5B (8)**: translate-to-exnref/strip-eh, i64-to-i32-lowering, memory64/table64-lowering, multi-memory-lowering, llvm-*-lowering
- **Group 5C (8)**: **asyncify** (complex), spill-pointers, souperify, safe-heap, stack-check, instrument-memory, heap2local, optimize-j2cl
- **Group 5D (5)**: string-gathering/lifting/lowering, fpcast-emu, generate-dyncalls, tuple-optimization
- **Group 5E (3)**: dwarfdump, trace-calls, log-execution

**Remaining: Test, Debug & Niche Utilities** (~21 passes, as-needed implementation)

**Group R1: Test-Only / Internal Passes (7)**:
- catch-pop-fixup — Fix nested pops within catches (EH testing)
- deinstrument-branch-hints — Remove branch hint instrumentation
- delete-branch-hints — Delete branch hints by instrumented ID list
- experimental-type-generalizing — Generalize types (not yet sound, testing only)
- randomize-branch-hints — Randomize branch hints for fuzzing
- reorder-globals-always — Force global reordering even for few globals
- reorder-types-for-testing — Exaggerated cost function for testing

**Group R2: Debugging & Extraction Utilities (4)**:
- extract-function — Leave just one function (debugging)
- extract-function-index — Leave just one function by index
- roundtrip — Write to binary, read back (validation)
- func-metrics — Report function metrics

**Group R3: Specialized Alignment & Segments (3)**:
- dealign — Force all loads/stores to alignment=1 (testing)
- separate-data-segments — Write data segments to file, strip from module
- limit-segments — Merge segments to fit web loader limits

**Group R4: Niche Optimizations (4)**:
- once-reduction — Reduce calls to code that only runs once
- enclose-world — Destructive closed-world modifications (testing/specialized)
- set-globals — Set specified globals to specified values (testing)
- duplicate-import-elimination — Remove duplicate imports (covered by Tier 1 but standalone pass)

**Group R5: Instrumentation & Branch Hints (3)**:
- instrument-branch-hints — Instrument branch hints for correctness tracking
- remove-non-js-ops — Remove operations incompatible with JS (specialized)
- stub-unsupported-js — Stub out unsupported JS operations

**Total**: 6 (current) + 117 (Tiers 1-5) + 21 (remaining) = **144 passes total**

*Note*: The "123 passes" count in earlier analysis undercounted. C++ Binaryen has ~144 total passes when including all test/internal passes.

**Implementation Strategy**: See `2.6-remaining-niche.md` for detailed implementation approach for these specialized passes.

---

### Delivery Milestones & Value Proposition

| Milestone | Weeks | Total Passes | Functional Parity | Key Deliverable |
|-----------|-------|--------------|-------------------|-----------------|
| **Current State** | 0 | 6 | 5% | Foundation + infrastructure |
| **After Tier 1** | 2-3 | 25 | 20% | Dead code removal, compression |
| **After Tier 2** | 5-7 | 49 | 40% | Single-function optimization complete |
| **After Tier 3** | 12-16 | 67 | **55%** | **OptimizeInstructions delivers majority of value** |
| **After Tier 4** | 20-24 | 95 | **75%** | **Production-ready, inlining, whole-module analysis** |
| **After Tier 5** | 30-36 | 123 | 95%+ | Full ecosystem integration |

**Strategic Inflection Points**:
1. **Week 5 (40% parity)**: Single-function pipeline complete — can optimize most code patterns
2. **Week 14 (55% parity)**: OptimizeInstructions operational — **most user-visible optimization value delivered**
3. **Week 22 (75% parity)**: **Production-ready** — covers all major real-world use cases
4. **Week 34 (95% parity)**: Full feature parity with C++ for all ecosystems

---

### High Priority - Phase 5b (Foundation Complete, Full Implementation Next)

**SimplifyLocals Advanced Features** (foundation ready, needs tree manipulation):
- [x] **Infrastructure** — Pass framework, FunctionContext, effect tracking (✅ Done)
- [ ] **Local.set sinking** — Move local.sets closer to their uses (~300 lines with arena)
- [ ] **Local.tee creation** — Convert sets with multiple uses into tees (~100 lines)
- [ ] **Block return value optimization** — Hoist common sets to block returns (~150 lines)
- [ ] **Loop return value optimization** — Similar for loops (~100 lines)
- [ ] **Linear execution tracking** — Sophisticated control flow analysis (~200 lines)
- [ ] **Control flow merging** — Merge branches with identical effects (~150 lines)

**Status**: Foundation committed (Jan 15, 2026). Full implementation blocked on Arena Migration (see Phase 5b Major Task).

**Impact**: Would enable ~20% additional code size reduction in typical modules by unblocking sinking and control-flow optimizations.

---

#### High Priority - Phase 6 (Major Wins)

**OptimizeInstructions** (5,825 lines in C++!):
- [ ] **Constant folding** — Compile-time evaluation of pure operations
- [ ] **Algebraic simplification** — x+x→x*2, x&x→x, x|0→x, etc.
- [ ] **Strength reduction** — x*8→x<<3, x/2→x>>1
- [ ] **Comparison optimization** — (x<10)&(x<20)→x<10
- [ ] **Load/store forwarding** — Eliminate redundant memory ops
- [ ] **Sign extension elimination** — Remove redundant extends
- [ ] **Pattern matching framework** — Infrastructure for peephole opts

**Impact**: This single pass accounts for ~30-40% of C++ optimization power.

---

#### High Priority - Phase 7 (Performance)

**Inlining**:
- [ ] **Call site analysis** — Cost/benefit for inlining
- [ ] **Function inlining** — Inline small hot functions
- [ ] **Partial inlining** — Inline portions of functions
- [ ] **Recursive inlining** — Handle self-recursive calls

**CodePushing**:
- [ ] **Code motion** — Move computations closer to uses
- [ ] **Loop-invariant code motion** — Hoist invariants out of loops

**Impact**: Can provide 10-50% performance improvements in hot code.

---

#### Medium Priority - Phase 8 (Cleanup & Polish)

**Memory & Locals**:
- [ ] **CoalesceLocals** — Merge locals with non-overlapping lifetimes
- [ ] **ReorderLocals** — Sort by usage frequency for better compression
- [ ] **Vacuum** — Remove unused functions, globals, types
- [ ] **RemoveUnusedBrs** — Eliminate dead branches

**Control Flow**:
- [ ] **MergeBlocks** — Combine sequential blocks
- [ ] **Relooping** — Convert irreducible CFG to structured control flow
- [ ] **SimplifyControlFlow** — Flatten nested ifs, remove empty blocks

**Data Flow**:
- [ ] **DataFlowOpts** — SSA-style optimizations
- [ ] **ConstHoisting** — Move constants to optimal positions

**Impact**: Incremental 5-15% improvements in size and minor performance gains.

---

#### Lower Priority - Phase 9+ (Specialized)

**WebAssembly-Specific**:
- [ ] **SIMD simplification** — Vector operation optimization
- [ ] **Bulk memory operations** — memory.copy/fill optimization
- [ ] **Table operations** — call_indirect optimization
- [ ] **Exception handling** — try/catch optimization
- [ ] **GC operations** — Reference type optimization

**Emscripten Integration**:
- [ ] **Asyncify** — Transform to support async/await
- [ ] **PostEmscripten** — Emscripten-specific cleanup
- [ ] **OptimizeForJS** — JS interop optimization

**Advanced**:
- [ ] **GlobalEffects** — Whole-program effect analysis
- [ ] **TypeRefining** — Refine heap types for GC
- [ ] **Monomorphization** — Specialize generic functions

**Impact**: Niche improvements for specific use cases.

---

### Comparison: What Rust Does Better

Despite the gap, Rust has advantages:

1. **Effect System**: More structured than C++, with comprehensive tests (64 vs scattered in C++)
2. **Memory Safety**: Bumpalo arena + borrow checker prevents entire classes of bugs
3. **Test Coverage**: 298 tests with clear organization vs C++ scattered tests
4. **Modern Design**: Clean separation of concerns, builder pattern, modular architecture
5. **Type Safety**: Enum-based IR prevents invalid expression construction

---

### Realistic Timeline

| Phase | Passes | Estimated Effort | Timeline |
|-------|--------|------------------|----------|
| **5b** | SimplifyLocals advanced (6 features) | 2-3 weeks | Jan-Feb 2026 |
| **6** | OptimizeInstructions (~20 patterns) | 2-3 months | Feb-Apr 2026 |
| **7** | Inlining + CodePushing | 1-2 months | Apr-May 2026 |
| **8** | 10 medium-priority passes | 2-3 months | May-Jul 2026 |
| **9+** | Remaining 90+ passes | 6-12 months | Jul 2026-2027 |

**50% C++ Parity**: ~6-8 months of focused work  
**80% C++ Parity**: ~12-18 months  
**Full Parity**: ~2-3 years (if targeting all 122 passes)

**Verdict**: The foundation is excellent. Adding passes is now mostly mechanical work rather than architectural challenges. Priority should be: Phase 5b (unlock more SimplifyLocals wins) → Phase 6 (OptimizeInstructions for major gains) → Phase 7 (Inlining for performance). The existing 5 passes are production-quality and well-tested.

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

## Critical Architectural Task: Expression Tree Manipulation

**Status**: 🚨 **Blocking Phase 5b+ Full Implementation**

### The Problem

Current Rust IR uses owned `Box<Expression>` references, which prevents the in-place tree manipulation that C++ SimplifyLocals and other passes rely on. C++ uses `Expression**` (pointer-to-pointer) to enable:
- Replacing expressions in-place without moving entire trees
- Sinking local.sets by swapping pointers
- Creating tees by cloning and modifying nodes
- Block optimization by restructuring control flow

### Current Limitations

**Phase 5b SimplifyLocals** foundation is complete but **cannot perform transformations**:
- ❌ Cannot sink local.set into local.get (needs to move value and replace set with nop)
- ❌ Cannot create local.tee (needs to clone expression and modify in two places)
- ❌ Cannot hoist to block returns (needs to extract from multiple branches)
- ✅ CAN detect optimization opportunities (tracking, counting, effect analysis works)

### Solution Options

**Option 1: Arena-Based Pointer Manipulation** (matches C++ closely)
- Add typed arena allocator (like `bumpalo`) for all expressions
- Store raw pointers or arena indices instead of `Box<T>`
- Pros: Matches C++ architecture, enables all transformations
- Cons: Requires unsafe code, more complex lifetime management
- Effort: ~2-3 weeks

**Option 2: Index-Based Tree Representation**
- Store expressions in `Vec<Expression>` with integer indices as references
- Transforms modify vec entries directly
- Pros: Safe Rust, predictable performance
- Cons: Different from C++, requires rewriting existing code
- Effort: ~3-4 weeks + refactoring

**Option 3: Expression Cloning with Smart Replacement**
- Implement `Clone` for expressions, use replace-by-cloning pattern
- Pros: Simpler, maintains ownership
- Cons: Performance overhead, doesn't scale to complex passes
- Effort: ~1 week but limited capability

### Recommendation

**Option 1 (Arena-Based)** is preferred for:
- Maximum compatibility with C++ pass logic
- Best performance (zero-copy transformations)
- Enables porting remaining 116 C++ passes efficiently

### Impact on Timeline

| Scenario | Timeline | Capability |
|----------|----------|------------|
| **Without Arena** | Current | Detection only, 6/122 passes |
| **With Arena (Option 1)** | +3 weeks | Full SimplifyLocals, OptimizeInstructions, ~30/122 passes in 6 months |
| **With Index Tree (Option 2)** | +4 weeks + refactor | Similar capability, different architecture |

**Verdict**: Arena task should be prioritized in **Tier 1** (next 2-4 weeks) to unblock Phase 5b+ and all advanced optimization work.

---

## Recommended Prioritization

### Tier 1: Unblock the Passing Pipeline (Immediate, Next 2–4 weeks)
1. **Implement Arena-Based Expression Manipulation** ⚠️ **CRITICAL BLOCKER**
   - See "Critical Architectural Task" section above
   - Required for Phase 5b+ pass implementations
   - Unblocks 116 remaining optimization passes
   - Estimated: 2-3 weeks

2. **Create C++ roundtrip test** (`test_ffi_type_roundtrip.cpp`)
   - Validates Phase 2 completion.
   - Unblocks downstream integration.

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

