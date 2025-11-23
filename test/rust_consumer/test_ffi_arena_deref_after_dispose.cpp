#include <stdio.h>
#include <string.h>
#include <assert.h>
#include "../../include/binaryen_ffi.h"

int main(void) {
    // Create arena and allocate a string
    BinaryenArena* a = BinaryenArenaCreate();
    const char* p = BinaryenArenaAllocString(a, "deref-after-dispose");
    if (!p) { fprintf(stderr, "alloc failed\n"); return 2; }
    // Dispose arena and attempt to dereference the string pointer.
    // This is undefined in normal builds but will be detected by sanitizers.
    BinaryenArenaDispose(a);
    // after dispose the arena should no longer be alive
    if (BinaryenArenaIsAlive(a) != 0) {
        fprintf(stderr, "arena still alive after dispose\n");
        return 2;
    }
    // Intentionally dereference pointer (UB) - ASAN should flag if enabled.
    printf("deref-after-dispose read: %s\n", p);
    return 0;
}
