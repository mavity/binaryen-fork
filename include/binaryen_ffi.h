#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define BINARYEN_FFI_ABI_VERSION 1

typedef struct BinaryenStringInterner {
  uint8_t _private[0];
} BinaryenStringInterner;

typedef struct BinaryenArena {
  uint8_t _private[0];
} BinaryenArena;

typedef struct BinaryenFastHashMap {
  uint8_t _private[0];
} BinaryenFastHashMap;

uint32_t binaryen_ffi_version(void);

uint32_t binaryen_ffi_abi_version(void);

// `extern "C"` guard for C++ consumers to ensure C linkage when included from C++
#ifdef __cplusplus
extern "C" {
#endif

const char *binaryen_ffi_echo(const char *s);

struct BinaryenStringInterner *BinaryenStringInternerCreate(void);

void BinaryenStringInternerDispose(struct BinaryenStringInterner *p);

const char *BinaryenStringInternerIntern(struct BinaryenStringInterner *p, const char *s);

struct BinaryenArena *BinaryenArenaCreate(void);

void BinaryenArenaDispose(struct BinaryenArena *p);

const char *BinaryenArenaAllocString(struct BinaryenArena *p, const char *s);

int BinaryenArenaIsAlive(struct BinaryenArena *p);

uint64_t BinaryenAhashBytes(const unsigned char *data, uintptr_t len);

struct BinaryenFastHashMap *BinaryenFastHashMapCreate(void);

void BinaryenFastHashMapDispose(struct BinaryenFastHashMap *p);

bool BinaryenFastHashMapInsert(struct BinaryenFastHashMap *p, const char *key, uint64_t value);

bool BinaryenFastHashMapGet(struct BinaryenFastHashMap *p, const char *key, uint64_t *out_value);

uintptr_t BinaryenFastHashMapLen(struct BinaryenFastHashMap *p);

#ifdef __cplusplus
}
#endif
