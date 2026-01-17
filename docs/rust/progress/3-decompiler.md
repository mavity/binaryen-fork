# Binaryen decompiler

Implementing a decompiler foundation on top of this Rust port is a strategic move, as Binaryen’s IR already performs **Control Flow Recovery**—the hardest part of decompilation. Because the IR is structured (using blocks and loops) rather than a flat stack machine, your "decompiler" is essentially a high-level "printer" for this IR.

## I. Decompiler Foundation: Core Functional Blocks

To move from a partial port to a functional decompiler, focus on these three layers:

### 1. The "Lifting" Layer (High Priority)

This layer augments Binaryen's S-expression-like IR with semantic annotations.

* **Expression Reconstruction**: Tag Binaryen's tree nodes (e.g., `Binary`, `Unary`, `Load`) with high-level semantic hints.
* **Structured Control Flow**: Identify `Block` and `Loop` nodes that function as `if/else`, `while`, and `for` blocks. Since Binaryen IR already enforces structure, you don't need complex graph analysis.
* **Local Coalescing**: Implement a pass that merges multiple WebAssembly locals into a single logical "high-level" variable. Use Binaryen’s existing `CoalesceLocals` and `SimplifyLocals` passes as your baseline.

### 2. The Type Inference Engine (Medium Priority)

Wasm only has four basic types. A "functional" decompiler must infer more:

* **Pointer Recovery**: Identify `i32` values used in `Load` or `Store` operations and treat them as pointer types.
* **Struct Recovery**: Analyze contiguous memory offsets (e.g., `base + 4`, `base + 8`) to "re-invent" struct definitions like `struct { int a; int b; }`.

### 3. The Backend Printer (Low Priority)

This is the final stage that takes the annotated IR and emits text.

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
| **`StructRecovery`** | Semantic | Detect sequences of `i32.load` with increasing offsets from the same base pointer. Tag them for lifting into a logical `struct` access during printing. |
| **`VariableNamingHeuristics`** | Fluency | Use heuristics to rename `local_0` based on its usage. (e.g., if used in `i32.add` with `1` in a loop, name it `i` or `counter`). |
| **`BooleanRecovery`** | Type Lifting | Find `i32` values that are only ever compared to `0` or `1` and cast them to `bool` in the output code. |

### 2. Implementation: The "Dual-Mode" Library

To ensure this is both a library and a tool, structure your Rust crates to separate the **IR Representation** from the **Decompilation Logic**.

* **`binaryen-core` (Crate 1)**: Your current port (IR, Binary Parser).
* **`binaryen-decompiler` (Crate 2)**:
* **The Annotation Store**: A side-table mechanism to attach semantic metadata (like `ForLoop` or `BoolType`) to existing IR nodes.
* **The Transpiler**: A set of passes that enrich `binaryen-core::Module` with semantic annotations.
* **The Printer**: Logic to turn the annotated `Module` into Rust or C strings.



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


## III. Advanced Semantic Enrichment: The "Human-Fluency" Layer

Based on our intent to elevate the decompiler from a structural "printer" to a semantically "fluent" tool, this section focuses on leveraging the **Binaryen Pass Runner** for pattern-matching and heuristic enrichment.

While the foundation ensures the decompiled code is **structurally correct**, this layer ensures it is **human-readable**. By implementing decompiler-specific passes, we can transform low-level Wasm idioms into high-level language constructs (like `for` loops or `struct` access) that are lost during compilation.

### 1. Benefits of Enrichment Passes

* **Noise Reduction**: Standard Wasm often includes compiler-generated "stack noise" (e.g., unnecessary local sets/gets). Using enrichment passes like `SimplifyLocals` and `Vacuum` as pre-processors strips this away before the user ever sees it.
* **Contextual Recovery**: Unlike a general-purpose disassembler, a pass-based decompiler can use heuristics to "guess" intent. For example, an `i32` that is only ever compared to `0` or `1` can be "lifted" into a `bool` type in the final output.
* **Infrastructure Synergy**: Since this port uses the same **Pass Runner** as the optimizer, these decompiler passes are "first-class citizens." They can be scheduled, validated, and tested using the same infrastructure you are building for `binaryen-ir`.

### 2. Implementation Plan: The Enrichment Pipeline

To implement these passes, follow this three-stage "Decompile-specific" pipeline:

#### Stage A: Normalization (Standard Passes)

Before applying heuristics, run the core Binaryen passes you have already ported:

* **`SimplifyLocals`**: Collapses temporary stack variables.
* **`CoalesceLocals`**: Merges different locals that have non-overlapping lifetimes into a single high-level variable.
* **`Vacuum`**: Removes instructions with no side effects.

#### Stage B: Pattern Recognition (Semantic Passes)

Implement a **Pattern Matching Engine** (potentially using a Rust DSL or macros) to identify specific code "shapes":

* **`IdentifyIdiomaticLoops`**: Look for `loop` nodes with a counter increment at the end and a conditional break at the top. Tag these as `ForLoop` metadata for the printer.
* **`StructRecovery`**: Identify base pointers with fixed offsets (e.g., `base + 4`, `base + 8`) and lift them into a logical `struct` access rather than raw memory loads.

#### Stage C: Fluency Injection (Heuristic Passes)

Apply high-level heuristics to "humanize" the code:

* **`VariableNamingHeuristics`**: Rename variables based on usage (e.g., rename `local_1` to `i` if it's used as a loop index, or `ptr` if used in a load).
* **`OriginTracking`**: Use advanced data-flow analysis (via crates like `petgraph`) to track if a value originated from a specific system call (e.g., naming a return value `bytes_read` if it comes from `fd_read`).

### 3. Strategy for Adding Passes

1. **Tagging, Not Re-writing**: Instead of changing the IR structure, use the **Metadata Tagging** strategy. A pass should find a `Loop` node and simply attach a `metadata: LoopType::For` tag to it.
2. **Visitor Integration**: Update the `DecompileWriter` (the visitor) to check for these tags. If a `ForLoop` tag exists, print `for (i=0; i < n; i++)`; otherwise, fall back to the standard `loop { ... }`.
3. **Cross-Compiler Fingerprinting**: Create specific passes that recognize "code shapes" unique to `rustc` vs. `emscripten` to allow the decompiler to switch "dialects" within the same merged Wasm module.