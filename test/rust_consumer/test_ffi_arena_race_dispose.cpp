#include <stdio.h>
#include <string.h>
#include <thread>
#include <chrono>
#include <atomic>
#include "../../include/binaryen_ffi.h"

int main() {
    BinaryenArena* a = BinaryenArenaCreate();
    if (!a) return 2;
    std::atomic<bool> ready(false);
    std::atomic<int> errors(0);

    std::thread t1([&]() {
        while (!ready.load()) std::this_thread::sleep_for(std::chrono::milliseconds(1));
        for (int i = 0; i < 100; ++i) {
            const char* p = BinaryenArenaAllocString(a, "race-dispose");
            if (!p) { errors++; return; }
            if (strcmp(p, "race-dispose") != 0) { errors++; return; }
            std::this_thread::sleep_for(std::chrono::microseconds(50));
        }
    });

    std::thread t2([&]() {
        while (!ready.load()) std::this_thread::sleep_for(std::chrono::milliseconds(1));
        // randomly dispose/recreate arena a few times while allocations are happening
        for (int i = 0; i < 10; ++i) {
            BinaryenArenaDispose(a);
            a = BinaryenArenaCreate();
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
    });

    ready.store(true);
    t1.join();
    t2.join();
    BinaryenArenaDispose(a);
    if (errors.load() != 0) {
        fprintf(stderr, "Errors during race test: %d\n", (int)errors.load());
        return 2;
    }
    printf("arena race-dispose OK\n");
    return 0;
}
