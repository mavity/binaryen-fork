#include <stdio.h>
#include <string>
#include <thread>
#include <assert.h>
#include "../../include/binaryen_ffi.h"

int main() {
    BinaryenStringInterner* inter = BinaryenStringInternerCreate();
    const char* p1 = BinaryenStringInternerIntern(inter, "threaded");
    const char* child_p = nullptr;

    std::thread t([&]() {
        child_p = BinaryenStringInternerIntern(inter, "threaded");
    });

    t.join();
    // Should be the same pointer (same interned string)
    if (p1 != child_p) {
        fprintf(stderr, "pointers differ: %p != %p\n", (void*)p1, (void*)child_p);
        BinaryenStringInternerDispose(inter);
        return 2;
    }
    BinaryenStringInternerDispose(inter);
    printf("threaded interner pointer equality OK\n");
    return 0;
}
