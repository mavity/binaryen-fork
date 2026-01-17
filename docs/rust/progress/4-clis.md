# Phase 4: CLI Tooling & Optimization Orchestration
**Status**: 📋 Planned | **Target**: Week 15-18

## 1. Vision & Architecture

Phase 4 moves from implementing individual passes to providing the final user-facing tools. We adopt a **Two-Tier Architecture** that centralizes optimization "knowledge" within the library while keeping the CLI binaries thin and focused on configuration mapping.

### Crate Structure
1. **`binaryen-ir` (Core + Orchestration)**: 
   - Contains IR, Passes, and I/O logic.
   - **New**: High-Level Pass Suites API (the "orchestrator").
   - Encapsulates the knowledge of standard optimization pipelines (levels, orders, and repetitions).
2. **`binaryen-tools` (Binaries)**:
   - A new crate for CLI binaries: `wasm-opt`, `wasm-as`, and `wasm-dis`.
   - Uses `clap` for high-fidelity CLI parsing.
   - Keeps CLI code "narrow and shallow", mapping CLI tokens to IR API calls.

---

## 2. The "Bundles" API in `binaryen-ir`

Knowledge of which passes make up an optimization level or a "cleanup" sequence is moved into the `PassRunner` within the IR library. This ensures the logic is reusable across CLI, JS/WASM bindings, and direct Rust library consumers.

### Optimization Options
```rust
// rust/binaryen-ir/src/pass.rs (expansion)

pub struct OptimizationOptions {
    pub optimize_level: u32, // 0, 1, 2, 3, 4
    pub shrink_level: u32,   // 0, 1, 2 (for -Os, -Oz)
    pub debug_info: bool,
    pub low_memory_unused: bool,
    // ... other global flags from C++ PassOptions
}

impl OptimizationOptions {
    pub fn o0() -> Self { Self { optimize_level: 0, shrink_level: 0, ..Default::default() } }
    pub fn o1() -> Self { Self { optimize_level: 1, shrink_level: 0, ..Default::default() } }
    pub fn o2() -> Self { Self { optimize_level: 2, shrink_level: 0, ..Default::default() } }
    pub fn o3() -> Self { Self { optimize_level: 3, shrink_level: 0, ..Default::default() } }
    pub fn o4() -> Self { Self { optimize_level: 4, shrink_level: 0, ..Default::default() } }
    pub fn os() -> Self { Self { optimize_level: 2, shrink_level: 1, ..Default::default() } }
    pub fn oz() -> Self { Self { optimize_level: 2, shrink_level: 2, ..Default::default() } }
}
```

### PassRunner Expansion
```rust
// rust/binaryen-ir/src/pass.rs (expansion)

impl PassRunner {
    /// The main entry point for -O1, -O2, etc. 
    /// Ported from C++ PassRunner::addDefaultOptimizationPasses.
    pub fn add_default_optimization_passes(&mut self, options: &OptimizationOptions) {
        if options.optimize_level == 0 {
            return; // -O0: no optimizations
        }

        // Global pre-passes
        self.add_global_pre_passes(options);
        
        // Function-level optimizations (the "meat")
        self.add_function_optimization_passes(options);
        
        // Global post-passes
        self.add_global_post_passes(options);
    }

    /// Bundle: Standard cleanup sequence (vacuum + name removal + local simplification)
    fn add_cleanup_passes(&mut self) {
        self.add(passes::vacuum::Vacuum);
        self.add(passes::remove_unused_names::RemoveUnusedNames);
        self.add(passes::simplify_locals::SimplifyLocals::new());
    }

    /// Bundle: Dead Code Elimination sequence
    fn add_dead_code_elimination_passes(&mut self) {
        self.add(passes::dce::DCE);
        self.add(passes::remove_unused_module_elements::RemoveUnusedModuleElements);
    }

    /// Bundle: Branch optimization (merge blocks + remove unused branches)
    fn add_branch_optimization_passes(&mut self) {
        self.add(passes::merge_blocks::MergeBlocks);
        self.add(passes::remove_unused_brs::RemoveUnusedBrs);
    }

    // ... (internal methods for global pre/post and function optimization logic)
}
```

