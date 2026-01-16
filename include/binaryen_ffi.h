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

BinaryenType BinaryenTypeCreateSignature(BinaryenType params, BinaryenType results);
BinaryenType BinaryenTypeGetParams(BinaryenType ty);
BinaryenType BinaryenTypeGetResults(BinaryenType ty);

BinaryenType BinaryenTypeInt32(void);
BinaryenType BinaryenTypeInt64(void);
BinaryenType BinaryenTypeFloat32(void);
BinaryenType BinaryenTypeFloat64(void);
BinaryenType BinaryenTypeVec128(void);
BinaryenType BinaryenTypeNone(void);

// IR and Module interface (added by Rust port)
typedef struct BinaryenRustModule *BinaryenRustModuleRef;
typedef struct BinaryenRustExpression *BinaryenRustExpressionRef;

BinaryenRustModuleRef BinaryenRustModuleCreate(void);
void BinaryenRustModuleDispose(BinaryenRustModuleRef module);

BinaryenRustModuleRef BinaryenRustModuleReadBinary(const uint8_t *bytes, uintptr_t len);
int32_t BinaryenRustModuleWriteBinary(BinaryenRustModuleRef module, uint8_t **out_ptr, uintptr_t *out_len);
void BinaryenRustModuleFreeBinary(uint8_t *ptr, uintptr_t len);

BinaryenRustModuleRef BinaryenRustModuleReadWat(const char *wat);
char *BinaryenRustModuleToWat(BinaryenRustModuleRef module);
void BinaryenRustModuleFreeWatString(char *wat);

int32_t BinaryenRustModuleRunPasses(BinaryenRustModuleRef module, const char **pass_names, uintptr_t num_passes);

BinaryenRustExpressionRef BinaryenRustConst(BinaryenRustModuleRef module, int32_t value);
BinaryenRustExpressionRef BinaryenRustBlock(BinaryenRustModuleRef module, const char *name, BinaryenRustExpressionRef *children, uintptr_t num_children, BinaryenType type);
BinaryenRustExpressionRef BinaryenRustUnary(BinaryenRustModuleRef module, uint32_t op, BinaryenRustExpressionRef value, BinaryenType type);
BinaryenRustExpressionRef BinaryenRustBinary(BinaryenRustModuleRef module, uint32_t op, BinaryenRustExpressionRef left, BinaryenRustExpressionRef right, BinaryenType type);
BinaryenRustExpressionRef BinaryenRustLocalGet(BinaryenRustModuleRef module, uint32_t index, BinaryenType type);
BinaryenRustExpressionRef BinaryenRustLocalSet(BinaryenRustModuleRef module, uint32_t index, BinaryenRustExpressionRef value);

void BinaryenRustAddFunction(BinaryenRustModuleRef module, const char *name, BinaryenType params, BinaryenType results, BinaryenRustExpressionRef body);

#ifdef __cplusplus
}
#endif
