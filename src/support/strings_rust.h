#pragma once

#ifdef BUILD_RUST_COMPONENTS
#include "../../include/binaryen_ffi.h"
#include <cstddef>
extern "C" {
    inline void* BinaryenStringInternerCreateWrapper() { return reinterpret_cast<void*>(BinaryenStringInternerCreate()); }
    inline void BinaryenStringInternerDisposeWrapper(void* p) { BinaryenStringInternerDispose(reinterpret_cast<BinaryenStringInterner*>(p)); }
    inline const char* BinaryenStringInternerInternWrapper(void* p, const char* s) { return BinaryenStringInternerIntern(reinterpret_cast<BinaryenStringInterner*>(p), s); }
}
#endif
