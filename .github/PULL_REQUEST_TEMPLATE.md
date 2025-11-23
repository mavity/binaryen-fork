# Pull Request Checklist

Please run the following checks locally when adding or changing code that affects the Rust FFI surface or Rust support crates. Include the output (or mention "ran successfully") in your PR description.

- [ ] Run the cross-language smoke tests (CMake consumer):
   - Ensure `BUILD_RUST_COMPONENTS=ON` and run the CMake smoke-runner locally (optional):
   - `mkdir -p build && cmake -S . -B build -DBUILD_RUST_COMPONENTS=ON && cmake --build build && ctest -R rust_ffi_smoke -V`
  - Also run the threaded smoke tests: `ctest -R rust_ffi_smoke_threaded -V`, `ctest -R rust_ffi_smoke_arena_threads -V`, and `ctest -R rust_ffi_smoke_arena_misuse -V`.
  - If you introduce or modify pointer semantics, run the sanitization tests locally (ASAN/UBSAN) and share the output in your PR. For example:
    - `CC=clang CXX=clang++ cmake -S . -B build -DBUILD_RUST_COMPONENTS=ON && cmake --build build && ASAN_OPTIONS=detect_leaks=0 LD_LIBRARY_PATH=$PWD/rust/target/release ./test/rust_consumer/test_ffi_arena_deref_after_dispose_asan`
- [ ] Run the Rust tests and clippy:
  - `cd rust && cargo test --all --all-features` and `cargo clippy --all --all-features -- -D warnings`

- [ ] If you updated exported FFI signatures and you intend to make them a stable public API, adjust the public ABI and obtain a maintainer sign-off. Internal-only changes do not require an ABI bump.

<!-- cbindgen and ABI checks for the internal `binaryen-ffi` header are optional; remove checklist items referencing them -->
Note: The `binaryen-ffi` header is INTERNAL to this repository and should not
be used by external projects. If your changes broaden the scope of these
exports, please get approval from the port maintainers before making the
header public or exporting functions for downstream projects.

If you're updating performance-sensitive code also consider running the nightly benchmarks and include the output if relevant:
