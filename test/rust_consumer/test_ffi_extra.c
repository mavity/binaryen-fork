#include <stdio.h>
#include <string.h>
#include "../../include/binaryen_ffi.h"

int main() {
    // Extra cross-language validation for FFI ownership and map semantics
    // Check missing key behavior
    BinaryenFastHashMap* fm = BinaryenFastHashMapCreate();
    if (!fm) { fprintf(stderr, "fast map create failed\n"); return 2; }
    uint64_t outv = 0;
    if (BinaryenFastHashMapGet(fm, "missing", &outv)) { fprintf(stderr, "expected missing key to return false\n"); return 2; }

    // Insert and overwrite semantics
    if (!BinaryenFastHashMapInsert(fm, "one", 10)) { fprintf(stderr, "map insert failed\n"); return 2; }
    uint64_t val = 0;
    if (!BinaryenFastHashMapGet(fm, "one", &val) || val != 10) { fprintf(stderr, "unexpected or missing value after insert\n"); return 2; }
    // Insert again with new value
    if (!BinaryenFastHashMapInsert(fm, "one", 20)) { fprintf(stderr, "map insert failed (overwrite)\n"); return 2; }
    if (!BinaryenFastHashMapGet(fm, "one", &val) || val != 20) { fprintf(stderr, "unexpected or missing value after overwrite\n"); return 2; }

    // Test repeated create/dispose for string interner
    for (int i = 0; i < 3; i++) {
        BinaryenStringInterner* inter = BinaryenStringInternerCreate();
        const char* s1 = BinaryenStringInternerIntern(inter, "a-unique-string");
        const char* s2 = BinaryenStringInternerIntern(inter, "a-unique-string");
        if (s1 != s2) { fprintf(stderr, "interner pointer mismatch\n"); return 2; }
        BinaryenStringInternerDispose(inter);
    }

    BinaryenFastHashMapDispose(fm);
    return 0;
}

