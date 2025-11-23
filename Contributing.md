# Contributing to WebAssembly

Interested in participating? Please follow
[the same contributing guidelines as the design repository][].

  [the same contributing guidelines as the design repository]: https://github.com/WebAssembly/design/blob/main/Contributing.md

Also, please be sure to read [the README.md](README.md) for this repository.

## Adding support for new instructions

Use this handy checklist to make sure your new instructions are fully supported:

 - [ ] Instruction class or opcode added to src/wasm.h
 - [ ] Instruction class added to src/wasm-builder.h
 - [ ] Instruction class added to src/wasm-traversal.h
 - [ ] Validation added to src/wasm/wasm-validator.cpp
 - [ ] Interpretation added to src/wasm-interpreter.h
 - [ ] Effects handled in src/ir/effects.h
 - [ ] Precomputing handled in src/passes/Precompute.cpp
 - [ ] Parsing added in scripts/gen-s-parser.py, src/parser/parsers.h, src/parser/contexts.h, src/wasm-ir-builder.h, and src/wasm/wasm-ir-builder.cpp
 - [ ] Printing added in src/passes/Print.cpp
 - [ ] Decoding added in src/wasm-binary.h and src/wasm/wasm-binary.cpp
 - [ ] Binary writing added in src/wasm-stack.h and src/wasm/wasm-stack.cpp
 - [ ] Support added in various classes inheriting OverriddenVisitor (and possibly other non-OverriddenVisitor classes as necessary)
 - [ ] Support added to src/tools/fuzzing.h
 - [ ] C API support added in src/binaryen-c.h and src/binaryen-c.cpp
 - [ ] JS API support added in src/js/binaryen.js-post.js
 - [ ] C API tested in test/example/c-api-kitchen-sink.c
 - [ ] JS API tested in test/binaryen.js/kitchen-sink.js
 - [ ] Tests added in test/spec
 - [ ] Tests added in test/lit

## Rust FFI and ABI changes

- If you're changing the Rust FFI (the `binaryen-ffi` crate) or any ABI-exposed symbols:
- Optionally run `rust/scripts/check_cbindgen.sh` and update `include/binaryen_ffi.h` if necessary with `rust/scripts/update_cbindgen.sh`. For internal-only changes these are primarily for maintainers' convenience and are not enforced by CI.
- Add Rust unit tests and appropriate C++ smoke tests in `test/rust_consumer` exercising the change.
- If the change is ABI-incompatible, bump `BINARYEN_FFI_ABI_VERSION` and add migration notes in the PR; ensure `CODEOWNERS` review.

IMPORTANT: The `binaryen-ffi` C header (`include/binaryen_ffi.h`) is an internal
ABI for the port effort and is not guaranteed to be stable. It is not intended
for external projects to depend upon. Do not document, advertise, or publish
`include/binaryen_ffi.h` as a supported public API without explicit approval
from maintainers and following the repository's ABI governance process.
