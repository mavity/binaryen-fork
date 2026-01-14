use binaryen_core::{Literal, Type};
use binaryen_ir::{
    BinaryOp, BinaryReader, BinaryWriter, Expression, Function, IrBuilder, Module, Pass,
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

#[no_mangle]
pub unsafe extern "C" fn BinaryenRustModuleDispose(module: BinaryenRustModuleRef) {
    if !module.is_null() {
        let _ = Box::from_raw(module);
    }
}

// Expression creation
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
    expr as *mut Expression as *mut c_void
}

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
        let child_ref = &mut *(child as *mut Expression);
        vec.push(child_ref);
    }

    // Safe transmutation of Type handle
    let type_ = std::mem::transmute::<u64, Type>(type_);

    let expr = builder.block(name, vec, type_);
    expr as *mut Expression as *mut c_void
}

// Helper to allocate string in bump
fn bump_string<'a>(bump: &'a Bump, s: &str) -> &'a str {
    let dest = bump.alloc_str(s);
    dest
}

// Function creation
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
        Some(&mut *(body as *mut Expression))
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
    let value_ref = &mut *(value as *mut Expression);
    let type_ = std::mem::transmute::<u64, Type>(type_);

    let expr = builder.unary(op, value_ref, type_);
    expr as *mut Expression as *mut c_void
}

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
    let left_ref = &mut *(left as *mut Expression);
    let right_ref = &mut *(right as *mut Expression);
    let type_ = std::mem::transmute::<u64, Type>(type_);

    let expr = builder.binary(op, left_ref, right_ref, type_);
    expr as *mut Expression as *mut c_void
}

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
    expr as *mut Expression as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn BinaryenRustLocalSet(
    module: BinaryenRustModuleRef,
    index: u32,
    value: BinaryenRustExpressionRef,
) -> BinaryenRustExpressionRef {
    let module = &mut *module;
    let builder = IrBuilder::new(&module.bump);
    let value_ref = &mut *(value as *mut Expression);

    let expr = builder.local_set(index, value_ref);
    expr as *mut Expression as *mut c_void
}

// Binary I/O functions

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

#[no_mangle]
pub unsafe extern "C" fn BinaryenRustModuleFreeBinary(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let _ = Box::from_raw(slice::from_raw_parts_mut(ptr, len));
    }
}

// Pass management

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
