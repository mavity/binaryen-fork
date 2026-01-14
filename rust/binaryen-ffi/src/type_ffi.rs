//! FFI bindings for Binaryen type system.
//!
//! This module exposes type creation and inspection functions to C/C++ consumers.

use binaryen_core::{type_store, Type};

/// Create an interned function signature type.
///
/// # Arguments
/// * `params` - Type representing parameter types (single or tuple)
/// * `results` - Type representing result types (single or tuple)
///
/// # Returns
/// A Type handle representing the canonicalized signature.
/// Repeated calls with the same params/results yield identical handles.
///
/// # Safety
/// This function is safe - all inputs are Copy types.
#[no_mangle]
pub extern "C" fn BinaryenTypeCreateSignature(params: Type, results: Type) -> Type {
    type_store::intern_signature(params, results)
}

/// Get the parameter types from a signature Type.
///
/// # Arguments
/// * `ty` - Type handle (must be an interned signature)
///
/// # Returns
/// The params Type, or Type::NONE if `ty` is not a signature.
#[no_mangle]
pub extern "C" fn BinaryenTypeGetParams(ty: Type) -> Type {
    type_store::lookup_signature(ty)
        .map(|sig| sig.params)
        .unwrap_or(Type::NONE)
}

/// Get the result types from a signature Type.
///
/// # Arguments
/// * `ty` - Type handle (must be an interned signature)
///
/// # Returns
/// The results Type, or Type::NONE if `ty` is not a signature.
#[no_mangle]
pub extern "C" fn BinaryenTypeGetResults(ty: Type) -> Type {
    type_store::lookup_signature(ty)
        .map(|sig| sig.results)
        .unwrap_or(Type::NONE)
}

/// Get basic Type constant for i32.
#[no_mangle]
pub extern "C" fn BinaryenTypeInt32() -> Type {
    Type::I32
}

/// Get basic Type constant for i64.
#[no_mangle]
pub extern "C" fn BinaryenTypeInt64() -> Type {
    Type::I64
}

/// Get basic Type constant for f32.
#[no_mangle]
pub extern "C" fn BinaryenTypeFloat32() -> Type {
    Type::F32
}

/// Get basic Type constant for f64.
#[no_mangle]
pub extern "C" fn BinaryenTypeFloat64() -> Type {
    Type::F64
}

/// Get basic Type constant for v128.
#[no_mangle]
pub extern "C" fn BinaryenTypeVec128() -> Type {
    Type::V128
}

/// Get the none/void Type.
#[no_mangle]
pub extern "C" fn BinaryenTypeNone() -> Type {
    Type::NONE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_create_signature() {
        let sig_ty = BinaryenTypeCreateSignature(Type::I32, Type::I64);
        assert!(sig_ty.is_signature(), "Should create signature Type");

        let params = BinaryenTypeGetParams(sig_ty);
        let results = BinaryenTypeGetResults(sig_ty);

        assert_eq!(params, Type::I32);
        assert_eq!(results, Type::I64);
    }

    #[test]
    fn test_ffi_signature_interning() {
        let sig1 = BinaryenTypeCreateSignature(Type::F32, Type::F64);
        let sig2 = BinaryenTypeCreateSignature(Type::F32, Type::F64);
        assert_eq!(sig1, sig2, "Same signature should be interned to same ID");
    }

    #[test]
    fn test_ffi_get_params_on_basic_type() {
        let params = BinaryenTypeGetParams(Type::I32);
        assert_eq!(
            params,
            Type::NONE,
            "Basic types should return NONE for params"
        );
    }

    #[test]
    fn test_ffi_basic_type_constants() {
        assert_eq!(BinaryenTypeInt32(), Type::I32);
        assert_eq!(BinaryenTypeFloat64(), Type::F64);
        assert_eq!(BinaryenTypeNone(), Type::NONE);
    }
}
