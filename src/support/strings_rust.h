#pragma once

#ifdef BUILD_RUST_COMPONENTS
#include "../../include/binaryen_ffi.h"
#include <cstddef>
extern "C" {
    inline void* BinaryenStringInternerCreateWrapper() { return reinterpret_cast<void*>(BinaryenStringInternerCreate()); }
    inline void BinaryenStringInternerDisposeWrapper(void* p) { BinaryenStringInternerDispose(reinterpret_cast<BinaryenStringInterner*>(p)); }
    inline const char* BinaryenStringInternerInternWrapper(void* p, const char* s) { return BinaryenStringInternerIntern(reinterpret_cast<BinaryenStringInterner*>(p), s); }
    inline uint64_t BinaryenAhashBytesWrapper(const uint8_t* data, size_t len) { return BinaryenAhashBytes(data, len); }
    inline void* BinaryenFastHashMapCreateWrapper() { return reinterpret_cast<void*>(BinaryenFastHashMapCreate()); }
    inline void BinaryenFastHashMapDisposeWrapper(void* p) { BinaryenFastHashMapDispose(reinterpret_cast<BinaryenFastHashMap*>(p)); }
    inline bool BinaryenFastHashMapInsertWrapper(void* p, const char* key, uint64_t v) { return BinaryenFastHashMapInsert(reinterpret_cast<BinaryenFastHashMap*>(p), key, v); }
    inline bool BinaryenFastHashMapGetWrapper(void* p, const char* key, uint64_t* out) { return BinaryenFastHashMapGet(reinterpret_cast<BinaryenFastHashMap*>(p), key, out); }
    inline size_t BinaryenFastHashMapLenWrapper(void* p) { return BinaryenFastHashMapLen(reinterpret_cast<BinaryenFastHashMap*>(p)); }
}
#endif
