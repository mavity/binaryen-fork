/**
 * C++ Round-trip test for Type FFI
 *
 * This test validates the Rust TypeStore against C++ expectations using
 * comprehensive type creation, interning, and querying scenarios.
 *
 * Key Validation Areas:
 * 1. Basic type retrieval and identity
 * 2. Signature creation and interning
 * 3. Round-trip parameter/result extraction
 * 4. Signature equality and canonicalization
 * 5. Edge cases (basic types, none types, complex signatures)
 */

#include "../../include/binaryen_ffi.h"
#include <cassert>
#include <cstdio>
#include <cstring>
#include <vector>
#include <unordered_map>
#include <functional>

// Helper to create a unique key from two BinaryenType values
// Uses standard hash combining approach to avoid truncation issues
inline size_t hash_pair(BinaryenType a, BinaryenType b) {
    size_t h1 = std::hash<uint64_t>{}(a);
    size_t h2 = std::hash<uint64_t>{}(b);
    // Standard hash combination from Boost
    return h1 ^ (h2 + 0x9e3779b9 + (h1 << 6) + (h1 >> 2));
}

// Test result tracking
struct TestResults {
    int passed = 0;
    int failed = 0;
    
    void pass(const char* test_name) {
        printf("  ✓ %s\n", test_name);
        passed++;
    }
    
    void fail(const char* test_name, const char* reason) {
        printf("  ✗ %s: %s\n", test_name, reason);
        failed++;
    }
    
    void summarize() {
        printf("\n===========================================\n");
        printf("Test Results: %d passed, %d failed\n", passed, failed);
        printf("===========================================\n");
    }
};

// Helper to verify type equality
#define ASSERT_TYPE_EQ(actual, expected, msg) \
    do { \
        if ((actual) != (expected)) { \
            results.fail(__func__, msg); \
            return; \
        } \
    } while(0)

// Helper to verify type inequality
#define ASSERT_TYPE_NE(actual, expected, msg) \
    do { \
        if ((actual) == (expected)) { \
            results.fail(__func__, msg); \
            return; \
        } \
    } while(0)

TestResults results;

// Test 1: Basic type constants are retrievable and stable
void test_basic_type_constants() {
    BinaryenType i32 = BinaryenTypeInt32();
    BinaryenType i64 = BinaryenTypeInt64();
    BinaryenType f32 = BinaryenTypeFloat32();
    BinaryenType f64 = BinaryenTypeFloat64();
    BinaryenType v128 = BinaryenTypeVec128();
    BinaryenType none = BinaryenTypeNone();
    
    // Basic types should be distinct
    ASSERT_TYPE_NE(i32, i64, "i32 and i64 should be different");
    ASSERT_TYPE_NE(i32, f32, "i32 and f32 should be different");
    ASSERT_TYPE_NE(f32, f64, "f32 and f64 should be different");
    ASSERT_TYPE_NE(i32, none, "i32 and none should be different");
    ASSERT_TYPE_NE(v128, i32, "v128 and i32 should be different");
    
    // Calling again should return same values (stable addresses/IDs)
    ASSERT_TYPE_EQ(BinaryenTypeInt32(), i32, "i32 should be stable");
    ASSERT_TYPE_EQ(BinaryenTypeFloat64(), f64, "f64 should be stable");
    ASSERT_TYPE_EQ(BinaryenTypeNone(), none, "none should be stable");
    
    results.pass(__func__);
}

// Test 2: Create a simple signature and verify round-trip
void test_simple_signature_roundtrip() {
    BinaryenType i32 = BinaryenTypeInt32();
    BinaryenType i64 = BinaryenTypeInt64();
    
    // Create signature (i32) -> (i64)
    BinaryenType sig = BinaryenTypeCreateSignature(i32, i64);
    
    // Verify params and results round-trip correctly
    BinaryenType params = BinaryenTypeGetParams(sig);
    BinaryenType result_types = BinaryenTypeGetResults(sig);
    
    ASSERT_TYPE_EQ(params, i32, "Params should be i32");
    ASSERT_TYPE_EQ(result_types, i64, "Results should be i64");
    
    results.pass(__func__);
}

// Test 3: Signature interning - same signature should yield same handle
void test_signature_interning() {
    BinaryenType i32 = BinaryenTypeInt32();
    BinaryenType f64 = BinaryenTypeFloat64();
    
    // Create same signature twice
    BinaryenType sig1 = BinaryenTypeCreateSignature(i32, f64);
    BinaryenType sig2 = BinaryenTypeCreateSignature(i32, f64);
    
    ASSERT_TYPE_EQ(sig1, sig2, "Same signature should intern to same handle");
    
    results.pass(__func__);
}

// Test 4: Different signatures should have different handles
void test_different_signatures() {
    BinaryenType i32 = BinaryenTypeInt32();
    BinaryenType i64 = BinaryenTypeInt64();
    BinaryenType f32 = BinaryenTypeFloat32();
    BinaryenType f64 = BinaryenTypeFloat64();
    
    BinaryenType sig1 = BinaryenTypeCreateSignature(i32, i64);
    BinaryenType sig2 = BinaryenTypeCreateSignature(f32, f64);
    BinaryenType sig3 = BinaryenTypeCreateSignature(i64, i32); // reversed
    
    ASSERT_TYPE_NE(sig1, sig2, "Different param/result types should differ");
    ASSERT_TYPE_NE(sig1, sig3, "Param/result order matters");
    ASSERT_TYPE_NE(sig2, sig3, "All three should be distinct");
    
    results.pass(__func__);
}

// Test 5: Query params/results from basic types should return none
void test_basic_type_queries() {
    BinaryenType i32 = BinaryenTypeInt32();
    BinaryenType f64 = BinaryenTypeFloat64();
    BinaryenType none = BinaryenTypeNone();
    
    // Basic types are not signatures, should return none for queries
    BinaryenType i32_params = BinaryenTypeGetParams(i32);
    BinaryenType f64_params = BinaryenTypeGetParams(f64);
    BinaryenType i32_result_types = BinaryenTypeGetResults(i32);
    
    ASSERT_TYPE_EQ(i32_params, none, "i32 params should be none");
    ASSERT_TYPE_EQ(f64_params, none, "f64 params should be none");
    ASSERT_TYPE_EQ(i32_result_types, none, "i32 results should be none");
    
    results.pass(__func__);
}

// Test 6: Signatures with none types (void functions)
void test_none_signatures() {
    BinaryenType i32 = BinaryenTypeInt32();
    BinaryenType none = BinaryenTypeNone();
    
    // (i32) -> ()
    BinaryenType sig1 = BinaryenTypeCreateSignature(i32, none);
    BinaryenType params1 = BinaryenTypeGetParams(sig1);
    BinaryenType result_types1 = BinaryenTypeGetResults(sig1);
    
    ASSERT_TYPE_EQ(params1, i32, "Params should be i32");
    ASSERT_TYPE_EQ(result_types1, none, "Results should be none");
    
    // () -> (i32)
    BinaryenType sig2 = BinaryenTypeCreateSignature(none, i32);
    BinaryenType params2 = BinaryenTypeGetParams(sig2);
    BinaryenType result_types2 = BinaryenTypeGetResults(sig2);
    
    ASSERT_TYPE_EQ(params2, none, "Params should be none");
    ASSERT_TYPE_EQ(result_types2, i32, "Results should be i32");
    
    // () -> ()
    BinaryenType sig3 = BinaryenTypeCreateSignature(none, none);
    BinaryenType params3 = BinaryenTypeGetParams(sig3);
    BinaryenType result_types3 = BinaryenTypeGetResults(sig3);
    
    ASSERT_TYPE_EQ(params3, none, "Params should be none");
    ASSERT_TYPE_EQ(result_types3, none, "Results should be none");
    
    // All three should be distinct
    ASSERT_TYPE_NE(sig1, sig2, "Different void signatures should differ");
    ASSERT_TYPE_NE(sig1, sig3, "Different void signatures should differ");
    ASSERT_TYPE_NE(sig2, sig3, "Different void signatures should differ");
    
    results.pass(__func__);
}

// Test 7: Multiple signatures with same components should intern correctly
void test_multiple_signature_interning() {
    BinaryenType i32 = BinaryenTypeInt32();
    BinaryenType i64 = BinaryenTypeInt64();
    BinaryenType f32 = BinaryenTypeFloat32();
    
    // Create multiple signatures and verify interning works across all
    std::vector<std::pair<BinaryenType, BinaryenType>> test_cases = {
        {i32, i64},
        {i64, i32},
        {f32, i32},
        {i32, f32},
        {i32, i32}, // same type for both
        {f32, f32},
    };
    
    std::unordered_map<size_t, BinaryenType> first_occurrence;
    
    for (const auto& test_case : test_cases) {
        BinaryenType params = test_case.first;
        BinaryenType result_types = test_case.second;
        
        // Create signature twice
        BinaryenType sig1 = BinaryenTypeCreateSignature(params, result_types);
        BinaryenType sig2 = BinaryenTypeCreateSignature(params, result_types);
        
        // Should intern to same value
        ASSERT_TYPE_EQ(sig1, sig2, "Repeated creation should intern");
        
        // Track first occurrence for cross-signature uniqueness
        size_t key = hash_pair(params, result_types);
        
        if (first_occurrence.find(key) == first_occurrence.end()) {
            first_occurrence[key] = sig1;
        } else {
            ASSERT_TYPE_EQ(sig1, first_occurrence[key], "Should match first occurrence");
        }
    }
    
    // Verify different signatures got different handles
    std::vector<BinaryenType> sigs;
    for (const auto& entry : first_occurrence) {
        sigs.push_back(entry.second);
    }
    
    for (size_t i = 0; i < sigs.size(); i++) {
        for (size_t j = i + 1; j < sigs.size(); j++) {
            ASSERT_TYPE_NE(sigs[i], sigs[j], "Different signatures must have different handles");
        }
    }
    
    results.pass(__func__);
}

