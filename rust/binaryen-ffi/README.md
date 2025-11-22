# binaryen-ffi

Rust crate that exports a C FFI for the `binaryen-support` utilities.

Public FFI surface (C header: `include/binaryen_ffi.h`)
- `BinaryenStringInternerCreate/Dispose/Intern`
- `BinaryenArenaCreate/Dispose/AllocString`
- `BinaryenAhashBytes`
- `BinaryenFastHashMap*` helpers

How to build and test (developer):
```bash
# Build all Rust crates and run cargo tests
cd rust
cargo test --all

# Run cbindgen and update golden header (developer task)
cd rust/binaryen-ffi
cargo install cbindgen || true
cbindgen --config cbindgen.toml --crate binaryen-ffi --output ../../include/binaryen_ffi.h

# To compare cbindgen output with golden header
rust/scripts/check_cbindgen.sh

# Run the C++ smoke consumer (requires BUILD_RUST_COMPONENTS=ON, cmake build)
mkdir -p build && cd build
cmake .. -DBUILD_RUST_COMPONENTS=ON
cmake --build .
ctest -R rust_ffi_smoke -V
```

CI notes
--------
- The `rust-ci` GitHub workflow runs `cargo test` + `cbindgen` golden header checks.
- If you change the public FFI surface, update `include/binaryen_ffi.h` and bump `BINARYEN_FFI_ABI_VERSION` in `rust/binaryen-ffi/src/lib.rs` and the golden header.
- Check `rust/scripts/check_cbindgen.sh` and `rust/scripts/update_cbindgen.sh` for helper scripts.

Ownership and memory model
--------------------------
- The `StringInterner` returns `const char*` pointers that are valid for the process lifetime (current implementation leaks backing Strings). This is considered stable for now but is subject to future change.
- The `Arena` returns pointers valid until `BinaryenArenaDispose` is called — callers must avoid using pointers after the corresponding arena is disposed.

For more detailed ownership rules, see `docs/rust/vision/arena-ownership.md`.

FFI reference (short)
---------------------
- `BinaryenArenaCreate()` -> returns `BinaryenArena*` (non-null on success).
- `BinaryenArenaIsAlive(a)` -> returns `1` if `a` is still alive, `0` otherwise.
- `BinaryenArenaAllocString(a, s)` -> returns a `const char*` valid until `BinaryenArenaDispose(a)`.
- `BinaryenArenaDispose(a)` -> dispose and free arena memory; using `a` afterwards is invalid.

Example (C++):
```cpp
BinaryenArena* a = BinaryenArenaCreate();
const char* s = BinaryenArenaAllocString(a, "foo");
printf("%s\n", s);
BinaryenArenaDispose(a);
if (BinaryenArenaIsAlive(a) == 0) {
	// pointer is invalid
}
```
