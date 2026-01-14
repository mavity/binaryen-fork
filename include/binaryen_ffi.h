#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define BINARYEN_FFI_ABI_VERSION 1

// ---------------------------------------------------------------------------
// INTERNAL-ONLY: WARNING
// ---------------------------------------------------------------------------
// This header declares the C ABI exported by the Rust `binaryen-ffi` crate.
// It is intended for INTERNAL use within this repository only — for the Rust
// port and the `test/rust_consumer` smoke tests. It is NOT a supported public
// ABI for external consumers. External use is FORBIDDEN and DISCOURAGED.
//
// The symbols and layout can change without notice. Do not rely on this header
// from external code. If you need stable public ABI, use the C API provided by
// `binaryen` core or request a supported public API to be added.
// ---------------------------------------------------------------------------

typedef struct BinaryenStringInterner {
  uint8_t _private[0];
} BinaryenStringInterner;

typedef struct BinaryenArena {
  uint8_t _private[0];
} BinaryenArena;

typedef struct BinaryenFastHashMap {
  uint8_t _private[0];
} BinaryenFastHashMap;

// `extern "C"` guard for C++ consumers to ensure C linkage when included from C++
#ifdef __cplusplus
extern "C" {
#endif

uint32_t binaryen_ffi_version(void);

uint32_t binaryen_ffi_abi_version(void);

const char *binaryen_ffi_echo(const char *s);

struct BinaryenStringInterner *BinaryenStringInternerCreate(void);

void BinaryenStringInternerDispose(struct BinaryenStringInterner *p);

const char *BinaryenStringInternerIntern(struct BinaryenStringInterner *p, const char *s);

struct BinaryenArena *BinaryenArenaCreate(void);

void BinaryenArenaDispose(struct BinaryenArena *p);

const char *BinaryenArenaAllocString(struct BinaryenArena *p, const char *s);

int BinaryenArenaIsAlive(struct BinaryenArena *p);

typedef struct BinaryenArenaHandle {
  uint8_t _private[0];
} BinaryenArenaHandle;

struct BinaryenArenaHandle *BinaryenArenaHandleCreate(void);
void BinaryenArenaHandleDispose(struct BinaryenArenaHandle *h);
const char *BinaryenArenaHandleAllocString(struct BinaryenArenaHandle *h, const char *s);
int BinaryenArenaHandleIsAlive(struct BinaryenArenaHandle *h);

uint64_t BinaryenAhashBytes(const unsigned char *data, uintptr_t len);

struct BinaryenFastHashMap *BinaryenFastHashMapCreate(void);

void BinaryenFastHashMapDispose(struct BinaryenFastHashMap *p);

bool BinaryenFastHashMapInsert(struct BinaryenFastHashMap *p, const char *key, uint64_t value);

bool BinaryenFastHashMapGet(struct BinaryenFastHashMap *p, const char *key, uint64_t *out_value);

uintptr_t BinaryenFastHashMapLen(struct BinaryenFastHashMap *p);

// Type system helpers (added by Rust port)
typedef uint64_t BinaryenType;

#ifdef __cplusplus
extern "C" {
#endif

BinaryenType BinaryenTypeCreateSignature(BinaryenType params, BinaryenType results);
BinaryenType BinaryenTypeGetParams(BinaryenType ty);
BinaryenType BinaryenTypeGetResults(BinaryenType ty);

BinaryenType BinaryenTypeInt32(void);
BinaryenType BinaryenTypeInt64(void);
BinaryenType BinaryenTypeFloat32(void);
BinaryenType BinaryenTypeFloat64(void);
BinaryenType BinaryenTypeVec128(void);
BinaryenType BinaryenTypeNone(void);

#ifdef __cplusplus
}
#endif

#ifdef __cplusplus
}
#endif