// Test 8: All basic types together
void test_all_basic_types() {
    BinaryenType i32 = BinaryenTypeInt32();
    BinaryenType i64 = BinaryenTypeInt64();
    BinaryenType f32 = BinaryenTypeFloat32();
    BinaryenType f64 = BinaryenTypeFloat64();
    BinaryenType v128 = BinaryenTypeVec128();
    BinaryenType none = BinaryenTypeNone();
    
    // Create signatures using all basic types
    std::vector<BinaryenType> basic_types = {i32, i64, f32, f64, v128, none};
    
    // Test creating signatures with each type as param and result
    for (auto param : basic_types) {
        for (auto result : basic_types) {
            BinaryenType sig = BinaryenTypeCreateSignature(param, result);
            
            BinaryenType extracted_param = BinaryenTypeGetParams(sig);
            BinaryenType extracted_result = BinaryenTypeGetResults(sig);
            
            ASSERT_TYPE_EQ(extracted_param, param, "Param mismatch");
            ASSERT_TYPE_EQ(extracted_result, result, "Result mismatch");
        }
    }
    
    results.pass(__func__);
}

// Test 9: Signature identity across repeated creations
void test_signature_identity_stress() {
    BinaryenType i32 = BinaryenTypeInt32();
    BinaryenType f64 = BinaryenTypeFloat64();
    
    // Create same signature many times
    const int iterations = 100;
    BinaryenType first_sig = BinaryenTypeCreateSignature(i32, f64);
    
    for (int i = 0; i < iterations; i++) {
        BinaryenType sig = BinaryenTypeCreateSignature(i32, f64);
        ASSERT_TYPE_EQ(sig, first_sig, "All iterations should yield same handle");
    }
    
    results.pass(__func__);
}

// Test 10: V128 type specific tests
void test_v128_type() {
    BinaryenType v128 = BinaryenTypeVec128();
    BinaryenType i32 = BinaryenTypeInt32();
    
    // Create signatures with v128
    BinaryenType sig1 = BinaryenTypeCreateSignature(v128, i32);
    BinaryenType sig2 = BinaryenTypeCreateSignature(i32, v128);
    BinaryenType sig3 = BinaryenTypeCreateSignature(v128, v128);
    
    ASSERT_TYPE_NE(sig1, sig2, "v128 signatures should be distinct");
    ASSERT_TYPE_NE(sig1, sig3, "v128 signatures should be distinct");
    ASSERT_TYPE_NE(sig2, sig3, "v128 signatures should be distinct");
    
    // Verify round-trip
    ASSERT_TYPE_EQ(BinaryenTypeGetParams(sig1), v128, "sig1 params should be v128");
    ASSERT_TYPE_EQ(BinaryenTypeGetResults(sig1), i32, "sig1 results should be i32");
    
    ASSERT_TYPE_EQ(BinaryenTypeGetParams(sig2), i32, "sig2 params should be i32");
    ASSERT_TYPE_EQ(BinaryenTypeGetResults(sig2), v128, "sig2 results should be v128");
    
    ASSERT_TYPE_EQ(BinaryenTypeGetParams(sig3), v128, "sig3 params should be v128");
    ASSERT_TYPE_EQ(BinaryenTypeGetResults(sig3), v128, "sig3 results should be v128");
    
    results.pass(__func__);
}

int main() {
    printf("===========================================\n");
    printf("C++ Type Roundtrip FFI Test Suite\n");
    printf("===========================================\n\n");
    
    printf("Running comprehensive type system validation...\n\n");
    
    // Run all tests
    test_basic_type_constants();
    test_simple_signature_roundtrip();
    test_signature_interning();
    test_different_signatures();
    test_basic_type_queries();
    test_none_signatures();
    test_multiple_signature_interning();
    test_all_basic_types();
    test_signature_identity_stress();
    test_v128_type();
    
    // Print summary
    results.summarize();
    
    if (results.failed > 0) {
        printf("\n❌ Some tests failed!\n");
        return 1;
    }
    
    printf("\n✅ All type roundtrip tests passed!\n");
    return 0;
}
