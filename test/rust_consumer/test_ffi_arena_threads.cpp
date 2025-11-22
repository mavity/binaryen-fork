#include <stdio.h>
#include <string>
#include <thread>
#include <mutex>
#include <assert.h>
#include <string.h>
#include "../../include/binaryen_ffi.h"

int main() {
    BinaryenArena* arena = BinaryenArenaCreate();
    const char* p1 = BinaryenArenaAllocString(arena, "arena-threaded");
    const char* child_p = nullptr;

    std::mutex mu;
    std::thread t([&]() {
        // Duplicate the behavior from C++ side: read pointer and verify string
        std::lock_guard<std::mutex> lock(mu);
        child_p = BinaryenArenaAllocString(arena, "arena-threaded");
        if (strcmp(child_p, p1) != 0) {
            fprintf(stderr, "arena string mismatch in thread: %s != %s\n", child_p, p1);
        }
    });

    t.join();

    if (strcmp(p1, "arena-threaded") != 0) {
        fprintf(stderr, "arena string mismatch: %s\n", p1);
        BinaryenArenaDispose(arena);
        return 2;
    }

    BinaryenArenaDispose(arena);
    printf("arena threaded usage OK\n");
    return 0;
}
