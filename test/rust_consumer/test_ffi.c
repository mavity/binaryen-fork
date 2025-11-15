#include <stdio.h>
#include <string.h>
#include "../../include/binaryen_ffi.h"

int main() {
    printf("binaryen version: %u\n", binaryen_ffi_version());
    const char* s = "hello";
    const char* out = binaryen_ffi_echo(s);
    printf("echo returned: %s\n", out ? out : "(null)");
    return 0;
}
