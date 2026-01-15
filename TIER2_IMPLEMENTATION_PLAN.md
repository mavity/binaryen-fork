# Tier 2 Implementation Plan

**Date**: January 15, 2026  
**Status**: In Progress  
**Target**: Implement 24 Tier 2 passes sequentially

---

## Critical Rules (FROM REQUIREMENTS)

1. **SEQUENTIAL IMPLEMENTATION**: Implement 1 pass at a time, or very small similar sets. NO half-implemented passes.
2. **BUILD MUST SUCCEED**: When a pass is implemented, the build MUST succeed fully.
3. **ALL TESTS MUST PASS**: No exceptions. "The test was failing before" is NOT accepted.

---

## Current State

### ✅ Already Implemented (6 passes):
1. coalesce-locals ✅
2. simplify-locals ✅ (foundation, needs tree manipulation for full)
3. simplify ✅
4. simplify-identity ✅
5. memory-optimization ✅
6. dce (dead-code-elimination) ✅

### 🔧 Infrastructure Available:
- ControlFlowGraph (basic)
- Liveness analysis
- Effect system (27 flags)
- Visitor pattern
- Pass framework

---

## Implementation Order

### Phase 1: Infrastructure Gaps (Days 1-3)
Before implementing more passes, we need:

**Day 1: DominanceTree**
- File: `rust/binaryen-ir/src/dataflow/dominance.rs`
- Features: idom calculation, dominance queries, LCA
- Tests: ≥5 unit tests
- ~200 lines

**Day 2: LocalGraph (Def-Use Chains)**
- File: `rust/binaryen-ir/src/dataflow/local_graph.rs`
- Features: definition tracking, use tracking, sinking safety checks
- Tests: ≥5 unit tests
- ~250 lines

**Day 3: Infrastructure Testing & Integration**
- Ensure all new infrastructure works with existing passes
- Run full test suite
- Document APIs

---

### Phase 2: Group 2A - Local Simplification (Days 4-14)

**Day 4: untee**
- File: `rust/binaryen-ir/src/passes/untee.rs`
- Purpose: Convert local.tees back to sets+gets
- C++ Reference: `src/passes/Untee.cpp`
- Tests: ≥3 unit tests
- ~150 lines

**Day 5-6: local-cse (Local Common Subexpression Elimination)**
- File: `rust/binaryen-ir/src/passes/local_cse.rs`
- Purpose: Eliminate redundant computations within basic blocks
- C++ Reference: `src/passes/LocalCSE.cpp`
- Tests: ≥5 unit tests (various expression patterns)
- ~300 lines

**Day 7-8: local-subtyping**
- File: `rust/binaryen-ir/src/passes/local_subtyping.rs`
- Purpose: Refine local types to more specific subtypes
- C++ Reference: `src/passes/LocalSubtyping.cpp`
- Tests: ≥3 unit tests
- ~250 lines

**Day 9: merge-locals**
- File: `rust/binaryen-ir/src/passes/merge_locals.rs`
- Purpose: Merge similar local patterns
- Tests: ≥3 unit tests
- ~200 lines

**Day 10-12: dae (Dead Argument Elimination)**
- File: `rust/binaryen-ir/src/passes/dae.rs`
- Purpose: Remove unused function parameters
- C++ Reference: `src/passes/DAE.cpp`
- Tests: ≥4 unit tests (caller/callee interaction)
- ~400 lines

**Day 13-14: dae-optimizing**
- File: `rust/binaryen-ir/src/passes/dae_optimizing.rs`
- Purpose: DAE + optimize callers after parameter removal
- C++ Reference: `src/passes/DAEOptimizing.cpp`
- Tests: ≥3 unit tests
- ~350 lines

---

### Phase 3: Group 2B - Block & Control Flow (Days 15-28)

**Day 15: Enhance ControlFlowGraph**
- Add loop detection
- Add backedge identification
- Add reducibility checks
- Tests: ≥5 unit tests

**Day 16-18: merge-blocks**
- File: `rust/binaryen-ir/src/passes/merge_blocks.rs`
- Purpose: Combine sequential blocks
- C++ Reference: `src/passes/MergeBlocks.cpp`
- Tests: ≥5 unit tests (many edge cases)
- ~300 lines

**Day 19-20: simplify-control-flow**
- File: `rust/binaryen-ir/src/passes/simplify_control_flow.rs`
- Purpose: Flatten nested structures, remove empty blocks
- Tests: ≥4 unit tests
- ~250 lines

**Day 21: rse (Redundant Set Elimination)**
- File: `rust/binaryen-ir/src/passes/rse.rs`
- Purpose: Remove redundant local.sets
- Tests: ≥3 unit tests
- ~200 lines

**Day 22: flatten**
- File: `rust/binaryen-ir/src/passes/flatten.rs`
- Purpose: Convert nested blocks to flat IR
- Tests: ≥3 unit tests
- ~200 lines

**Day 23-24: code-pushing**
- File: `rust/binaryen-ir/src/passes/code_pushing.rs`
- Purpose: Move code closer to uses
- Tests: ≥4 unit tests
- ~300 lines

**Day 25-26: licm (Loop-Invariant Code Motion)**
- File: `rust/binaryen-ir/src/passes/licm.rs`
- Purpose: Hoist invariants out of loops
- C++ Reference: `src/passes/LICM.cpp`
- Tests: ≥5 unit tests (various loop patterns)
- ~350 lines

**Day 27: poppify**
- File: `rust/binaryen-ir/src/passes/poppify.rs`
- Purpose: Optimize pop/drop patterns
- Tests: ≥3 unit tests
- ~150 lines

