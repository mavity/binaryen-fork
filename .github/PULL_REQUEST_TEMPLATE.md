# Pull Request Checklist

Please run the following checks locally when adding or changing code that affects the Rust FFI surface or Rust support crates. Include the output (or mention "ran successfully") in your PR description.

- [ ] Run the cbindgen golden header check:
  - `bash rust/scripts/check_cbindgen.sh`
- [ ] Run the ABI runtime/version check:
  - `bash rust/scripts/check_abi_changes.sh`
 - [ ] Run the cross-language smoke tests (CMake consumer):
   - Ensure `BUILD_RUST_COMPONENTS=ON` and run the CMake smoke-runner locally (optional):
   - `mkdir -p build && cmake -S . -B build -DBUILD_RUST_COMPONENTS=ON && cmake --build build && ctest -R rust_ffi_smoke -V`
  - Also run the threaded smoke tests: `ctest -R rust_ffi_smoke_threaded -V`, `ctest -R rust_ffi_smoke_arena_threads -V`, and `ctest -R rust_ffi_smoke_arena_misuse -V`.
- [ ] Run the Rust tests and clippy:
  - `cd rust && cargo test --all --all-features` and `cargo clippy --all --all-features -- -D warnings`
- [ ] If you updated public FFI signatures, bump `BINARYEN_FFI_ABI_VERSION` in `include/binaryen_ffi.h` and `rust/binaryen-ffi/src/lib.rs`, and add a changelog entry indicating the ABI change.

If you're updating performance-sensitive code also consider running the nightly benchmarks and include the output if relevant:
- `bash rust/scripts/run_bench.sh` (optional; requires local setup)
