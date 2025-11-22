#include <stdio.h>
#include <string.h>
#include "../../include/binaryen_ffi.h"

// Compile-time ABI check — ensure the golden header ABI version matches the
// expected value for this repository state.
#ifndef BINARYEN_FFI_ABI_VERSION
#error "BINARYEN_FFI_ABI_VERSION not defined in include/binaryen_ffi.h"
#endif
#if BINARYEN_FFI_ABI_VERSION != 1
#error "BINARYEN_FFI_ABI_VERSION mismatch — check ABI changes and update header"
#endif

int main() {
    printf("binaryen version: %u\n", binaryen_ffi_version());
    const char* s = "hello";
    const char* out = binaryen_ffi_echo(s);
    printf("echo returned: %s\n", out ? out : "(null)");

    // Test the string interner via FFI
    BinaryenStringInterner* interner = BinaryenStringInternerCreate();
    const char* i1 = BinaryenStringInternerIntern(interner, "world");
    const char* i2 = BinaryenStringInternerIntern(interner, "world");
    printf("intern pointers equal: %d\n", i1 == i2);
    BinaryenStringInternerDispose(interner);

    // Test arena
    BinaryenArena* a = BinaryenArenaCreate();
    const char* a1 = BinaryenArenaAllocString(a, "arena-hello");
    const char* a2 = BinaryenArenaAllocString(a, "arena-hello");
    printf("arena intern pointers equal: %d\n", a1 == a2);
    BinaryenArenaDispose(a);
    
    // Test hash helper
    uint64_t hv = BinaryenAhashBytes((const uint8_t*)"hello", 5);
    uint64_t hv2 = BinaryenAhashBytes((const uint8_t*)"hello", 5);
    printf("ahash(hello) = %llu\n", (unsigned long long)hv);
    if (hv != hv2) {
        fprintf(stderr, "hash mismatch: %llu != %llu\n", (unsigned long long)hv, (unsigned long long)hv2);
        return 2;
    }

    // Test FastHashMap via FFI
    BinaryenFastHashMap* fm = BinaryenFastHashMapCreate();
    if (!fm) { fprintf(stderr, "fast map create failed\n"); return 2; }
    if (!BinaryenFastHashMapInsert(fm, "one", 42)) { fprintf(stderr, "map insert failed\n"); return 2; }
    if (!BinaryenFastHashMapInsert(fm, "two", 7)) { fprintf(stderr, "map insert failed\n"); return 2; }
    // len should be 2
    size_t len = BinaryenFastHashMapLen(fm);
    printf("fastmap len = %zu\n", len);
    uint64_t outv = 0;
    if (!BinaryenFastHashMapGet(fm, "one", &outv)) { fprintf(stderr, "map get failed\n"); return 2; }
    printf("fastmap[one] = %llu\n", (unsigned long long)outv);
    if (outv != 42) { fprintf(stderr, "unexpected value: %llu\n", (unsigned long long)outv); return 2; }
    BinaryenFastHashMapDispose(fm);
    
    // Runtime ABI check — ensure library ABI matches the header macro.
    if (binaryen_ffi_abi_version() != BINARYEN_FFI_ABI_VERSION) {
        fprintf(stderr, "ABI mismatch: runtime=%u header=%u\n", binaryen_ffi_abi_version(), BINARYEN_FFI_ABI_VERSION);
        return 1;
    }
    return 0;
}
