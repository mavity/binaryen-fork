Fuzzing
=======

This directory contains `libFuzzer` harnesses for the Rust support and FFI code.

Requirements
------------
- Install `cargo-fuzz` (https://github.com/rust-fuzz/cargo-fuzz):
  ```bash
  cargo install cargo-fuzz
  ```

Running fuzz targets
--------------------
From the repository root run:
```bash
cd fuzz
cargo fuzz run interner
```

To run multiple targets, run them one-by-one or add a small orchestrator script.

CI / gating notes
-----------------
This repository does not enable fuzzing in CI by default. If you want to add a CI job, use `runs-on: ubuntu-latest`, `cargo install cargo-fuzz`, and run each harness under `cargo fuzz run --jobs 1` in a time-bound manner.
