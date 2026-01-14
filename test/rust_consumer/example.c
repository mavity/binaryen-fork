// Example C code with optimization opportunities for the Rust IR optimizer
#include <stdint.h>

// Function with identity operations that SimplifyIdentity can optimize
int32_t compute_value(int32_t x, int32_t y) {
    // These identity operations should be optimized away:
    int32_t a = x + 0;        // x + 0 -> x
    int32_t b = y * 1;        // y * 1 -> y
    int32_t c = a + b;        // Should become x + y
    
    // Some real computation
    int32_t result = c * 2;
    result = result + 10;
    
    return result;
}

// Function with unreachable code that DCE can remove
int32_t check_bounds(int32_t value, int32_t max) {
    if (value < 0) {
        return -1;
        // Dead code after return - DCE should remove this
        value = value * 2;
        value = value + 100;
    }
    
    if (value > max) {
        return -2;
        // More dead code
        value = value / 2;
    }
    
    return value;
}

// Function that combines both optimization opportunities
int32_t process_array_element(int32_t element, int32_t index) {
    // Identity operations
    int32_t adjusted = element + 0;
    adjusted = adjusted * 1;
    
    // Bounds checking with unreachable code
    if (adjusted < 0) {
        return 0;
        adjusted = adjusted + 1000;  // Dead code
    }
    
    // Apply some transformation
    int32_t result = adjusted + index;
    result = result * 1;  // Another identity
    
    return result;
}

// Export functions for WASM
__attribute__((export_name("compute_value")))
int32_t exported_compute_value(int32_t x, int32_t y) {
    return compute_value(x, y);
}

__attribute__((export_name("check_bounds")))
int32_t exported_check_bounds(int32_t value, int32_t max) {
    return check_bounds(value, max);
}

__attribute__((export_name("process_array_element")))
int32_t exported_process_array_element(int32_t element, int32_t index) {
    return process_array_element(element, index);
}
