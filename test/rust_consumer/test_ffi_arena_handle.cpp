#include <stdio.h>
#include <string.h>
#include "../../include/binaryen_ffi.h"

int main() {
    BinaryenArenaHandle* h = BinaryenArenaHandleCreate();
    if (!h) return 2;
    const char* p = BinaryenArenaHandleAllocString(h, "handle-test");
    if (!p) return 2;
    if (strcmp(p, "handle-test") != 0) return 2;
    if (!BinaryenArenaHandleIsAlive(h)) return 2;
    BinaryenArenaHandleDispose(h);
    (void)h; /* no-op to avoid unused warnings */
    (void)p; /* no-op */
    if (BinaryenArenaHandleIsAlive(h)) return 2;
    printf("arena handle test OK\n");
    return 0;
}
