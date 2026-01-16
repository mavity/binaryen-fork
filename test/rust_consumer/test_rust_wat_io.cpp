#include "binaryen_ffi.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main() {
  printf("Testing Rust IR WAT I/O Integration...\n");

  const char* wat = 
    "(module\n"
    "  (func $main (result i32)\n"
    "    (i32.const 42)\n"
    "  )\n"
    "  (export \"main\" (func $main))\n"
    ")";

  // 1. Read WAT
  printf("1. Reading WAT...\n");
  BinaryenRustModuleRef module = BinaryenRustModuleReadWat(wat);
  if (module == NULL) {
    printf("FAILED: BinaryenRustModuleReadWat returned NULL\n");
    return 1;
  }
  assert(module != NULL);

  // 2. Write WAT
  printf("2. Writing WAT...\n");
  char* output_wat = BinaryenRustModuleToWat(module);
  if (output_wat == NULL) {
    printf("FAILED: BinaryenRustModuleToWat returned NULL\n");
    return 1;
  }
  printf("Output WAT:\n%s\n", output_wat);

  // 3. Verify content
  printf("3. Verifying output...\n");
  // The output might be formatted differently, but it should contain the export and the constant.
  assert(strstr(output_wat, "main") != NULL);
  assert(strstr(output_wat, "42") != NULL);

  // 4. Cleanup
  printf("4. Cleaning up...\n");
  BinaryenRustModuleFreeWatString(output_wat);
  BinaryenRustModuleDispose(module);

  printf("SUCCESS: Rust IR WAT I/O Integration verified!\n");
  return 0;
}
