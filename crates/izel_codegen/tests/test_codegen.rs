use inkwell::context::Context;
use izel_codegen::{llvm_type_static, Codegen};
use izel_typeck::type_system::{PrimType, Type};

#[test]
fn llvm_type_static_maps_i32_to_32_bit_int() {
    let context = Context::create();
    let ty = llvm_type_static(&context, &Type::Prim(PrimType::I32)).expect("i32 should map");

    assert!(ty.is_int_type());
    assert_eq!(ty.into_int_type().get_bit_width(), 32);
}

#[test]
fn llvm_type_static_rejects_void_basic_type() {
    let context = Context::create();
    let err =
        llvm_type_static(&context, &Type::Prim(PrimType::Void)).expect_err("void should error");
    assert!(err.to_string().contains("void"));
}

#[test]
fn codegen_emits_named_module_ir() {
    let context = Context::create();
    let codegen = Codegen::new(&context, "integration_codegen", "");
    let ir = codegen.emit_llvm_ir();

    assert!(ir.contains("ModuleID = 'integration_codegen'"));
}
