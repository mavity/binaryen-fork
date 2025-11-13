# Getting Started with Binaryen Rust Conversion

This guide helps you get started with the Binaryen to Rust conversion effort.

## Prerequisites

### Required Tools

1. **Rust Toolchain** (version 1.70 or later)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup update stable
   ```

2. **Development Tools**
   ```bash
   # Install additional components
   rustup component add clippy rustfmt
   cargo install cargo-miri cargo-fuzz cbindgen
   ```

3. **Existing Build Tools**
   - CMake 3.16+
   - C++17 compiler
   - Python 3.6+

### Verify Installation

```bash
rustc --version    # Should be 1.70+
cargo --version
cmake --version    # Should be 3.16+
```

## Repository Setup

### 1. Clone and Build Baseline

First, ensure the C++ version builds and tests pass:

```bash
git clone https://github.com/mavity/binaryen-fork.git
cd binaryen-fork

# Initialize submodules
git submodule init
git submodule update

# Build C++ version
cmake . && make

# Run tests to establish baseline
./check.py
```

### 2. Create Rust Workspace

```bash
# Create directory structure
mkdir -p rust

# Create workspace Cargo.toml
cat > rust/Cargo.toml << 'EOF'
[workspace]
resolver = "2"

members = [
    "binaryen-support",
    "binaryen-ffi",
]

[workspace.dependencies]
# Common dependencies will go here
EOF
```

### 3. Create First Crate

```bash
# Create support utilities crate
cd rust
cargo new --lib binaryen-support

# Create FFI crate
cargo new --lib binaryen-ffi
```

## First Component: String Interning

Let's start with a simple, self-contained component as a proof of concept.

### Create the Rust Implementation

```rust
// rust/binaryen-support/src/lib.rs
use std::collections::HashMap;
use std::sync::RwLock;

/// String interner for efficient string storage
pub struct StringInterner {
    strings: RwLock<HashMap<String, &'static str>>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            strings: RwLock::new(HashMap::new()),
        }
    }
    
    /// Intern a string, returning a static reference
    pub fn intern(&self, s: &str) -> &'static str {
        // Fast path: check if already interned
        {
            let strings = self.strings.read().unwrap();
            if let Some(&interned) = strings.get(s) {
                return interned;
            }
        }
        
        // Slow path: insert new string
        let mut strings = self.strings.write().unwrap();
        
        // Double-check after acquiring write lock
        if let Some(&interned) = strings.get(s) {
            return interned;
        }
        
        // Leak the string to get a 'static lifetime
        let boxed = Box::new(s.to_string());
        let leaked: &'static str = Box::leak(boxed).as_str();
        strings.insert(s.to_string(), leaked);
        leaked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_intern_same_string() {
        let interner = StringInterner::new();
        let s1 = interner.intern("hello");
        let s2 = interner.intern("hello");
        
        // Should return the same pointer
        assert_eq!(s1.as_ptr(), s2.as_ptr());
    }
    
    #[test]
    fn test_intern_different_strings() {
        let interner = StringInterner::new();
        let s1 = interner.intern("hello");
        let s2 = interner.intern("world");
        
        assert_ne!(s1, s2);
    }
}
```

### Add C FFI Bindings

```rust
// rust/binaryen-ffi/src/lib.rs
use binaryen_support::StringInterner;
use std::os::raw::c_char;
use std::ffi::CStr;

/// Opaque handle to StringInterner
#[repr(C)]
pub struct BinaryenStringInterner {
    _private: [u8; 0],
}

/// Create a new string interner
#[no_mangle]
pub extern "C" fn BinaryenStringInternerCreate() -> *mut BinaryenStringInterner {
    let interner = Box::new(StringInterner::new());
    Box::into_raw(interner) as *mut BinaryenStringInterner
}

/// Destroy a string interner
#[no_mangle]
pub extern "C" fn BinaryenStringInternerDispose(
    interner: *mut BinaryenStringInterner
) {
    if !interner.is_null() {
        unsafe {
            let _ = Box::from_raw(interner as *mut StringInterner);
        }
    }
}

/// Intern a string
#[no_mangle]
pub extern "C" fn BinaryenStringInternerIntern(
    interner: *mut BinaryenStringInterner,
    s: *const c_char,
) -> *const c_char {
    if interner.is_null() || s.is_null() {
        return std::ptr::null();
    }
    
    unsafe {
        let interner = &*(interner as *const StringInterner);
        let c_str = CStr::from_ptr(s);
        
        if let Ok(str_slice) = c_str.to_str() {
            let interned = interner.intern(str_slice);
            interned.as_ptr() as *const c_char
        } else {
            std::ptr::null()
        }
    }
}
```

### Configure Dependencies

```toml
# rust/binaryen-ffi/Cargo.toml
[package]
name = "binaryen-ffi"
version = "0.1.0"
edition = "2021"

[dependencies]
binaryen-support = { path = "../binaryen-support" }

[lib]
crate-type = ["staticlib", "rlib"]
```

### Build and Test

```bash
# From rust/ directory
cargo build
cargo test
cargo clippy
cargo fmt

