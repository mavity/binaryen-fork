# Arena Handle design: raw pointer vs stable handle

This short design note considers transitioning the current FFI interface which returns raw `const char*` pointers and raw `BinaryenArena*` pointers into a handle-based API.

Motivation
----------
- Raw pointers across FFI cause confusion about lifetimes and can lead to UB if dereferenced after owner drop.
- A handle-based API maps a small integer or opaque handle to internal resources and allows the FFI layer to manage lifetime checks safely.

Pros
----
- Easier to validate usage purely in the FFI layer (e.g. `BinaryenArenaHandleIsAlive()` returning boolean), strong safety.
- Avoid pointer reuse confusion: handle values can be sequential and detect stale handles easily by generation counters.
- Can add checks on other operations (e.g., `BinaryenArenaAllocString` checks handle validity internally).

Cons
----
- Additional boilerplate in the FFI: provide creation/disposal of handles and mapping to internal pointers.
- Slight runtime overhead for handle lookups.
- Major change to public FFI: will likely require `BINARYEN_FFI_ABI_VERSION` bump and PR gating.

Suggested incremental approach
-----------------------------
1. Implement handle-backed API as extra-optional functions first (e.g., `BinaryenArenaHandleCreate/Dispose`), keeping existing pointer-based API unchanged.
2. Run migration tests and introduce wrappers in C++ clients to convert to handle-based API.
3. If stable and verified, deprecate raw pointer API in subsequent releases with clear deprecation warnings.
