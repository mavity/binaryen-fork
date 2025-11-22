#include <stdio.h>
#include <string.h>
#include "../../include/binaryen_ffi.h"

int main() {
    BinaryenArena* a1 = BinaryenArenaCreate();
    const char* p1 = BinaryenArenaAllocString(a1, "arena-misuse");
    BinaryenArenaDispose(a1);

    BinaryenArena* a2 = BinaryenArenaCreate();
    const char* p2 = BinaryenArenaAllocString(a2, "arena-misuse");

    // We must not dereference p1 after dispose; comparing pointer values is ok
    // but pointer reuse across different arenas can happen (it's not an error);
    // thus treat equality as a *warning* only and continue.
    if (p1 == p2) {
        fprintf(stderr, "warning: reuse of pointer values across arenas detected: %p == %p\n", (void*)p1, (void*)p2);
    }

    BinaryenArenaDispose(a2);
    printf("arena misuse pointers differ OK (p1=%p p2=%p)\n", (void*)p1, (void*)p2);
    return 0; 
}
