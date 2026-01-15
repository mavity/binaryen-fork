use binaryen_core::{Literal, Type};
use binaryen_ir::{
    BinaryOp, BinaryReader, BinaryWriter, ExprRef, Expression, Function, IrBuilder, Module,
    PassRunner, UnaryOp,
};
use bumpalo::Bump;
use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::slice;

// Opaque pointer types for C
pub type BinaryenRustModuleRef = *mut WrappedModule;
pub type BinaryenRustExpressionRef = *mut c_void; // Actually &mut Expression

pub struct WrappedModule {
    bump: Box<Bump>,
    module: Module<'static>,
}

/// Creates a new Binaryen module.
///
/// # Safety
/// Returns a raw pointer that must be freed with `BinaryenRustModuleDispose`.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustModuleCreate() -> BinaryenRustModuleRef {
    let bump = Box::new(Bump::new());
    let module = Module::new();

    // Safety: The module will contain references to bump.
    // We treat them as 'static internally but ensure bump lives as long as module.
    let module: Module<'static> = std::mem::transmute(module);

    let wrapper = Box::new(WrappedModule { bump, module });
    Box::into_raw(wrapper)
}

/// Frees a module created with `BinaryenRustModuleCreate`.
///
/// # Safety
/// `module` must be a valid pointer from `BinaryenRustModuleCreate` and must not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustModuleDispose(module: BinaryenRustModuleRef) {
    if !module.is_null() {
        let _ = Box::from_raw(module);
    }
}

// Expression creation

/// Creates a constant i32 expression.
///
/// # Safety
/// `module` must be a valid module pointer. Returns a pointer valid for the module's lifetime.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustConst(
    module: BinaryenRustModuleRef,
    value: i32, // Simplified: assume i32 for now, we should have a generic Literal creator
) -> BinaryenRustExpressionRef {
    let module = &mut *module;
    let builder = IrBuilder::new(&module.bump);
    let expr = builder.const_(Literal::I32(value));

    // Transmute the reference to a raw pointer.
    // The reference is valid as long as `module.bump` is valid.
    expr.as_ptr() as *mut c_void
}

/// Creates a block expression.
///
/// # Safety
/// `module`, `name`, and `children` must be valid pointers. The block name must be null-terminated UTF-8.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustBlock(
    module: BinaryenRustModuleRef,
    name: *const c_char,
    children: *mut BinaryenRustExpressionRef,
    num_children: usize,
    type_: u64, // Type handle
) -> BinaryenRustExpressionRef {
    let module = &mut *module;
    let builder = IrBuilder::new(&module.bump);

    let name = if name.is_null() {
        None
    } else {
        // We probably need to intern string or allocate it in bump
        // For now, let's just duplicate to string and leak/store in bump if we could?
        // But IrBuilder takes &'a str.
        // Simplification: We assume the name string lives long enough OR we copy it to bump.
        // Bumpalo can alloc strings.
        let c_str = CStr::from_ptr(name);
        Some(crate::ir_ffi::bump_string(
            &module.bump,
            c_str.to_str().unwrap(),
        ))
    };

    let children_slice = slice::from_raw_parts(children, num_children);
    let mut vec = bumpalo::collections::Vec::new_in(&module.bump);
    for &child in children_slice {
        let child_ref = ExprRef::new(&mut *(child as *mut Expression));
        vec.push(child_ref);
    }

    // Safe transmutation of Type handle
    let type_ = std::mem::transmute::<u64, Type>(type_);

    let expr = builder.block(name, vec, type_);
    expr.as_ptr() as *mut c_void
}

// Helper to allocate string in bump
fn bump_string<'a>(bump: &'a Bump, s: &str) -> &'a str {
    let dest = bump.alloc_str(s);
    dest
}

// Function creation

