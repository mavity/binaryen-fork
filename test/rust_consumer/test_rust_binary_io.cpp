#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// FFI declarations
extern "C" {
typedef void* BinaryenRustModuleRef;

BinaryenRustModuleRef BinaryenRustModuleReadBinary(const unsigned char* bytes,
                                                    size_t len);
int BinaryenRustModuleWriteBinary(BinaryenRustModuleRef module,
                                   unsigned char** out_ptr,
                                   size_t* out_len);
void BinaryenRustModuleFreeBinary(unsigned char* ptr, size_t len);
void BinaryenRustModuleDispose(BinaryenRustModuleRef module);
int BinaryenRustModuleRunPasses(BinaryenRustModuleRef module,
                                 const char** pass_names,
                                 size_t num_passes);
}

int main() {
  printf("Testing Rust IR Binary I/O Integration...\n");

  // Minimal WASM: (module (func (result i32) i32.const 42))
  unsigned char wasm[] = {
      0x00, 0x61, 0x73, 0x6D, // magic
      0x01, 0x00, 0x00, 0x00, // version
      // Type section
      0x01, 0x05,             // section 1, size 5
      0x01,                   // 1 type
      0x60, 0x00, 0x01, 0x7F, // func type: () -> i32
      // Function section
      0x03, 0x02, // section 3, size 2
      0x01, 0x00, // 1 function, type 0
      // Code section
      0x0A, 0x06, // section 10, size 6
      0x01,       // 1 code
      0x04,       // body size 4
      0x00,       // 0 locals
      0x41, 0x2A, // i32.const 42
      0x0B,       // end
  };
  size_t wasm_len = sizeof(wasm);

  // 1. Read binary
  printf("1. Reading WASM binary...\n");
  BinaryenRustModuleRef module =
      BinaryenRustModuleReadBinary(wasm, wasm_len);
  assert(module != nullptr);
  printf("   ✓ Module loaded\n");

  // 2. Run optimization passes
  printf("2. Running optimization passes...\n");
  const char* passes[] = {"simplify-identity", "dce"};
  int result = BinaryenRustModuleRunPasses(module, passes, 2);
  assert(result == 0);
  printf("   ✓ Passes executed\n");

  // 3. Write binary back
  printf("3. Writing WASM binary...\n");
  unsigned char* out_ptr = nullptr;
  size_t out_len = 0;
  result = BinaryenRustModuleWriteBinary(module, &out_ptr, &out_len);
  assert(result == 0);
  assert(out_ptr != nullptr);
  assert(out_len > 0);
  printf("   ✓ Binary written (%zu bytes)\n", out_len);

  // 4. Verify magic and version
  printf("4. Verifying output...\n");
  assert(out_len >= 8);
  assert(out_ptr[0] == 0x00 && out_ptr[1] == 0x61 && out_ptr[2] == 0x73 &&
         out_ptr[3] == 0x6D);
  assert(out_ptr[4] == 0x01 && out_ptr[5] == 0x00 && out_ptr[6] == 0x00 &&
         out_ptr[7] == 0x00);
  printf("   ✓ Valid WASM header\n");

  // 5. Cleanup
  BinaryenRustModuleFreeBinary(out_ptr, out_len);
  BinaryenRustModuleDispose(module);
  printf("   ✓ Cleanup complete\n");

  printf("\n✅ All tests passed!\n");
  return 0;
}