**Day 28: rereloop (COMPLEX - may extend)**
- File: `rust/binaryen-ir/src/passes/rereloop.rs`
- Purpose: Convert irreducible CFG to structured form
- C++ Reference: `src/passes/ReReloop.cpp` + `src/cfg/Relooper.cpp`
- Tests: ≥5 unit tests
- ~600 lines (may need Relooper algorithm study)
- **NOTE**: Most complex pass, might split into 2-3 days

---

### Phase 4: Group 2C - Index & Memory Locals (Days 29-35)

**Day 29: pick-load-signs**
- File: `rust/binaryen-ir/src/passes/pick_load_signs.rs`
- Purpose: Infer optimal signedness for loads
- C++ Reference: `src/passes/PickLoadSigns.cpp`
- Tests: ≥3 unit tests
- ~200 lines

**Day 30: signext-lowering**
- File: `rust/binaryen-ir/src/passes/signext_lowering.rs`
- Purpose: Lower sign-extension operations to MVP
- Tests: ≥3 unit tests
- ~150 lines

**Day 31: avoid-reinterprets**
- File: `rust/binaryen-ir/src/passes/avoid_reinterprets.rs`
- Purpose: Prevent unsafe type punning
- Tests: ≥3 unit tests
- ~150 lines

**Day 32: optimize-added-constants**
- File: `rust/binaryen-ir/src/passes/optimize_added_constants.rs`
- Purpose: Fold constant offsets into loads/stores
- C++ Reference: `src/passes/OptimizeAddedConstants.cpp`
- Tests: ≥4 unit tests
- ~250 lines

**Day 33: optimize-added-constants-propagate**
- File: `rust/binaryen-ir/src/passes/optimize_added_constants_propagate.rs`
- Purpose: Propagate constant additions through locals
- Tests: ≥3 unit tests
- ~200 lines

**Day 34: instrument-locals**
- File: `rust/binaryen-ir/src/passes/instrument_locals.rs`
- Purpose: Add instrumentation for debugging
- Tests: ≥3 unit tests
- ~150 lines

**Day 35: trap-mode passes**
- File: `rust/binaryen-ir/src/passes/trap_mode.rs`
- Purpose: trap-mode-clamp, trap-mode-js
- Tests: ≥3 unit tests
- ~200 lines

---

### Phase 5: SSA Implementation (Days 36-40)

**Day 36-38: ssa-nomerge**
- File: `rust/binaryen-ir/src/passes/ssa_nomerge.rs`
- Purpose: Convert to SSA without phi node merges
- Tests: ≥5 unit tests
- ~300 lines

**Day 39-40: ssa (full)**
- File: `rust/binaryen-ir/src/passes/ssa.rs`
- Purpose: Full SSA with phi nodes
- C++ Reference: `src/passes/SSAify.cpp`
- Tests: ≥5 unit tests
- ~450 lines

---

## Success Criteria

### After Each Pass:
- [ ] Build succeeds (`cargo build --release`)
- [ ] All existing tests pass (`cargo test`)
- [ ] New pass has ≥3 unit tests
- [ ] Documentation added to pass module
- [ ] Pass registered in `passes/mod.rs`

### After Each Phase:
- [ ] Integration test with multiple passes
- [ ] Run on real-world WASM modules
- [ ] Validate output with official validator
- [ ] Update this document with completion status

### Final (After Day 40):
- [ ] 24 new passes implemented (30 total)
- [ ] ~40% functional parity with C++ Binaryen
- [ ] All 300+ existing tests pass
- [ ] 150+ new tests added
- [ ] Documentation complete

---

## Timeline

**Start**: January 15, 2026  
**Estimated Completion**: March 10, 2026 (8 weeks)  

### Milestones:
- **Week 2 (Day 10)**: Group 2A complete, LocalGraph working
- **Week 4 (Day 28)**: Group 2B complete, CFG enhancements done
- **Week 6 (Day 35)**: Group 2C complete
- **Week 8 (Day 40)**: SSA complete, Tier 2 finished

---

## Notes

- **Arena Migration**: If needed during implementation, pause and complete arena task first
- **Rereloop**: Most complex pass, may require extended study time
- **SSA**: Critical for Tier 4, ensure thorough testing
- **Testing**: Each pass MUST be validated with real WASM before moving to next

---

## Progress Tracking

### Infrastructure (Phase 1): ✅ COMPLETE
- [x] Day 1: DominanceTree - **DONE** (5 tests passing)
- [x] Day 2: LocalGraph - **DONE** (1 placeholder test, infrastructure complete)
- [x] Day 3: Integration testing - **DONE** (317 total tests passing, build succeeds)

### Group 2A (Phase 2):
- [ ] Day 4: untee
- [ ] Day 5-6: local-cse
- [ ] Day 7-8: local-subtyping
- [ ] Day 9: merge-locals
- [ ] Day 10-12: dae
- [ ] Day 13-14: dae-optimizing

### Group 2B (Phase 3):
- [ ] Day 15: CFG enhancements
- [ ] Day 16-18: merge-blocks
- [ ] Day 19-20: simplify-control-flow
- [ ] Day 21: rse
- [ ] Day 22: flatten
- [ ] Day 23-24: code-pushing
- [ ] Day 25-26: licm
- [ ] Day 27: poppify
- [ ] Day 28: rereloop

### Group 2C (Phase 4):
- [ ] Day 29: pick-load-signs
- [ ] Day 30: signext-lowering
- [ ] Day 31: avoid-reinterprets
- [ ] Day 32: optimize-added-constants
- [ ] Day 33: optimize-added-constants-propagate
- [ ] Day 34: instrument-locals
- [ ] Day 35: trap-mode passes

### SSA (Phase 5):
- [ ] Day 36-38: ssa-nomerge
- [ ] Day 39-40: ssa

---

**Last Updated**: January 15, 2026
