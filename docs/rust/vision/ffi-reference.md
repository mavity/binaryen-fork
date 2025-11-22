# Rust FFI Reference (binaryen-ffi)

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
