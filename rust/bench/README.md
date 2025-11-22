# Benchmark baseline & comparator

This folder stores benchmark baselines used by the nightly bench comparator workflow. 

To update the baseline (for example when you intentionally change a hot path):
- Run benchmarks locally: `cd rust/binaryen-support && cargo bench`.
- Copy the benchmark output directory (e.g., `target/criterion`) into `rust/bench/baseline/<date-or-tag>/`.
- Commit the new baseline and update any relevant documentation.

The nightly bench workflow is `.github/workflows/rust-bench-nightly.yml` which will compare the current bench to a stored baseline.
