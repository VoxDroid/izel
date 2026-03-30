use izel_parser::ast;
use izel_typeck::type_system::{Lifetime, PrimType, Type};
use izel_typeck::TypeChecker;

#[test]
fn with_builtins_registers_core_primitive_types_and_ptr() {
    let mut checker = TypeChecker::with_builtins();

    assert_eq!(checker.resolve_name("i32"), Some(Type::Prim(PrimType::I32)));
    assert_eq!(
        checker.resolve_name("bool"),
        Some(Type::Prim(PrimType::Bool))
    );

    let ptr = checker
        .resolve_name("ptr")
        .expect("ptr built-in should exist");
    assert_eq!(
        ptr,
        Type::Pointer(
            Box::new(Type::Prim(PrimType::Void)),
            false,
            Lifetime::Static
        )
    );
}

#[test]
fn nested_scopes_shadow_and_restore_bindings() {
    let mut checker = TypeChecker::with_builtins();
    checker.define("value".to_string(), Type::Prim(PrimType::I32));
    assert_eq!(
        checker.resolve_name("value"),
        Some(Type::Prim(PrimType::I32))
    );

    checker.push_scope();
    checker.define("value".to_string(), Type::Prim(PrimType::Bool));
    assert_eq!(
        checker.resolve_name("value"),
        Some(Type::Prim(PrimType::Bool))
    );

    checker.pop_scope();
    assert_eq!(
        checker.resolve_name("value"),
        Some(Type::Prim(PrimType::I32))
    );
}

#[test]
fn checking_empty_module_emits_no_diagnostics() {
    let mut checker = TypeChecker::with_builtins();
    let module = ast::Module { items: vec![] };

    checker.check_ast(&module);
    assert!(checker.diagnostics.is_empty());
}
