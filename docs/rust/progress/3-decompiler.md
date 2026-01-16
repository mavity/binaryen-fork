# Binaryen decompiler

Implementing a decompiler foundation on top of this Rust port is a strategic move, as Binaryen’s IR already performs **Control Flow Recovery**—the hardest part of decompilation. Because the IR is structured (using blocks and loops) rather than a flat stack machine, your "decompiler" is essentially a high-level "printer" for this IR.

## I. Decompiler Foundation: Core Functional Blocks

To move from a partial port to a functional decompiler, focus on these three layers:

### 1. The "Lifting" Layer (High Priority)

This layer translates Binaryen's S-expression-like IR into a C-style Abstract Syntax Tree (AST).

* **Expression Reconstruction**: Map Binaryen's tree nodes (e.g., `Binary`, `Unary`, `Load`) into a C-style expression format.
* **Structured Control Flow**: Convert `Block` and `Loop` nodes into `if/else`, `while`, and `for` blocks. Since Binaryen IR already enforces structure, you don't need complex graph analysis.
* **Local Coalescing**: Implement a pass that merges multiple WebAssembly locals into a single logical "high-level" variable. Use Binaryen’s existing `CoalesceLocals` and `SimplifyLocals` passes as your baseline.

### 2. The Type Inference Engine (Medium Priority)

Wasm only has four basic types. A "functional" decompiler must infer more:

* **Pointer Recovery**: Identify `i32` values used in `Load` or `Store` operations and treat them as pointer types.
* **Struct Recovery**: Analyze contiguous memory offsets (e.g., `base + 4`, `base + 8`) to "re-invent" struct definitions like `struct { int a; int b; }`.

### 3. The Backend Printer (Low Priority)

This is the final stage that takes your internal C-style AST and emits text.

* **Target Rust vs. C**: For Rust output, this printer must handle specific syntax like `mut` declarations and `loop` blocks instead of `while`.

---

## II. Implementation Plan: Architecture & Tooling

To make this both a library and a part of `wasm-opt`, you should follow the **"Library-First"** pattern common in the Rust ecosystem.

### 1. Crate Architecture

Divide your project into three distinct parts within your workspace:

* **`binaryen-ir` (Library)**: The core IR and decompiler logic.
* **`binaryen-decompile` (Library)**: A thin wrapper crate specifically for the decompilation API.
* **`wasm-opt-rust` (Binary)**: The CLI tool that imports the library.

### 2. Integration into `wasm-opt`

In your Rust version of `wasm-opt`, treat "Decompile" as just another **Pass** or an **Output Mode**:

* **As an Output Mode**: Add a flag like `--print-decompiled` to the CLI. When active, the tool runs its optimization passes and then calls your library's `decompile(module)` function instead of the standard binary writer.
* **As a Library**: Ensure your `decompile` function accepts a `Module` object (your Rust-ported IR) and returns a `String`. This allows other Rust projects to simply call `binaryen::decompiler::run(&my_module)`.

### 3. Strategic Priorities from Gap Analysis

Based on your current status, you should focus on:

* **Phase 2 & 3 Completion**: You cannot decompile what you cannot represent. Finish porting the IR nodes before starting the decompiler printer.
* **Pass Integration**: Reuse existing optimization passes (like `Vacuum` or `SimplifyLocals`) as "pre-processing" steps for the decompiler to ensure the output isn't cluttered with compiler-generated "junk".

# Additional opportunity

The core plan focused on **structural parity** (getting the code to exist) omitting the **semantic lifting** necessary for a decompiler to be actually useful.

Porting the **Binaryen Pass Runner** comes with a unique infrastructure advantage: implementing "Decompiler Passes" that look and feel exactly like optimization passes but serve the opposite goal—**increasing readability at the expense of "perfect" binary size.**

Below is an additional, high-leverage opportunities for the decompiler foundation, focusing on pattern extraction and semantic heuristics.

### 1. The "Semantic Lifting" Pass Pipeline

Instead of just printing the IR, you should implement a series of **Decompiler-Specific Passes** that run after standard optimizations.

| Pass Name | Goal | Pattern to Extract |
| --- | --- | --- |
| **`IdentifyIdiomaticLoops`** | Readability | Convert `loop` + `br_if` patterns back into `while` or `for` loops. Standard Binaryen often leaves these as raw branch-to-top structures. |
| **`StructRecovery`** | Semantic | Detect sequences of `i32.load` with increasing offsets from the same base pointer. Replace them with a single `struct` access in the decompiler's AST. |
| **`VariableNamingHeuristics`** | Fluency | Use heuristics to rename `local_0` based on its usage. (e.g., if used in `i32.add` with `1` in a loop, name it `i` or `counter`). |
| **`BooleanRecovery`** | Type Lifting | Find `i32` values that are only ever compared to `0` or `1` and cast them to `bool` in the output code. |

### 2. Implementation: The "Dual-Mode" Library

To ensure this is both a library and a tool, structure your Rust crates to separate the **IR Representation** from the **Decompilation Logic**.

* **`binaryen-core` (Crate 1)**: Your current port (IR, Binary Parser).
* **`binaryen-decompiler` (Crate 2)**:
* **The Decompiler AST**: A simpler, higher-level tree than Binaryen IR that supports "Sugar" (like `for` loops and `struct` definitions).
* **The Transpiler**: A set of passes that transform `binaryen-core::Module` into `DecompilerAST`.
* **The Printer**: Logic to turn the `DecompilerAST` into Rust or C strings.



### 3. Tactical Plan: Functional Blocks to Focus On

**Phase A: Pattern Matching Engine (High Priority)**
Don't write hardcoded logic for every pattern. Use a **Pattern Matching DSL** (or a macro-based approach in Rust) to identify common compiler-generated sequences.

* *Example*: A pattern like `(local.set $x (i32.add (local.get $x) (i32.const 1)))` should be immediately tagged as an `Increment` operation.

**Phase B: Integration with `wasm-opt**`
In your Rust port of `wasm-opt`, add a new `Command` called `Decompile`.

* It should accept the same flags as `wasm-opt` (allowing you to run `--simplify-locals` before decompiling).
* It should expose an API: `fn decompile(module: &Module, options: DecompileOptions) -> String`.

**Phase C: Heuristic Variable Recovery**
Since your port is in Rust, you can easily use crates like `petgraph` to perform more advanced Data Flow Analysis than the original C++ Binaryen does. Use this to:

* Track a value's "origin" (e.g., did this `i32` come from an `fd_read` call? If so, rename it to `bytes_read`).

### Why this works

By treating decompilation as a **Pass-based transformation**, you stay "Binaryen-idiomatic." You are just adding a few more passes at the end of the pipeline that target human-readability instead of machine-efficiency. This allows you to leverage all 48+ passes you are currently porting while adding your own "secret sauce" for the decompiler.