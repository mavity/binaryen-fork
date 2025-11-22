# Rust FFI PR Checklist

Before marking a PR ready for review (or merging), ensure the following items are complete for changes affecting Rust FFI exports or behavior:

- [ ] Update `include/binaryen_ffi.h` (use `rust/scripts/update_cbindgen.sh` to regenerate) and run `rust/scripts/check_cbindgen.sh` locally.
- [ ] Add or update a C++ cross-language smoke test in `test/rust_consumer` and ensure it is added to `test/rust_consumer/CMakeLists.txt`.
- [ ] Add/adjust unit tests in the Rust crate and any integration tests in `test/rust_consumer`.
- [ ] If an ABI change is required, bump `BINARYEN_FFI_ABI_VERSION` in `rust/binaryen-ffi/src/lib.rs` and document the migration notes in the PR.
- [ ] Add a short code example to `rust/binaryen-ffi/README.md` and update `docs/rust/vision` if behavior/lifetime semantics change.
- [ ] Add a maintainer/owner review by `CODEOWNERS` and `@mavity` if ABI changes are included.
- [ ] Run CI smoke tests locally: `cmake -S . -B build -DBUILD_RUST_COMPONENTS=ON` then `ctest -R rust_ffi_smoke -V`.
