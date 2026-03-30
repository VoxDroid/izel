use izel_lexer::{Lexer, TokenKind};
use izel_parser::ast;
use izel_span::{SourceId, Span};
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

fn assert_has_diagnostic(checker: &TypeChecker, needle: &str) {
    let messages: Vec<String> = checker
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert!(
        messages.iter().any(|m| m.contains(needle)),
        "expected diagnostic containing '{needle}', got: {messages:?}"
    );
}

fn assert_no_diagnostic(checker: &TypeChecker, needle: &str) {
    let messages: Vec<String> = checker
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert!(
        messages.iter().all(|m| !m.contains(needle)),
        "did not expect diagnostic containing '{needle}', got: {messages:?}"
    );
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

#[test]
fn dual_missing_decode_is_reported() {
    let encode = ast::Forge {
        name: "encode".to_string(),
        name_span: Span::dummy(),
        visibility: ast::Visibility::Open,
        is_flow: false,
        generic_params: vec![],
        params: vec![],
        ret_type: ast::Type::Prim("i32".to_string()),
        effects: vec![],
        attributes: vec![],
        requires: vec![],
        ensures: vec![],
        body: None,
        span: Span::dummy(),
    };

    let module = ast::Module {
        items: vec![ast::Item::Dual(ast::Dual {
            name: "Codec".to_string(),
            visibility: ast::Visibility::Open,
            generic_params: vec![],
            items: vec![ast::Item::Forge(encode)],
            attributes: vec![],
            span: Span::dummy(),
        })],
    };

    let mut checker = TypeChecker::with_builtins();
    checker.check_ast(&module);

    assert_has_diagnostic(
        &checker,
        "must define or elaborate both encode and decode forges",
    );
}

#[test]
fn shape_derive_diagnostics_cover_empty_invalid_and_unsupported_targets() {
    let module = parse_module(
        r#"
@derive()
shape Empty {}

@derive(1)
shape Invalid {}

@derive(NotReal)
shape Unsupported {}
"#,
    );

    let mut checker = TypeChecker::with_builtins();
    checker.check_ast(&module);

    assert_has_diagnostic(&checker, "requires at least one derive target");
    assert_has_diagnostic(&checker, "invalid derive target");
    assert_has_diagnostic(&checker, "unsupported built-in derive");
}

#[test]
fn forge_attribute_macro_diagnostics_cover_invalid_forms() {
    let module = parse_module(
        r#"
@test(1)
forge bad_test() {}

@bench(1)
forge bad_bench() {}

@test
@bench
forge both() {}

@inline(always, never)
forge too_many_inline_args() {}

@inline(maybe)
forge bad_inline_mode() {}
"#,
    );

    let mut checker = TypeChecker::with_builtins();
    checker.check_ast(&module);

    assert_has_diagnostic(&checker, "invalid #[test] usage");
    assert_has_diagnostic(&checker, "invalid #[bench] usage");
    assert_has_diagnostic(&checker, "cannot use #[test] and #[bench] together");
    assert_has_diagnostic(&checker, "invalid #[inline] usage");
    assert_has_diagnostic(&checker, "invalid #[inline] mode");
}

#[test]
fn non_forge_attribute_macro_on_shape_is_rejected() {
    let module = parse_module(
        r#"
@test
shape InvalidUse {}
"#,
    );

    let mut checker = TypeChecker::with_builtins();
    checker.check_ast(&module);

    assert_has_diagnostic(
        &checker,
        "attribute macro #[test] can only be applied to forge declarations",
    );
}

#[test]
fn bridge_validation_reports_abi_and_item_rules() {
    let module = parse_module(
        r#"
bridge "Rust" {
    forge ext() {}
    static value: i32 = 1
    shape NotAllowed {}
}

bridge {
    forge missing_abi()
}
"#,
    );

    let mut checker = TypeChecker::with_builtins();
    checker.check_ast(&module);

    assert_has_diagnostic(&checker, "bridge ABI 'Rust' is not supported");
    assert_has_diagnostic(&checker, "must be a declaration without a body");
    assert_has_diagnostic(&checker, "cannot define an initializer");
    assert_has_diagnostic(
        &checker,
        "bridge blocks may only contain forge and static declarations",
    );
    assert_has_diagnostic(&checker, "requires an explicit ABI string");
}

#[test]
fn asm_validation_reports_non_raw_and_missing_template() {
    let module = parse_module(
        r#"
forge main() {
    asm!()
}
"#,
    );

    let mut checker = TypeChecker::with_builtins();
    checker.check_ast(&module);

    assert_has_diagnostic(&checker, "asm! is only allowed inside raw blocks");
    assert_has_diagnostic(
        &checker,
        "asm! requires at least a template string argument",
    );
}

#[test]
fn asm_validation_reports_non_string_template_inside_raw() {
    let module = parse_module(
        r#"
forge main() {
    raw { asm!(1) }
}
"#,
    );

    let mut checker = TypeChecker::with_builtins();
    checker.check_ast(&module);

    assert_has_diagnostic(
        &checker,
        "asm! first argument must be a string literal template",
    );
}

#[test]
fn echo_validation_reports_non_const_and_non_ident_pattern_cases() {
    let module = parse_module(
        r#"
forge side() -> void !io {}

echo {
    let x
    let _ = 1
    let y = side()
    side()
    y
}
"#,
    );

    let mut checker = TypeChecker::with_builtins();
    checker.check_ast(&module);

    assert_has_diagnostic(&checker, "echo let binding requires an initializer");
    assert_has_diagnostic(
        &checker,
        "echo let bindings currently require identifier patterns",
    );
    assert_has_diagnostic(&checker, "echo initializer is not compile-time evaluable");
    assert_has_diagnostic(&checker, "echo statement is not compile-time evaluable");
    assert_has_diagnostic(
        &checker,
        "echo trailing expression is not compile-time evaluable",
    );
}

#[test]
fn echo_validation_reports_purity_for_effectful_calls() {
    let mut checker = TypeChecker::with_builtins();
    checker.define(
        "log".to_string(),
        Type::Function {
            params: vec![],
            ret: Box::new(Type::Prim(PrimType::Void)),
            effects: EffectSet::Concrete(vec![Effect::IO]),
        },
    );

    let module = ast::Module {
        items: vec![ast::Item::Echo(ast::Echo {
            body: ast::Block {
                stmts: vec![ast::Stmt::Expr(ast::Expr::Call(
                    Box::new(ast::Expr::Ident("log".to_string(), Span::dummy())),
                    vec![],
                ))],
                expr: None,
                span: Span::dummy(),
            },
            attributes: vec![],
            span: Span::dummy(),
        })],
    };

    checker.check_ast(&module);
    assert_has_diagnostic(
        &checker,
        "echo block must be pure and cannot use runtime effects",
    );
}

#[test]
fn check_forge_reports_ensures_violation_for_const_return() {
    let module = parse_module(
        r#"
@ensures(result > 0)
forge bad_ensures() -> i32 {
    0
}
"#,
    );

    let mut checker = TypeChecker::with_builtins();
    checker.check_ast(&module);

    assert_has_diagnostic(&checker, "postcondition violation");
}

#[test]
fn error_attribute_is_rejected_on_all_non_scroll_item_kinds() {
    let module = parse_module(
        r#"
@error
weave W {}

@error
ward Bag {}

@error
static x: i32 = 1

@error
forge f() {}

@error
shape S {}

@error
dual shape D {}

@error
type A = i32

@error
echo { 1 }

@error
bridge "C" {}
"#,
    );

    let mut checker = TypeChecker::with_builtins();
    checker.check_ast(&module);

    assert_has_diagnostic(&checker, "found on weave");
    assert_has_diagnostic(&checker, "found on ward 'Bag'");
    assert_has_diagnostic(&checker, "found on static 'x'");
    assert_has_diagnostic(&checker, "found on forge 'f'");
    assert_has_diagnostic(&checker, "found on shape 'S'");
    assert_has_diagnostic(&checker, "found on dual 'D'");
    assert_has_diagnostic(&checker, "found on alias 'A'");
}

#[test]
fn error_attribute_on_echo_and_bridge_is_rejected() {
    let error_attr = ast::Attribute {
        name: "error".to_string(),
        args: vec![],
        span: Span::dummy(),
    };

    let module = ast::Module {
        items: vec![
            ast::Item::Echo(ast::Echo {
                body: ast::Block {
                    stmts: vec![],
                    expr: Some(Box::new(ast::Expr::Literal(ast::Literal::Int(1)))),
                    span: Span::dummy(),
                },
                attributes: vec![error_attr.clone()],
                span: Span::dummy(),
            }),
            ast::Item::Bridge(ast::Bridge {
                abi: Some("C".to_string()),
                items: vec![],
                attributes: vec![error_attr],
                span: Span::dummy(),
            }),
        ],
    };

    let mut checker = TypeChecker::with_builtins();
    checker.check_ast(&module);

    assert_has_diagnostic(&checker, "found on echo block");
    assert_has_diagnostic(&checker, "found on bridge block");
}

#[test]
fn effect_boundary_invalid_arg_is_reported_and_valid_effect_still_masks() {
    let module = parse_module(
        r#"
@effect_boundary(io, 1)
forge wrapped() -> void !io {}

forge caller() -> void {
    wrapped()
}
"#,
    );

    let mut checker = TypeChecker::with_builtins();
    checker.check_ast(&module);

    assert_has_diagnostic(&checker, "invalid #[effect_boundary] argument");
    assert_no_diagnostic(&checker, "Function has effects");
}

#[test]
fn impl_duplicate_and_orphan_paths_are_exercised() {
    let weave_item = ast::Item::Weave(ast::Weave {
        name: "LocalWeave".to_string(),
        visibility: ast::Visibility::Open,
        parents: vec![],
        associated_types: vec![],
        methods: vec![],
        attributes: vec![],
        span: Span::dummy(),
    });

    let local_impl = ast::Item::Impl(ast::Impl {
        target: ast::Type::Prim("i32".to_string()),
        weave: Some(ast::Type::Prim("LocalWeave".to_string())),
        items: vec![],
        attributes: vec![],
        span: Span::dummy(),
    });

    let duplicate_local_impl = ast::Item::Impl(ast::Impl {
        target: ast::Type::Prim("i32".to_string()),
        weave: Some(ast::Type::Prim("LocalWeave".to_string())),
        items: vec![],
        attributes: vec![],
        span: Span::dummy(),
    });

    let module = ast::Module {
        items: vec![weave_item, local_impl, duplicate_local_impl],
    };

    let mut checker = TypeChecker::with_builtins();
    checker.check_ast(&module);

    let local_impl_count = checker
        .trait_impls
        .get("LocalWeave")
        .map(|v| v.len())
        .unwrap_or(0);
    assert_eq!(local_impl_count, 1, "duplicate impl should not be inserted");

    let orphan_module = ast::Module {
        items: vec![ast::Item::Impl(ast::Impl {
            target: ast::Type::Prim("i32".to_string()),
            weave: Some(ast::Type::Prim("ForeignWeave".to_string())),
            items: vec![],
            attributes: vec![],
            span: Span::dummy(),
        })],
    };

    let mut orphan_checker = TypeChecker::with_builtins();
    orphan_checker.check_ast(&orphan_module);
    assert!(!orphan_checker.trait_impls.contains_key("ForeignWeave"));
}