/// Adds a function to a module.
///
/// # Safety
/// `module` and `name` must be valid pointers. `name` must be null-terminated UTF-8.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustAddFunction(
    module: BinaryenRustModuleRef,
    name: *const c_char,
    params: u64,
    results: u64,
    body: BinaryenRustExpressionRef,
) {
    let module_wrapper = &mut *module;
    let name_str = CStr::from_ptr(name).to_string_lossy().into_owned();

    let params = std::mem::transmute::<u64, Type>(params);
    let results = std::mem::transmute::<u64, Type>(results);

    let body_ref = if body.is_null() {
        None
    } else {
        Some(ExprRef::new(&mut *(body as *mut Expression)))
    };

    let func = Function::new(
        name_str,
        params,
        results,
        vec![], // vars TODO
        body_ref,
    );

    // We need to transmute func lifetime to 'static to match module's expected lifetime
    let func_static: Function<'static> = std::mem::transmute(func);

    module_wrapper.module.add_function(func_static);
}

/// Creates a unary expression.
///
/// # Safety
/// `module` and `value` must be valid pointers. `op` must be a valid UnaryOp discriminant.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustUnary(
    module: BinaryenRustModuleRef,
    op: u32,
    value: BinaryenRustExpressionRef,
    type_: u64,
) -> BinaryenRustExpressionRef {
    let module = &mut *module;
    let builder = IrBuilder::new(&module.bump);

    // Safety: we assume op index is valid. A real implementation would validate.
    let op: UnaryOp = std::mem::transmute(op);
    let value_ref = ExprRef::new(&mut *(value as *mut Expression));
    let type_ = std::mem::transmute::<u64, Type>(type_);

    let expr = builder.unary(op, value_ref, type_);
    expr.as_ptr() as *mut c_void
}

/// Creates a binary expression.
///
/// # Safety
/// `module`, `left`, and `right` must be valid pointers. `op` must be a valid BinaryOp discriminant.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustBinary(
    module: BinaryenRustModuleRef,
    op: u32,
    left: BinaryenRustExpressionRef,
    right: BinaryenRustExpressionRef,
    type_: u64,
) -> BinaryenRustExpressionRef {
    let module = &mut *module;
    let builder = IrBuilder::new(&module.bump);

    let op: BinaryOp = std::mem::transmute(op);
    let left_ref = ExprRef::new(&mut *(left as *mut Expression));
    let right_ref = ExprRef::new(&mut *(right as *mut Expression));
    let type_ = std::mem::transmute::<u64, Type>(type_);

    let expr = builder.binary(op, left_ref, right_ref, type_);
    expr.as_ptr() as *mut c_void
}

/// Creates a local.get expression.
///
/// # Safety
/// `module` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustLocalGet(
    module: BinaryenRustModuleRef,
    index: u32,
    type_: u64,
) -> BinaryenRustExpressionRef {
    let module = &mut *module;
    let builder = IrBuilder::new(&module.bump);
    let type_ = std::mem::transmute::<u64, Type>(type_);

    let expr = builder.local_get(index, type_);
    expr.as_ptr() as *mut c_void
}

/// Creates a local.set expression.
///
/// # Safety
/// `module` and `value` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustLocalSet(
    module: BinaryenRustModuleRef,
    index: u32,
    value: BinaryenRustExpressionRef,
) -> BinaryenRustExpressionRef {
    let module = &mut *module;
    let builder = IrBuilder::new(&module.bump);
    let value_ref = ExprRef::new(&mut *(value as *mut Expression));

    let expr = builder.local_set(index, value_ref);
    expr.as_ptr() as *mut c_void
}

/// Creates an atomic RMW expression.
///
/// # Safety
/// `module`, `ptr`, and `value` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustAtomicRMW(
    module: BinaryenRustModuleRef,
    op: u32,
    bytes: u32,
    offset: u32,
    ptr: BinaryenRustExpressionRef,
    value: BinaryenRustExpressionRef,
    type_: u64,
) -> BinaryenRustExpressionRef {
    let module = &mut *module;
    let builder = IrBuilder::new(&module.bump);
    let op = std::mem::transmute::<u32, binaryen_ir::ops::AtomicOp>(op);
    let ptr_ref = ExprRef::new(&mut *(ptr as *mut Expression));
    let value_ref = ExprRef::new(&mut *(value as *mut Expression));
    let type_ = std::mem::transmute::<u64, Type>(type_);

    let expr = builder.atomic_rmw(op, bytes, offset, ptr_ref, value_ref, type_);
    expr.as_ptr() as *mut c_void
}

