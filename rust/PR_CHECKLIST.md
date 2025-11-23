# Rust FFI PR Checklist

Before marking a PR ready for review (or merging), ensure the following items are complete for changes affecting Rust FFI exports or behavior:

<!-- cbindgen & internal-marker checks removed: header is internal-only by default -->
- [ ] Add or update a C++ cross-language smoke test in `test/rust_consumer` and ensure it is added to `test/rust_consumer/CMakeLists.txt`.
- [ ] Add/adjust unit tests in the Rust crate and any integration tests in `test/rust_consumer`.
[ ] If an ABI change is required and the change is intended to be part of a **public** API, bump `BINARYEN_FFI_ABI_VERSION` in `rust/binaryen-ffi/src/lib.rs` and document migration notes in the PR. Internal-only changes do not require an ABI bump.
- [ ] Add a short code example to `rust/binaryen-ffi/README.md` and update `docs/rust/vision` if behavior/lifetime semantics change.
- [ ] Add a maintainer/owner review by `CODEOWNERS` and `@mavity` if ABI changes are included.
- [ ] Run CI smoke tests locally: `cmake -S . -B build -DBUILD_RUST_COMPONENTS=ON` then `ctest -R rust_ffi_smoke -V`.
 - [ ] Add ASAN-enabled tests (if applicable) and run them locally: compile and run the sanitized consumer to ensure ASAN/UB detection flags potential UB.
	 - Example: `CC=clang CXX=clang++ cmake -S . -B build -GNinja -DBUILD_RUST_COMPONENTS=ON && cmake --build build && ASAN_OPTIONS=detect_leaks=0 LD_LIBRARY_PATH=$PWD/rust/target/release ./test/rust_consumer/test_rust_consumer_arena_deref_after_dispose_asan`