# Test FFI manually (create a test C program)
cat > test_ffi.c << 'EOF'
#include <stdio.h>
#include <string.h>

// Declare extern functions
extern void* BinaryenStringInternerCreate();
extern void BinaryenStringInternerDispose(void* interner);
extern const char* BinaryenStringInternerIntern(void* interner, const char* s);

int main() {
    void* interner = BinaryenStringInternerCreate();
    
    const char* s1 = BinaryenStringInternerIntern(interner, "hello");
    const char* s2 = BinaryenStringInternerIntern(interner, "hello");
    
    printf("s1: %s\n", s1);
    printf("s2: %s\n", s2);
    printf("Same pointer: %d\n", s1 == s2);
    
    BinaryenStringInternerDispose(interner);
    return 0;
}
EOF

gcc test_ffi.c -L../rust/target/debug -lbinaryen_ffi -o test_ffi
./test_ffi
```

## Integrate with CMake

Create a CMake module to build Rust code:

```cmake
# cmake/BinaryenRust.cmake
option(BUILD_RUST_COMPONENTS "Build Rust components" OFF)

if(BUILD_RUST_COMPONENTS)
    find_program(CARGO_EXECUTABLE cargo)
    
    if(NOT CARGO_EXECUTABLE)
        message(FATAL_ERROR "Cargo not found. Install Rust toolchain.")
    endif()
    
    # Set build type for Cargo
    if(CMAKE_BUILD_TYPE STREQUAL "Debug")
        set(CARGO_BUILD_TYPE "debug")
        set(CARGO_BUILD_FLAG "")
    else()
        set(CARGO_BUILD_TYPE "release")
        set(CARGO_BUILD_FLAG "--release")
    endif()
    
    # Build Rust libraries
    add_custom_target(
        rust_libs ALL
        COMMAND ${CARGO_EXECUTABLE} build ${CARGO_BUILD_FLAG}
        WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}/rust
        COMMENT "Building Rust libraries"
    )
    
    # Add Rust library to link
    set(RUST_LIBS
        ${CMAKE_SOURCE_DIR}/rust/target/${CARGO_BUILD_TYPE}/libbinaryen_ffi.a
    )
endif()
```

## Development Workflow

### Daily Workflow

1. **Write Rust code**
   ```bash
   cd rust/binaryen-support
   # Edit src/lib.rs
   ```

2. **Test frequently**
   ```bash
   cargo test
   cargo clippy
   ```

3. **Check C++ integration**
   ```bash
   cd ../..
   cmake . -DBUILD_RUST_COMPONENTS=ON
   make
   ./check.py
   ```

4. **Commit changes**
   ```bash
   git add rust/
   git commit -m "Add string interning in Rust"
   ```

### Code Review Checklist

Before submitting a PR:

- [ ] `cargo test` passes
- [ ] `cargo clippy` has no warnings
- [ ] `cargo fmt` applied
- [ ] Documentation added (rustdoc)
- [ ] FFI tested if applicable
- [ ] C++ tests still pass
- [ ] Performance benchmarked
- [ ] Code reviewed

## Common Issues and Solutions

### Issue: Linker Errors with Rust Libraries

**Solution**: Make sure you're linking pthread and dl:
```cmake
target_link_libraries(your_target ${RUST_LIBS} pthread dl)
```

### Issue: Cargo Not Found in CI

**Solution**: Install Rust in CI configuration:
```yaml
- name: Install Rust
  run: |
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    echo "$HOME/.cargo/bin" >> $GITHUB_PATH
```

### Issue: Different Behavior in Debug vs Release

**Solution**: Always test both:
```bash
cargo test
cargo test --release
```

## Resources

### Documentation
- [Rust Book](https://doc.rust-lang.org/book/)
- [Rustonomicon (Unsafe Rust)](https://doc.rust-lang.org/nomicon/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [FFI Guide](https://doc.rust-lang.org/nomicon/ffi.html)

### Tools
- [rust-analyzer](https://rust-analyzer.github.io/) - IDE support
- [Clippy](https://github.com/rust-lang/rust-clippy) - Linter
- [Miri](https://github.com/rust-lang/miri) - UB detector

### Community
- [Rust Discord](https://discord.gg/rust-lang)
- [r/rust](https://www.reddit.com/r/rust/)
- [WebAssembly Working Group](https://github.com/rustwasm)

## Next Steps

1. **Complete Phase 0**: Set up full infrastructure
2. **Choose first component**: Pick a utility from Phase 1
3. **Implement with tests**: Write Rust version with comprehensive tests
4. **Integrate**: Add to build system
5. **Validate**: Ensure C++ tests still pass
6. **Document**: Update documentation
7. **Review**: Get code review
8. **Repeat**: Move to next component

## Getting Help

- **Technical questions**: Open an issue with "rust-conversion" label
- **Design discussions**: Start a discussion on GitHub
- **Quick questions**: Ask in project chat/Discord

Good luck with the conversion! 🦀
