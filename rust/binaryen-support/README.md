# binaryen-support

Rust support crate for Binaryen utility libraries.

This crate implements:
- StringInterner (`StringInterner`) — a thread-safe string interner with Box leak semantics for now.
- Arena (`Arena`) — a `bumpalo` backed arena with `alloc_str` helper for C-compatible strings.
- Hash helpers (`ahash_bytes`) and `FastHashMap` type alias based on `ahash`.

Usage (Rust):

```rust
use binaryen_support::{StringInterner, Arena};

let interner = StringInterner::new();
let s = interner.intern("hello");
assert_eq!(s, "hello");

let arena = Arena::new();
let p = arena.alloc_str("arena hello");
assert!(!p.is_null());
```

FFI usage and tests are provided via `rust/binaryen-ffi` crate and `test/rust_consumer` C++ smoke tests. See `rust-binaryen-ffi/README.md` for the public FFI surface and usage.

Testing
-------

Run all support crate tests:

```bash
cd rust/binaryen-support && cargo test
```

Run benchmarks:

```bash
cd rust/binaryen-support
cargo bench
```

Fuzz targets exist and can be executed via `cargo fuzz` (nightly + `cargo install cargo-fuzz`), or run the helper script:

```bash
rust/scripts/run_cargo_fuzz.sh
```
