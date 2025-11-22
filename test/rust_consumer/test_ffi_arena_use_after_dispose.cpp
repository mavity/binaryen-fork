#include <stdio.h>
#include <string.h>
#include "../../include/binaryen_ffi.h"

int main() {
    BinaryenArena* a = BinaryenArenaCreate();
    const char* p = BinaryenArenaAllocString(a, "alive");
    if (!p) { return 2; }
    if (BinaryenArenaIsAlive(a) != 1) {
        fprintf(stderr, "arena should be alive but is not\n");
        return 2;
    }
    BinaryenArenaDispose(a);
    // After dispose, arena should be considered not alive and allocations should return null
    if (BinaryenArenaIsAlive(a) != 0) {
        fprintf(stderr, "arena should not be alive after dispose\n");
        return 2;
    }
    const char* p2 = BinaryenArenaAllocString(a, "after-dispose");
    if (p2 != nullptr) {
        fprintf(stderr, "alloc should have returned null after dispose (got %p)\n", (void*)p2);
        return 2;
    }
    printf("arena use-after-dispose detection OK\n");
    return 0;
}
