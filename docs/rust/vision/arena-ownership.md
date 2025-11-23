# Arena Ownership & Lifetime (FFI contract)

This document describes the ownership, lifetime, and thread-safety guarantees for `Arena` in `binaryen-support` (Rust) and the `BinaryenArena*` FFI functions in `binaryen-ffi`.

Note: The `binaryen-ffi` crate and the FFI types declared here are INTERNAL to
this port. They are not a public ABI and are not intended for use outside the
project. If you use or rely on these types externally, your code may break at
any time and is unsupported.

Overview
--------
- `Arena` is a bump allocator intended for short-lived allocations. It provides faster allocation for many short-lived string allocations.
- The `BinaryenArena*` FFI functions allow C++ clients to create/dispose arenas and allocate null-terminated strings that are valid for the lifetime of the arena.

Lifetime & Ownership Rules
--------------------------
- A pointer returned by `BinaryenArenaAllocString` is valid only while the originating `BinaryenArena` is alive.
- The caller must not attempt to free those pointers; the memory is owned by the Rust arena.
- After `BinaryenArenaDispose` the pointer becomes invalid and must not be dereferenced. The FFI will now return `NULL` for allocations on a disposed arena.
- To detect misuse, `BinaryenArenaIsAlive` checks whether an arena pointer is still alive.

Thread-safety Guarantees
------------------------
- `Arena` is thread-safe: allocations from multiple threads are supported concurrently. This is implemented by using a `Mutex` internally.
- `BinaryenArenaAllocString` may be called concurrently from multiple threads. However, pointers remain valid only while the owning arena remains alive.
 - `Arena` is thread-safe: allocations from multiple threads are supported concurrently (implemented using a `Mutex`).
 - `BinaryenArenaAllocString` may be called concurrently from multiple threads; however, pointers remain valid only while the owning arena remains alive.

Recommended FFI usage from C++
------------------------------
1. Create an arena with `BinaryenArenaCreate()` and hold onto the returned pointer band.
2. Use `BinaryenArenaAllocString` to allocate strings to pass back into C/C++ code; pointers are valid while the arena is alive.
3. To check pointer validity before dereferencing, call `BinaryenArenaIsAlive(arena)`. Avoid dereferencing pointers if the arena is not alive.
4. When done with the arena and all strings, call `BinaryenArenaDispose`, which frees the arena memory. Do not use the arena pointer afterwards.

Lifetime & cross-thread example (C++)
------------------------------------
```cpp
// Create an arena and share between threads
BinaryenArena* a = BinaryenArenaCreate();
std::thread t1([a]{
  const char* s = BinaryenArenaAllocString(a, "hello");
  // safe to use while arena alive
  printf("%s\n", s);
});
// Other thread
std::thread t2([a]{
  const char* s2 = BinaryenArenaAllocString(a, "world");
});
t1.join(); t2.join();
BinaryenArenaDispose(a);
```

Misuse Examples
----------------
- Dereferencing `const char*` returned by `BinaryenArenaAllocString` after `BinaryenArenaDispose` is undefined; use `BinaryenArenaIsAlive` to detect misuse.
- Passing an arena pointer to a thread that outlives the arena is unsafe.

Implementation Notes
--------------------
- `binaryen-ffi` maintains an internal global registry of currently-live arenas to detect use-after-free and prevent allocations on a disposed arena. Allocations will return NULL if a non-live arena is used.
- This registry introduces a small runtime check but provides important safety for cross-language code.

Review Checklist
----------------
- Ensure FFI changes do not remove ABI stability macros and _do not_ change `BINARYEN_FFI_ABI_VERSION` without proper ABI sign-off.
- Add tests that exercise concurrent allocation and misuse detection from C++.
- Document changes in crate README and in `docs/rust`.
# Arena ownership & lifetime rules

This document defines the ownership, lifetimes, and rules for FFI values returned from the `Arena` and the `StringInterner`.

Overview
--------
- `BinaryenArena`: Rust-owned `Arena` created via `BinaryenArenaCreate()` and disposed with `BinaryenArenaDispose()`.
  - Any pointers returned from `BinaryenArenaAllocString()` are valid for the lifetime of the `Arena` instance.
  - It is undefined behavior to use pointers returned by `BinaryenArenaAllocString()` after `BinaryenArenaDispose()` is called for that arena.
  - The `Arena` object itself is not thread-local: it may be shared across threads. The
    current implementation provides internal synchronization and is thread-safe; no
    additional synchronization is required to allocate concurrently.

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
  - `Arena` is thread-safe in the current implementation: allocations are protected by an internal `Mutex`.
   - `Arena` is thread-safe with the current implementation; allocations are protected by an internal mutex.

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
