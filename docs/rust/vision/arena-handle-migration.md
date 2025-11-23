# Arena Handle Migration Guide

This guide explains how to adopt the new handle-based API for `Arena` and provides a migration plan from raw pointer-based APIs.

Note: The handle-based API and the old pointer-based FFI are INTERNAL to this repository's Rust port and are not intended for external consumption.

Overview
--------
- The new handle-based API exposes `BinaryenArenaHandle*` for C/C++ consumers.
- The handle acts as an opaque wrapper around an internal arena pointer and provides extra safety: we can detect stale handles and avoid accidental use-after-free.

Migration steps
---------------
1. For new code, prefer the handle-based API (`BinaryenArenaHandle*`) instead of raw pointers.
2. Existing code that already uses `BinaryenArena*` may continue to use it. Consider migrating by adding a shim in C++: call `BinaryenArenaHandleCreate` and use the handle-based API.
3. To migrate an existing pointer to a handle, re-create the arena and refactor that part of the code to use the handle API’s lifecycle.

Deprecation policy (optional)
---------------------------
- The pointer API is stable for now. If we want to deprecate pointer-based API in favor of the handle-based API, a future release should:
  - Add deprecation markers to headers and docs.
  - Provide a compatibility shim (like a helper that wraps `BinaryenArenaCreate` returning handle) for a migration period.
  - Bump `BINARYEN_FFI_ABI_VERSION` if the pointer-based API is removed.

Examples and tips
-----------------
```cpp
// New recommended: use handle-based API
BinaryenArenaHandle *h = BinaryenArenaHandleCreate();
const char *s = BinaryenArenaHandleAllocString(h, "hello");
BinaryenArenaHandleDispose(h);

// For older code: keep using pointer based API until migrate
BinaryenArena *a = BinaryenArenaCreate();
const char *s2 = BinaryenArenaAllocString(a, "hello-old");
BinaryenArenaDispose(a);
```
