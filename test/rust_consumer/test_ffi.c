#include <stdio.h>
#include <string.h>
#include "../../include/binaryen_ffi.h"

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
    return 0;
}
