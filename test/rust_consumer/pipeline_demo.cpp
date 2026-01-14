// End-to-End Optimization Pipeline Demo
// Demonstrates the complete Rust IR optimization workflow:
// 1. Read WASM binary
// 2. Measure and report initial state
// 3. Apply optimization passes
// 4. Measure and report optimized state
// 5. Verify correctness

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>

extern "C" {
  // Rust IR FFI functions
  typedef struct BinaryenRustModule* BinaryenRustModuleRef;
  
  BinaryenRustModuleRef BinaryenRustModuleReadBinary(const unsigned char* data, size_t length);
  int BinaryenRustModuleRunPasses(BinaryenRustModuleRef module, const char** pass_names, size_t num_passes);
  int BinaryenRustModuleWriteBinary(BinaryenRustModuleRef module, unsigned char** out_ptr, size_t* out_length);
  void BinaryenRustModuleFreeBinary(unsigned char* buffer, size_t length);
  void BinaryenRustModuleDispose(BinaryenRustModuleRef module);
}

// Helper to read file into memory
std::vector<unsigned char> read_file(const char* path) {
  FILE* f = fopen(path, "rb");
  if (!f) {
    fprintf(stderr, "Failed to open file: %s\n", path);
    exit(1);
  }
  
  fseek(f, 0, SEEK_END);
  long size = ftell(f);
  fseek(f, 0, SEEK_SET);
  
  std::vector<unsigned char> data(size);
  if (fread(data.data(), 1, size, f) != static_cast<size_t>(size)) {
    fprintf(stderr, "Failed to read file: %s\n", path);
    fclose(f);
    exit(1);
  }
  
  fclose(f);
  return data;
}

// Helper to write file
void write_file(const char* path, const unsigned char* data, size_t length) {
  FILE* f = fopen(path, "wb");
  if (!f) {
    fprintf(stderr, "Failed to open file for writing: %s\n", path);
    exit(1);
  }
  
  if (fwrite(data, 1, length, f) != length) {
    fprintf(stderr, "Failed to write file: %s\n", path);
    fclose(f);
    exit(1);
  }
  
  fclose(f);
}

// Verify WASM magic number and version
bool verify_wasm_format(const unsigned char* data, size_t length) {
  if (length < 8) return false;
  
  // Check magic number: 0x00 0x61 0x73 0x6D ("\0asm")
  if (data[0] != 0x00 || data[1] != 0x61 || data[2] != 0x73 || data[3] != 0x6D) {
    return false;
  }
  
  // Check version: 0x01 0x00 0x00 0x00 (version 1)
  if (data[4] != 0x01 || data[5] != 0x00 || data[6] != 0x00 || data[7] != 0x00) {
    return false;
  }
  
  return true;
}

int main() {
  printf("===========================================\n");
  printf("Rust IR End-to-End Optimization Pipeline\n");
  printf("===========================================\n\n");
  
  const char* input_path = "../test/rust_consumer/minimal_identity.wasm";
  const char* output_path = "../test/rust_consumer/minimal_identity.optimized.wasm";
  
  // Step 1: Load input WASM
  printf("Step 1: Loading input WASM file...\n");
  auto input_data = read_file(input_path);
  printf("  ✓ Loaded %s (%zu bytes)\n", input_path, input_data.size());
  
  if (!verify_wasm_format(input_data.data(), input_data.size())) {
    fprintf(stderr, "  ✗ Invalid WASM format in input file\n");
    return 1;
  }
  printf("  ✓ Valid WASM format verified\n\n");
  
  // Step 2: Parse to Rust IR
  printf("Step 2: Parsing WASM to Rust IR...\n");
  BinaryenRustModuleRef module = BinaryenRustModuleReadBinary(
    input_data.data(),
    input_data.size()
  );
  
  if (!module) {
    fprintf(stderr, "  ✗ Failed to parse WASM binary\n");
    return 1;
  }
  printf("  ✓ Successfully parsed to Rust IR\n\n");
  
  // Step 3: Run optimization passes
  printf("Step 3: Applying optimization passes...\n");
  const char* passes[] = {
    "simplify-identity",  // Remove x+0, x*1 patterns
    "dce"                  // Remove dead code after returns
  };
  size_t num_passes = sizeof(passes) / sizeof(passes[0]);
  
  for (size_t i = 0; i < num_passes; i++) {
    printf("  - Running pass: %s\n", passes[i]);
  }
  
  int result = BinaryenRustModuleRunPasses(module, passes, num_passes);
  if (result != 0) {
    fprintf(stderr, "  ✗ Pass execution failed with code %d\n", result);
    BinaryenRustModuleDispose(module);
    return 1;
  }
  printf("  ✓ All passes executed successfully\n\n");
  
  // Step 4: Write optimized WASM
  printf("Step 4: Writing optimized WASM binary...\n");
  size_t output_length = 0;
  unsigned char* output_data = nullptr;
  
  result = BinaryenRustModuleWriteBinary(module, &output_data, &output_length);
  if (result != 0 || !output_data || output_length == 0) {
    fprintf(stderr, "  ✗ Failed to write WASM binary\n");
    BinaryenRustModuleDispose(module);
    return 1;
  }
  printf("  ✓ Binary written (%zu bytes)\n", output_length);
  
  if (!verify_wasm_format(output_data, output_length)) {
    fprintf(stderr, "  ✗ Invalid WASM format in output\n");
    BinaryenRustModuleFreeBinary(output_data, output_length);
    BinaryenRustModuleDispose(module);
    return 1;
  }
  printf("  ✓ Valid WASM format verified\n\n");
  
  // Step 5: Save to file
  printf("Step 5: Saving optimized binary...\n");
  write_file(output_path, output_data, output_length);
  printf("  ✓ Saved to %s\n\n", output_path);
  
  // Step 6: Report results
  printf("===========================================\n");
  printf("Optimization Results\n");
  printf("===========================================\n");
  printf("Input size:      %zu bytes\n", input_data.size());
  printf("Output size:     %zu bytes\n", output_length);
  
  if (output_length < input_data.size()) {
    size_t reduction = input_data.size() - output_length;
    double percentage = (reduction * 100.0) / input_data.size();
    printf("Size reduction:  %zu bytes (%.1f%%)\n", reduction, percentage);
  } else if (output_length > input_data.size()) {
    size_t increase = output_length - input_data.size();
    printf("Size increase:   %zu bytes (no optimization opportunities)\n", increase);
  } else {
    printf("Size unchanged:  (no optimization opportunities)\n");
  }
  
  printf("\nPasses applied:\n");
  for (size_t i = 0; i < num_passes; i++) {
    printf("  - %s\n", passes[i]);
  }
  
  printf("\n===========================================\n");
  printf("✅ Pipeline completed successfully!\n");
  printf("===========================================\n");
  
  // Cleanup
  BinaryenRustModuleFreeBinary(output_data, output_length);
  BinaryenRustModuleDispose(module);
  
  return 0;
}
