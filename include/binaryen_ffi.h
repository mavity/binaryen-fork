#ifndef BINARYEN_FFI_H
#define BINARYEN_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

uint32_t binaryen_ffi_version();

const char* binaryen_ffi_echo(const char* s);

#ifdef __cplusplus
}
#endif

#endif // BINARYEN_FFI_H