/// Creates a memory.init expression.
///
/// # Safety
/// `module`, `dest`, `offset`, and `size` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustMemoryInit(
    module: BinaryenRustModuleRef,
    segment: u32,
    dest: BinaryenRustExpressionRef,
    offset: BinaryenRustExpressionRef,
    size: BinaryenRustExpressionRef,
) -> BinaryenRustExpressionRef {
    let module = &mut *module;
    let builder = IrBuilder::new(&module.bump);
    let dest_ref = ExprRef::new(&mut *(dest as *mut Expression));
    let offset_ref = ExprRef::new(&mut *(offset as *mut Expression));
    let size_ref = ExprRef::new(&mut *(size as *mut Expression));

    let expr = builder.memory_init(segment, dest_ref, offset_ref, size_ref);
    expr.as_ptr() as *mut c_void
}

// Binary I/O functions

/// Parses a WebAssembly binary and creates a module.
///
/// # Safety
/// `bytes` must be a valid pointer to at least `len` bytes. Returns null on parse failure.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustModuleReadBinary(
    bytes: *const u8,
    len: usize,
) -> BinaryenRustModuleRef {
    let data = slice::from_raw_parts(bytes, len).to_vec();
    let bump = Box::new(Bump::new());

    // Parse the module
    let bump_ref: &'static Bump = std::mem::transmute(bump.as_ref());
    let mut reader = BinaryReader::new(bump_ref, data);

    match reader.parse_module() {
        Ok(module) => {
            let module: Module<'static> = std::mem::transmute(module);
            let wrapper = Box::new(WrappedModule { bump, module });
            Box::into_raw(wrapper)
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Serializes a module to WebAssembly binary format.
///
/// # Safety
/// All pointers must be valid. The output buffer must be freed with `BinaryenRustModuleFreeBinary`.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustModuleWriteBinary(
    module: BinaryenRustModuleRef,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if module.is_null() || out_ptr.is_null() || out_len.is_null() {
        return -1;
    }

    let module_ref = &(*module).module;
    let mut writer = BinaryWriter::new();

    match writer.write_module(module_ref) {
        Ok(bytes) => {
            let len = bytes.len();
            let ptr = Box::into_raw(bytes.into_boxed_slice()) as *mut u8;
            *out_ptr = ptr;
            *out_len = len;
            0 // Success
        }
        Err(_) => -1,
    }
}

/// Frees a binary buffer allocated by `BinaryenRustModuleWriteBinary`.
///
/// # Safety
/// `ptr` must be from `BinaryenRustModuleWriteBinary` with the correct `len`.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustModuleFreeBinary(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len));
    }
}

// Pass management

/// Runs optimization passes on a module.
///
/// # Safety
/// `module` and `pass_names` must be valid pointers. Each pass name must be null-terminated UTF-8.
#[no_mangle]
pub unsafe extern "C" fn BinaryenRustModuleRunPasses(
    module: BinaryenRustModuleRef,
    pass_names: *const *const c_char,
    num_passes: usize,
) -> i32 {
    if module.is_null() {
        return -1;
    }

    let module_ref = &mut (*module).module;
    let mut runner = PassRunner::new();

    // Parse pass names
    for i in 0..num_passes {
        let name_ptr = *pass_names.add(i);
        if name_ptr.is_null() {
            continue;
        }

        let name = CStr::from_ptr(name_ptr).to_str().unwrap_or("");

        match name {
            "simplify-identity" => {
                runner.add(binaryen_ir::passes::simplify_identity::SimplifyIdentity);
            }
            "dce" => {
                runner.add(binaryen_ir::passes::dce::DCE);
            }
            _ => {} // Unknown pass, skip
        }
    }

    runner.run(module_ref);
    0 // Success
}
