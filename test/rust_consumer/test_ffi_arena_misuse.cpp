#include <stdio.h>
#include <string.h>
#include "../../include/binaryen_ffi.h"

int main() {
    BinaryenArena* a1 = BinaryenArenaCreate();
    const char* p1 = BinaryenArenaAllocString(a1, "arena-misuse");
    BinaryenArenaDispose(a1);

    BinaryenArena* a2 = BinaryenArenaCreate();
    const char* p2 = BinaryenArenaAllocString(a2, "arena-misuse");

    // We must not dereference p1 after dispose; but comparing pointer values is safe
    if (p1 == p2) {
        fprintf(stderr, "unexpected reuse of pointer values across arenas: %p == %p\n", (void*)p1, (void*)p2);
        BinaryenArenaDispose(a2);
        return 2;
    }

    BinaryenArenaDispose(a2);
    printf("arena misuse pointers differ OK (p1=%p p2=%p)\n", (void*)p1, (void*)p2);
    return 0; 
}
