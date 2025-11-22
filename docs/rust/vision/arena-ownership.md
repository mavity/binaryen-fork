# Arena ownership & lifetime rules

This document defines the ownership, lifetimes, and rules for FFI values returned from the `Arena` and the `StringInterner`.

Overview
--------
- `BinaryenArena`: Rust-owned `Arena` created via `BinaryenArenaCreate()` and disposed with `BinaryenArenaDispose()`.
  - Any pointers returned from `BinaryenArenaAllocString()` are valid for the lifetime of the `Arena` instance.
  - It is undefined behavior to use pointers returned by `BinaryenArenaAllocString()` after `BinaryenArenaDispose()` is called for that arena.
  - The `Arena` object itself is not thread-local. If you share the `Arena` across threads, ensure you manage synchronization in the caller (the current implementation is not thread-safe).

- `StringInterner`: The current `StringInterner` implementation leaks `String`s to return `&'static str` references. As a result:
  - Pointers returned by `BinaryenStringInternerIntern` are valid for the process lifetime (they are intentionally leaked and not freed).
  - This is considered a stable behavior for now; if this changes, ABI & docs must be updated and `BINARYEN_FFI_ABI_VERSION` must be bumped.

Rules for cross-language usage
-----------------------------
- Always **tie the lifetime** of pointers to the owning resource:
  - For pointers returned by `BinaryenArenaAllocString`, keep the `BinaryenArena*` alive while any thread is using the pointer.
  - For `BinaryenStringInternerIntern` pointers, they can be reused across threads and owned by C/C++ callers safely for the process lifespan.

- Do not assume any pointer remains valid after disposing its owning handle.

- Threading rules:
  - `StringInterner` is safe to use from multiple threads on the current implementation (it uses `RwLock` internally); concurrent `intern` calls for the same string will return the same pointer.
  - `Arena` is NOT guaranteed thread-safe in the current implementation; callers that share an `Arena` across threads should protect operations with a mutex or ensure thread-safety in the caller.

Recommended reviewer checklist for FFI changes
----------------------------------------------
- If you add or change any `#[repr(C)]` type, ensure a reviewer understands the layout and impact.
- If exported functions are changed, verify `cbindgen` output and update `include/binaryen_ffi.h`.
- If ownership semantics change (for example, `Arena` pointers become stable until a global free or are copied), update `docs/rust/vision/arena-ownership.md` and bump `BINARYEN_FFI_ABI_VERSION`.
- Add cross-language tests under `test/rust_consumer` that exercise the new behavior (ownership, threading, pointer lifetime) and add appropriate CI coverage.

Example: safe use of pointer across thread
-----------------------------------------
```c++
// Pseudocode
BinaryenArena* a = BinaryenArenaCreate();
const char* p = BinaryenArenaAllocString(a, "hello");

std::thread t([p]() {
    printf("thread read: %s\n", p);
});

// Ensure arena remains alive until thread completes
// (do NOT call BinaryenArenaDispose until thread joined)

t.join();
BinaryenArenaDispose(a);
```

Example: unsafe use (do not do this)
-----------------------------------
```c++
BinaryenArena* a = BinaryenArenaCreate();
const char* p = BinaryenArenaAllocString(a, "hello");
BinaryenArenaDispose(a);
// p may be invalid now — do not deref.
```

This document should be kept up-to-date with any changes in the `Arena`/`StringInterner` implementation or the public FFI surface.
