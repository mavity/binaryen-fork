#include <stdio.h>
#include <assert.h>
#include <string.h>
#include "binaryen_ffi.h"

// Smoke test for the Rust IR module interface
int main() {
    printf("Creating module...\n");
    BinaryenRustModuleRef module = BinaryenRustModuleCreate();
    assert(module != NULL);

    printf("Creating standard types...\n");
    BinaryenType i32 = BinaryenTypeInt32();
    
    printf("Creating expressions...\n");
    // Create: (i32.add (i32.const 1) (i32.const 2))
    BinaryenRustExpressionRef c1 = BinaryenRustConst(module, 1);
    BinaryenRustExpressionRef c2 = BinaryenRustConst(module, 2);
    
    // AddInt32 = 0 (based on definition in Rust ops.rs)
    BinaryenRustExpressionRef add = BinaryenRustBinary(module, 0, c1, c2, i32);
    assert(add != NULL);

    printf("Adding function...\n");
    BinaryenRustAddFunction(module, "test_func", BinaryenTypeNone(), i32, add);

    printf("Creating block...\n");
    BinaryenRustExpressionRef expressions[] = { c1, c2 };
    BinaryenRustExpressionRef block = BinaryenRustBlock(module, "my_block", expressions, 2, i32);
    assert(block != NULL);

    printf("Disposing module...\n");
    BinaryenRustModuleDispose(module);
    
    printf("Module test passed!\n");
    return 0;
}
