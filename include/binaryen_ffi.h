#ifndef BINARYEN_FFI_H
#define BINARYEN_FFI_H

#include <stdint.h>

// An integer ABI version identifier for the FFI. Bump this when making
// incompatible changes to the exported symbols, types, or ownership semantics.
#define BINARYEN_FFI_ABI_VERSION 1

#ifdef __cplusplus
extern "C" {
#endif

uint32_t binaryen_ffi_version();
uint32_t binaryen_ffi_abi_version();

const char* binaryen_ffi_echo(const char* s);

// String interner FFI
typedef struct BinaryenStringInterner BinaryenStringInterner;

BinaryenStringInterner* BinaryenStringInternerCreate(void);
void BinaryenStringInternerDispose(BinaryenStringInterner*);

const char* BinaryenStringInternerIntern(BinaryenStringInterner* interner, const char* s);

// Arena FFI
typedef struct BinaryenArena BinaryenArena;
BinaryenArena* BinaryenArenaCreate(void);
void BinaryenArenaDispose(BinaryenArena*);

const char* BinaryenArenaAllocString(BinaryenArena* arena, const char* s);

#ifdef __cplusplus
}
#endif

#endif // BINARYEN_FFI_H
