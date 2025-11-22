#!/usr/bin/env bash
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

# Build rust libs
(cd rust && cargo build --release)

# Build test_ffi linking the static rust lib
gcc test/rust_consumer/test_ffi.c -Lrust/target/release -lbinaryen_ffi -ldl -pthread -o test/rust_consumer/test_ffi

# Build threaded interner test and arena threaded test
g++ test/rust_consumer/test_ffi_threaded.cpp -Lrust/target/release -lbinaryen_ffi -ldl -pthread -o test/rust_consumer/test_ffi_threaded
g++ test/rust_consumer/test_ffi_arena_threads.cpp -Lrust/target/release -lbinaryen_ffi -ldl -pthread -o test/rust_consumer/test_ffi_arena_threads
g++ test/rust_consumer/test_ffi_arena_misuse.cpp -Lrust/target/release -lbinaryen_ffi -ldl -pthread -o test/rust_consumer/test_ffi_arena_misuse

# Run with dynamic loader search path so the runtime linker finds the Rust cdylib
LD_LIBRARY_PATH=$PWD/rust/target/release:${LD_LIBRARY_PATH:-} ./test/rust_consumer/test_ffi
LD_LIBRARY_PATH=$PWD/rust/target/release:${LD_LIBRARY_PATH:-} ./test/rust_consumer/test_ffi_threaded
LD_LIBRARY_PATH=$PWD/rust/target/release:${LD_LIBRARY_PATH:-} ./test/rust_consumer/test_ffi_arena_threads
LD_LIBRARY_PATH=$PWD/rust/target/release:${LD_LIBRARY_PATH:-} ./test/rust_consumer/test_ffi_arena_misuse
