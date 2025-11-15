# Binaryen Rust Conversion - Technical Specifications

## Overview

This document provides detailed technical specifications for converting Binaryen components to Rust. It includes code patterns, FFI strategies, safety considerations, and implementation guidelines.

## FFI Patterns

### Basic Opaque Types

When exposing Rust types to C/C++, use opaque pointers with explicit create/dispose functions:

```rust
// Rust side - Internal type
pub struct Module {
    functions: Vec<Function>,
    // ... other fields
}

// Opaque handle for C
#[repr(C)]
pub struct BinaryenModuleRef {
    _private: [u8; 0],
}

#[no_mangle]
pub extern "C" fn BinaryenModuleCreate() -> *mut BinaryenModuleRef {
    let module = Box::new(Module::new());
    Box::into_raw(module) as *mut BinaryenModuleRef
}

#[no_mangle]
pub extern "C" fn BinaryenModuleDispose(module: *mut BinaryenModuleRef) {
    if !module.is_null() {
        unsafe { 
            let _ = Box::from_raw(module as *mut Module);
        }
    }
}
```

## Memory Management

### Arena Allocation

Binaryen uses arena allocation for IR nodes. Rust equivalent using `bumpalo`:

```rust
use bumpalo::Bump;

pub struct ModuleArena {
    bump: Bump,
}

impl ModuleArena {
    pub fn new() -> Self {
        Self { bump: Bump::new() }
    }
    
    pub fn alloc<T>(&self, value: T) -> &mut T {
        self.bump.alloc(value)
    }
}
```

## Testing Strategy

### Validation at Each Phase

1. **Unit Tests**: Test individual components
2. **Integration Tests**: Test component interactions  
3. **Regression Tests**: Ensure no functionality breaks
4. **Performance Tests**: Benchmark against C++
5. **Fuzzing**: Find edge cases
6. **Property Tests**: Validate invariants

## Validation Checklist

Before considering a component complete:

- [ ] All unit tests pass
- [ ] Integration tests pass
- [ ] Benchmarks show acceptable performance
- [ ] Miri clean (no undefined behavior)
- [ ] Clippy clean (no warnings)
- [ ] Documentation complete
- [ ] FFI tested with C++
- [ ] Memory leaks checked
- [ ] Fuzz testing run
- [ ] Code review completed

## References

- [The Rustonomicon](https://doc.rust-lang.org/nomicon/)
- [Rust FFI Omnibus](http://jakegoulding.com/rust-ffi-omnibus/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [WebAssembly Specification](https://webassembly.github.io/spec/)
