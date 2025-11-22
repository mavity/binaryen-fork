#include <stdio.h>
#include <string.h>
#include <thread>
#include <vector>
#include <atomic>
#include "../../include/binaryen_ffi.h"

int main() {
    BinaryenArena* a = BinaryenArenaCreate();
    if (!a) { return 2; }
    const int kThreads = 8;
    std::vector<std::thread> threads;
    std::atomic<int> errors{0};
    for (int i = 0; i < kThreads; ++i) {
        threads.emplace_back([a, i, &errors]() {
            char buf[64];
            snprintf(buf, sizeof(buf), "arena-thread-%d", i);
            const char* s = BinaryenArenaAllocString(a, buf);
            if (!s) { errors++; return; }
            if (strcmp(s, buf) != 0) {
                fprintf(stderr, "thread %d got wrong string: %s vs %s\n", i, s, buf);
                errors++;
            }
        });
    }
    for (auto &t: threads) t.join();
    BinaryenArenaDispose(a);
    if (errors.load() != 0) return 2;
    printf("arena many-threads allocation OK\n");
    return 0;
}
