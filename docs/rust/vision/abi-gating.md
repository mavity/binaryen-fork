# ABI gating & FFI change policy

This document describes the policy for making changes to the Rust FFI and ABI (the C ABI in `include/binaryen_ffi.h`).

Summary
-------
- Any change that touches the public FFI (the `binaryen-ffi` crate) must:
  - Update `include/binaryen_ffi.h` via `rust/scripts/update_cbindgen.sh`.
  - Bump `BINARYEN_FFI_ABI_VERSION` in `rust/binaryen-ffi/src/lib.rs` (major change) if the change is not ABI-compatible.
  - Add `ctest` rust_ffi smoke tests or additional consumer tests in `test/rust_consumer` covering the change.
  - Notify `CODEOWNERS` for the Rust FFI paths for review and sign-off.

Checklist for PR authors
-----------------------
1. Run `rust/scripts/check_cbindgen.sh` locally and ensure the generated header matches `include/binaryen_ffi.h`.
2. Add/update unit tests and cross-language smoke tests for the new API/behavior.
3. If the change is ABI incompatible: bump `BINARYEN_FFI_ABI_VERSION` and provide accompanying migration notes in the PR.
4. If disagreeing about ABI compatibility, consult with FFI code owners (@mavity) and add a detailed justification.
5. Add a C++ smoke consumer test that demonstrates the expected usage.

Review process
--------------
- Reviewers should ensure:
  - New FFI functions are documented in crate README and `docs/rust/vision`.
  - ABI versioning is updated if necessary.
  - Tests include cross-language `ctest` execution and C++ compilation.
  - cbindgen header is updated and validated by CI.
