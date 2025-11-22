# Pull Request Checklist

Please run the following checks locally when adding or changing code that affects the Rust FFI surface or Rust support crates. Include the output (or mention "ran successfully") in your PR description.

- [ ] Run the cbindgen golden header check:
  - `bash rust/scripts/check_cbindgen.sh`
- [ ] Run the ABI runtime/version check:
  - `bash rust/scripts/check_abi_changes.sh`
- [ ] Run the Rust tests and clippy:
  - `cd rust && cargo test --all --all-features` and `cargo clippy --all --all-features -- -D warnings`
- [ ] If you updated public FFI signatures, bump `BINARYEN_FFI_ABI_VERSION` in `include/binaryen_ffi.h` and `rust/binaryen-ffi/src/lib.rs`, and add a changelog entry indicating the ABI change.

If you're updating performance-sensitive code also consider running the nightly benchmarks and include the output if relevant:
- `bash rust/scripts/run_bench.sh` (optional; requires local setup)