**Key Design Principle**: The `PassRunner` now "knows" the standard sequences from the C++ codebase. This allows any consumer (CLI, WASM, or library) to request `-O3` optimization without needing to understand which 30+ passes are involved or their order.

---

## 3. CLI Design (`wasm-opt`)

The `wasm-opt` binary acts as a shallow wrapper, mapping arguments to `PassRunner` configuration.

### Order-Aware Argument Parsing
We use `clap` with `ArgAction::Append` and iterate through `matches.ids()` in the order they were encountered to ensure that the user-specified sequence of passes and optimization flags is preserved.

```rust
// rust/binaryen-tools/src/bin/wasm_opt.rs

use clap::{Arg, ArgAction, Command};
use binaryen_ir::{Module, PassRunner, OptimizationOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("wasm-opt")
        .version(env!("CARGO_PKG_VERSION"))
        .about("WebAssembly optimizer")
        .arg(Arg::new("input").required(true).help("Input file (.wasm or .wat)"))
        .arg(Arg::new("output").short('o').long("output").help("Output file"))
        .arg(Arg::new("O1").short('O').value_name("1").action(ArgAction::SetTrue))
        .arg(Arg::new("O2").short('O').value_name("2").action(ArgAction::SetTrue))
        .arg(Arg::new("O3").short('O').value_name("3").action(ArgAction::SetTrue))
        .arg(Arg::new("dce").long("dce").action(ArgAction::Append).help("Dead code elimination"))
        .arg(Arg::new("simplify-locals").long("simplify-locals").action(ArgAction::Append))
        // ... (all pass flags)
        .get_matches();

    let allocator = bumpalo::Bump::new();
    let input = matches.get_one::<String>("input").unwrap();
    let mut module = load_module(&allocator, input)?;
    
    let mut runner = PassRunner::new();
    
    // Maintain CLI order: wasm-opt --dce -O3 --dce
    for arg_id in matches.ids() {
        match arg_id.as_str() {
            "dce" => runner.add(binaryen_ir::passes::dce::DCE),
            "simplify-locals" => runner.add(binaryen_ir::passes::simplify_locals::SimplifyLocals::new()),
            "O1" => runner.add_default_optimization_passes(&OptimizationOptions::o1()),
            "O2" => runner.add_default_optimization_passes(&OptimizationOptions::o2()),
            "O3" => runner.add_default_optimization_passes(&OptimizationOptions::o3()),
            // ...
            _ => {}
        }
    }
    
    runner.run(&mut module);
    
    let output = matches.get_one::<String>("output");
    save_module(&module, output)?;
    Ok(())
}
```

### Shorthand Flags (`-O3`, `-Os`)
These map directly to `OptimizationOptions` presets which the `PassRunner` expands into the full sequence of passes from the C++ `addDefaultOptimizationPasses` logic.

---

## 4. `wasm-as` and `wasm-dis` Implementations

These tools are significantly simpler as they primarily bridge file I/O and the existing text/binary components in `binaryen-ir`.

### `wasm-as` (Assembler)
```rust
// rust/binaryen-tools/src/bin/wasm_as.rs

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("wasm-as")
        .arg(Arg::new("input").required(true))
        .arg(Arg::new("output").short('o').long("output"))
        .get_matches();
    
    let allocator = bumpalo::Bump::new();
    let input = std::fs::read_to_string(matches.get_one::<String>("input").unwrap())?;
    let module = Module::read_wat(&allocator, &input)?;
    
    let mut writer = BinaryWriter::new();
    let bytes = writer.write_module(&module)?;
    
    let output = matches.get_one::<String>("output").map(|s| s.as_str()).unwrap_or("output.wasm");
    std::fs::write(output, bytes)?;
    Ok(())
}
```

