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
    
    // Runtime ABI check — ensure library ABI matches the header macro.
    if (binaryen_ffi_abi_version() != BINARYEN_FFI_ABI_VERSION) {
        fprintf(stderr, "ABI mismatch: runtime=%u header=%u\n", binaryen_ffi_abi_version(), BINARYEN_FFI_ABI_VERSION);
        return 1;
    }
    return 0;
}
