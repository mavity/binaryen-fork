use binaryen_core::{Literal, Type};
use binaryen_ir::{BinaryOp, Expression, ExpressionKind, Function, IrBuilder, Module, UnaryOp};
use bumpalo::Bump;
use std::ffi::{c_void, CStr, CString};
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
