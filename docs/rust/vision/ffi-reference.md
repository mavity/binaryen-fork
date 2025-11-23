# Rust FFI Reference (binaryen-ffi)

IMPORTANT: This header and FFI are INTERNAL to the Rust port and are not
intended for use by external consumers. The `include/binaryen_ffi.h` header is
maintained for integration tests and the rust port. External projects should
not include or depend on this header; do not ship it as part of external
releases.

Short reference for the FFI surface provided by `binaryen-ffi` (see `include/binaryen_ffi.h` for the canonical C header).

String Interner
----------------
- `BinaryenStringInternerCreate()` -> `BinaryenStringInterner*` (create an interner)
- `BinaryenStringInternerDispose(p)` -> dispose interner (`p` may be `NULL`)
- `BinaryenStringInternerIntern(p, s)` -> `const char*` interned string pointer; remains valid for process lifetime (current implementation leaks)

Arena
-----
- `BinaryenArenaCreate()` -> `BinaryenArena*` (create an arena) — returns non-null on success
- `BinaryenArenaDispose(p)` -> dispose arena; after dispose, `p` is invalid and any returned pointers must not be used
- `BinaryenArenaAllocString(p, s)` -> `const char*` valid until `p` is disposed
- `BinaryenArenaIsAlive(p)` -> return `1` if arena is live, `0` otherwise; handy for debugging and conditional deref safety tests

Handles (alternative API)
-------------------------
- `BinaryenArenaHandleCreate()` -> `BinaryenArenaHandle*` (create an arena handle)
- `BinaryenArenaHandleDispose(h)` -> dispose handle; `h` is invalid afterwards
- `BinaryenArenaHandleAllocString(h, s)` -> returns `const char*` valid while the handle is alive
- `BinaryenArenaHandleIsAlive(h)` -> return `1` if handle is live, `0` otherwise

Notes on handle usage
---------------------
- The handle-based API provides an indirection for safely using arena objects across language boundaries; handles are small opaque pointer wrappers to ensure safer cleanup and detection of stale handles.
- Handle-based `AllocString` will return `NULL` if the handle is not found or not alive.

Sanitizers & Safety Checks
--------------------------
- For pointer-safety changes, run ASAN checks described in `docs/rust/vision/asan-guide.md` to detect use-after-free errors and race conditions.
- Use `BinaryenArenaIsAlive` in consumer code to verify pointer validity. For example, before dereferencing a pointer, check whether the arena is still alive.

FastHashMap (String -> u64)
--------------------------
- `BinaryenFastHashMapCreate()` -> create instance
- `BinaryenFastHashMapDispose(p)` -> dispose (no-op if `p==NULL`)
- `BinaryenFastHashMapInsert(p, key, value)` -> bool success
- `BinaryenFastHashMapGet(p, key, out_value)` -> bool found
- `BinaryenFastHashMapLen(p)` -> size_t length

Hash Functions
--------------
- `BinaryenAhashBytes(data, len)` -> compute a 64-bit ahash of the byte slice

Lifetime notes
--------------
- Do not free or mutate returned `const char*` pointers from Rust (`BinaryenArenaAllocString` or interned pointers) — they are owned by Rust.
- Calling `BinaryenArenaDispose` invalidates any pointers produced by that arena.

Examples
--------
See `rust/binaryen-ffi/README.md` and `test/rust_consumer/*` for short examples demonstrating typical usage and common pitfalls.