### `wasm-dis` (Disassembler)
```rust
// rust/binaryen-tools/src/bin/wasm_dis.rs

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("wasm-dis")
        .arg(Arg::new("input").required(true))
        .arg(Arg::new("output").short('o').long("output"))
        .get_matches();
    
    let allocator = bumpalo::Bump::new();
    let bytes = std::fs::read(matches.get_one::<String>("input").unwrap())?;
    let mut reader = BinaryReader::new(&allocator, bytes);
    let module = reader.parse_module()?;
    
    let wat = module.to_wat()?;
    
    let output = matches.get_one::<String>("output").map(|s| s.as_str()).unwrap_or("output.wat");
    std::fs::write(output, wat)?;
    Ok(())
}
```

---

## 5. Deliverables & Implementation Steps

| Step | Task | Description | Files |
|---|---|---|---|
| **4.1** | **OptimizationOptions API** | Define the configuration struct and preset constructors (`::o1()`, `::o2()`, etc.) | `rust/binaryen-ir/src/pass.rs` |
| **4.2** | **Pass Bundles** | Implement helper methods for common pass sequences (`add_cleanup_passes()`, `add_dead_code_elimination_passes()`, etc.) | `rust/binaryen-ir/src/pass.rs` |
| **4.3** | **Default Optimization Logic** | Port `PassRunner::addDefaultOptimizationPasses` from C++, implementing the full `-O1`/`-O2`/`-O3`/`-O4`/`-Os`/`-Oz` logic | `rust/binaryen-ir/src/pass.rs` |
| **4.4** | **Tooling Crate** | Create `rust/binaryen-tools` and configure `Cargo.toml` with `[[bin]]` targets for `wasm-opt`, `wasm-as`, and `wasm-dis` | `rust/binaryen-tools/Cargo.toml` |
| **4.5** | **wasm-as / wasm-dis** | Implement basic assemblers/disassemblers using existing IR I/O bridges | `rust/binaryen-tools/src/bin/wasm_{as,dis}.rs` |
| **4.6** | **wasm-opt Core** | Implement `clap` driver with order-preserving pass injection | `rust/binaryen-tools/src/bin/wasm_opt.rs` |
| **4.7** | **Advanced Flags** | Add support for feature flags (`--enable-simd`), validation settings, and pass-specific arguments | All tool binaries |

### Recommended Implementation Order
1. **Start with 4.1-4.3**: Build the high-level orchestration API in `binaryen-ir` first. This allows testing via unit tests and Rust API before any CLI is involved.
2. **Then 4.4-4.5**: Implement the simple tools (`wasm-as`, `wasm-dis`) to validate the I/O bridge and provide immediate utility.
3. **Finally 4.6-4.7**: Implement the full `wasm-opt` CLI with all flags and ordering semantics.

---

## 6. Parity Verification

### Automated Lit Tests
- Run the existing `test/lit` suite using the new Rust `wasm-opt`.
- Success criterion: Output is semantically equivalent to C++ output (verified via module validation or execution tests).

### Differential Fuzzing
- Compare results of C++ `wasm-opt` and Rust `wasm-opt` when processing random valid WASM modules.
- Any divergence in resulting module behavior indicates a regression in either pass logic or orchestration.

### CLI Parity Check
- Verify that `wasm-opt --help` contains the same categories, pass names, and descriptions as the C++ original.
- Use a diff tool to compare the help text line-by-line.

---

## 7. First Implementation Step (Step 4.1)

**Task**: Define `OptimizationOptions` struct and preset constructors in `rust/binaryen-ir/src/pass.rs`.

**Goal**: Establish the configuration API that will be used by both the CLI and library consumers to specify optimization levels.

**Deliverable**: 
- A `pub struct OptimizationOptions` with fields for `optimize_level`, `shrink_level`, and other global settings.
- Constructor methods: `::o0()`, `::o1()`, `::o2()`, `::o3()`, `::o4()`, `::os()`, `::oz()`.
- Unit tests verifying that the presets have correct field values.

**Estimated Effort**: 1-2 hours (straightforward struct definition and tests).

**Why This First?**:
1. It's the foundation for all subsequent work—both the "bundles" API and the CLI need this type.
2. It can be implemented and tested in isolation without requiring new dependencies or significant changes to existing code.
3. It's a small, atomic change that can be reviewed and merged quickly, establishing momentum for the phase.
