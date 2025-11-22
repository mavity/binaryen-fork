# Binaryen → Rust: Vision

This project aims to port Binaryen from C++ to Rust while preserving functionality, API compatibility, performance, and user workflows. The conversion is incremental and reversible — Rust and C++ components will coexist across FFI boundaries during the transition. The goal is to achieve a safer, more maintainable, and equally performant implementation that integrates into existing build systems and toolchains.

## Core outcomes
- Maintain 100% C API compatibility during conversion
- Keep existing tools and CLIs (wasm-opt, wasm-as, wasm-dis, wasm2js, etc.) working
- Match or improve performance and memory characteristics vs. C++ (target: ≤5% perf regression, ≤10% memory variance)
- Eliminate undefined behavior using Rust's safety guarantees and miri-based checks
- Preserve test coverage; each phase is validated with unit, integration, property, fuzz, and regression tests

## The scope
- Rust equivalents for Binaryen’s utility libraries (arena allocators, string interning, hash utilities)
- WebAssembly type system and core types
- Intermediate Representation (IR): expressions, modules, builders, and traversal utilities
- Binary and text format readers/writers and parsers
- Optimization passes and pass infrastructure
- Command-line tools and developer-facing binaries
- C and JavaScript bindings / API surface

## Phases
- Phase 0: Infrastructure — Rust workspace, Cargo + CMake integration, FFI patterns, CI
- Phase 1: Utilities — core data structures, arenas, string interning, literals
- Phase 2: Types — WebAssembly types, signatures, shapes
- Phase 3: IR Core — expressions, module, builder, traversal
- Phase 4: Binary Format — read/write and text format support
- Phase 5: Passes — convert optimization passes, start with simpler passes
- Phase 6: Tools — wasm-opt and other CLIs
- Phase 7: APIs — C API and JavaScript bindings
- Phase 8: Finalization — performance tuning, documentation, eventual C++ deprecation

![Timeline Overview](./rust/progress/team-diagram.png)

## Validation & success criteria:
- All tests (C++, Rust) pass after each phase
- Benchmarks show performance within target range
- No undefined behavior detected by miri or fuzzers
- C API is unchanged for existing consumers
- Documentation, CI, and developer ergonomics are preserved

How to get involved:
- Read the detailed plan and checklists in `docs/rust/vision/`
- Follow `docs/rust/vision/RUST_CONVERSION_PLAN.md` for phase details
- Pick a phase or component from the checklist and open a PR with tests and benchmarks

This document is an intentionally short vision and point of orientation; for implementation details, check the technical specs and the full conversion plan in `docs/rust/vision/`.

