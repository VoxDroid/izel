use izel_lexer::{Lexer, TokenKind};
use izel_parser::ast;
use izel_span::SourceId;
use izel_typeck::type_system::{Effect, EffectSet, Lifetime, PrimType, Type};
use izel_typeck::TypeChecker;

fn parse_module(source: &str) -> ast::Module {
    let mut lexer = Lexer::new(source, SourceId(0));
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next_token();
        let kind = token.kind;
        tokens.push(token);
        if kind == TokenKind::Eof {
            break;
        }
    }

    let mut parser = izel_parser::Parser::new(tokens, source.to_string());
    parser.source = source.to_string();
    let cst = parser.parse_source_file();
    let lowerer = izel_ast_lower::Lowerer::new(source);
    lowerer.lower_module(&cst)
}

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

#[test]
fn check_project_collects_drawn_module_signatures() {
    let main = parse_module(
        r#"
draw std/io;
forge main() -> i32 {
    helper()
}
"#,
    );

    let imported = parse_module(
        r#"
forge helper() -> i32 {
    7
}
"#,
    );

    let mut checker = TypeChecker::with_builtins();
    let mut others = std::collections::HashMap::new();
    others.insert("std/io".to_string(), imported);

    checker.check_project(&main, others);

    assert!(
        checker.resolve_name("helper").is_some(),
        "drawn module symbol should be imported into the current scope"
    );
    assert!(checker.handled_modules.contains("std/io"));
    assert!(
        checker.diagnostics.is_empty(),
        "draw/import flow should remain diagnostic-free: {:?}",
        checker.diagnostics
    );
}

#[test]
fn branch_effects_merge_when_forge_declares_union_of_arm_effects() {
    let module = parse_module(
        r#"
forge io_call() -> void !io {}
forge net_call() -> void !net {}

forge choose(flag: bool) -> void !io !net {
    branch flag {
        true => io_call(),
        false => net_call(),
    }
}
"#,
    );

    let mut checker = TypeChecker::with_builtins();
    checker.check_ast(&module);

    assert!(
        checker.diagnostics.is_empty(),
        "branch arm effects should merge into declared forge effects: {:?}",
        checker.diagnostics
    );
}

#[test]
fn pure_effect_expectation_rejects_io_effect_set() {
    let mut checker = TypeChecker::new();
    let actual = EffectSet::Concrete(vec![Effect::IO]);
    let declared_pure = EffectSet::Concrete(vec![Effect::Pure]);

    assert!(
        !checker.unify_effects(&actual, &declared_pure),
        "effect unification should reject io where pure is required"
    );
}
