# Phase 2 TypeStore Implementation

## Status: ✅ Core Implementation Complete

This implements the TypeStore infrastructure for Phase 2 of the Rust conversion.

## What Was Implemented

### 1. TypeStore Backend (`binaryen-core/src/type_store.rs`)
- Global thread-safe `RwLock<TypeStore>` singleton
- Signature interning with bidirectional HashMap
- Automatic ID generation starting at 0x1000 (above basic type range)
- Functions: `intern_signature()` and `lookup_signature()`

### 2. Type Handle Encoding (`binaryen-core/src/type.rs`)
- Added `SIGNATURE_FLAG` (bit 32) to distinguish interned signatures
- Methods: `is_signature()`, `signature_id()`, `from_signature_id()`
- Maintains `Copy` semantics with 64-bit handle

### 3. FFI Layer (`binaryen-ffi/src/type_ffi.rs`)
- `BinaryenTypeCreateSignature(params, results) -> Type`
- `BinaryenTypeGetParams(ty) -> Type`
- `BinaryenTypeGetResults(ty) -> Type`
- Basic type getters: `BinaryenTypeInt32()`, etc.
- Comprehensive FFI unit tests (4 tests)

### 4. Cross-Language Validation (`test/rust_consumer/test_ffi_type_roundtrip.c`)
- C test that verifies:
  - Signature creation and param/result retrieval
  - Interning (same inputs → same ID)
  - Different signatures get different IDs
  - Basic types return `none` for signature queries

## Testing

Run the new test:
```bash
cmake -S . -B build -DBUILD_RUST_COMPONENTS=ON
cmake --build build
ctest --test-dir build -R rust_ffi_smoke_type_roundtrip -V
```

Run all Rust tests:
```bash
cd rust && cargo test
```

## Next Steps (from 1.3-ir.md plan)

- [ ] Add `Display` for interned types (query TypeStore during formatting)
- [ ] Add property tests with `proptest` for concurrent interning
- [ ] Expand TypeStore to support HeapType struct/array definitions
- [ ] Update golden header with `rust/scripts/check_cbindgen.sh`
- [ ] Add ASAN cross-language tests

## Architecture Notes

**Signature Representation**: Current `Signature` has `params: Type, results: Type`. This works for:
- Single param/result functions: `Signature { params: Type::I32, results: Type::F64 }`
- Multi-param functions require tuple types: `params = Type::Tuple([i32, f64])`

Tuple type interning is deferred to later phases when needed.

**HeapType Interning**: Basic HeapType constants (FUNC, ANY, etc.) remain hardcoded. Complex struct/array layouts will be interned in Phase 3 when IR construction needs them.
