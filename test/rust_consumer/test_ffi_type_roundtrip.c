/*
 * Test roundtrip of Type creation through FFI
 */
#include "../../include/binaryen_ffi.h"
#include <assert.h>
#include <stdio.h>

int main() {
    printf("Testing Type FFI roundtrip...\n");
    
    // Get basic types
    BinaryenType i32_ty = BinaryenTypeInt32();
    BinaryenType i64_ty = BinaryenTypeInt64();
    BinaryenType f32_ty = BinaryenTypeFloat32();
    BinaryenType f64_ty = BinaryenTypeFloat64();
    BinaryenType none_ty = BinaryenTypeNone();
    
    printf("  Basic types retrieved: i32, i64, f32, f64, none\n");
    
    // Create a signature (i32) -> (i64)
    BinaryenType sig1 = BinaryenTypeCreateSignature(i32_ty, i64_ty);
    printf("  Created signature (i32) -> (i64)\n");
    
    // Verify we can get params/results back
    BinaryenType params = BinaryenTypeGetParams(sig1);
    BinaryenType results = BinaryenTypeGetResults(sig1);
    
    assert(params == i32_ty && "Params should be i32");
    assert(results == i64_ty && "Results should be i64");
    printf("  ✓ Params and results match\n");
    
    // Test interning: creating same signature again should yield same handle
    BinaryenType sig2 = BinaryenTypeCreateSignature(i32_ty, i64_ty);
    assert(sig1 == sig2 && "Same signature should be interned to same ID");
    printf("  ✓ Signature interning works (sig1 == sig2)\n");
    
    // Different signature should have different ID
    BinaryenType sig3 = BinaryenTypeCreateSignature(f32_ty, f64_ty);
    assert(sig1 != sig3 && "Different signatures should have different IDs");
    printf("  ✓ Different signatures have different IDs\n");
    
    // Getting params from a basic type should return none
    BinaryenType basic_params = BinaryenTypeGetParams(i32_ty);
    assert(basic_params == none_ty && "Basic type should return none for params");
    printf("  ✓ Basic types return none for params query\n");
    
    printf("All Type FFI roundtrip tests passed!\n");
    return 0;
}
