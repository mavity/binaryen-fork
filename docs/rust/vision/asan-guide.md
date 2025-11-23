# ASAN / UBSAN Guide for Rust + C++ cross-language tests

This guide documents how to run AddressSanitizer (ASAN) or UndefinedBehaviorSanitizer (UBSAN) for the C++ consumer tests that exercise Rust FFI. These tools help detect memory-safety issues like use-after-free, races, or invalid reads/writes.

# ASAN / UBSAN Guide for Rust + C++ cross-language tests

This guide documents how to run AddressSanitizer (ASAN) or UndefinedBehaviorSanitizer (UBSAN) for the C++ consumer tests that exercise Rust FFI. These tools help detect memory-safety issues like use-after-free, races, or invalid reads/writes.

When to use

Note: The sanitizer guidance below is meant for maintainers and internal tests
only. `binaryen-ffi` is an INTERNAL FFI used for port integration tests; do not
headline or promote these instructions as general-purpose external consumer
instructions.

How to run ASAN locally (recommended)
-------------------------------------
1. Build Rust cdylib (release) and place it into `rust/target/release`:
```
cd rust
cargo build --release
```
2. Build the C++ consumer tests with ASAN flags:
```
export CFLAGS="-fsanitize=address -fno-omit-frame-pointer -g"
export CXXFLAGS="$CFLAGS"
gcc test/rust_consumer/test_ffi.c -Lrust/target/release -l:libbinaryen_ffi.so -ldl -pthread -o test/rust_consumer/test_ffi
g++ test/rust_consumer/test_ffi_threaded.cpp $CXXFLAGS -Lrust/target/release -l:libbinaryen_ffi.so -ldl -pthread -o test/rust_consumer/test_ffi_threaded_asan
g++ test/rust_consumer/test_ffi_arena_deref_after_dispose.cpp $CXXFLAGS -Lrust/target/release -l:libbinaryen_ffi.so -ldl -pthread -o test/rust_consumer/test_ffi_arena_deref_after_dispose_asan
```
3. Run the ASAN-enabled tests (some tests are expected to fail if they intentionally trigger UB; those should only be run under sanitizer):
```
ASAN_OPTIONS=detect_leaks=0 LD_LIBRARY_PATH=$PWD/rust/target/release ./test/rust_consumer/test_ffi_threaded_asan
ASAN_OPTIONS=detect_leaks=0 LD_LIBRARY_PATH=$PWD/rust/target/release ./test/rust_consumer/test_ffi_arena_deref_after_dispose_asan || true
```

Notes
-----
- Building Rust code with ASAN instrumentation is possible via nightly `-Z sanitizer` flags, but that is optional. For many cross-language cases, ASAN on the C++ side is sufficient to detect misuse like deref-after-dispose.
- Some tests intentionally trigger UB and therefore should be run only in sanitized environments or guarded to avoid failing general CI runs.
- The CI job `asan-ubsan` runs a set of ASAN-enabled consumer tests and is intended to catch UB or memory-safety issues introduced by PRs.
