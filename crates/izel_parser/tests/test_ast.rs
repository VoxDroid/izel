use izel_parser::ast::{self, AlphaEq};
use izel_span::Span;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn span() -> Span {
    Span::dummy()
}

fn ident(name: &str) -> ast::Expr {
    ast::Expr::Ident(name.to_string(), span())
}

fn int_lit(value: i128) -> ast::Expr {
    ast::Expr::Literal(ast::Literal::Int(value))
}

#[test]
fn literal_float_equality_and_hash_follow_bit_pattern() {
    let a = ast::Literal::Float(f64::from_bits(0x7ff8_0000_0000_0001));
    let b = ast::Literal::Float(f64::from_bits(0x7ff8_0000_0000_0001));
    let c = ast::Literal::Float(f64::from_bits(0x7ff8_0000_0000_0002));

    assert_eq!(a, b);
    assert_ne!(a, c);

    let mut hash_a = DefaultHasher::new();
    a.hash(&mut hash_a);
    let mut hash_b = DefaultHasher::new();
    b.hash(&mut hash_b);

    assert_eq!(hash_a.finish(), hash_b.finish());
}

#[test]
fn type_alpha_eq_covers_function_pointer_and_witness_shapes() {
    let fn_type = ast::Type::Function {
        params: vec![
            ast::Type::Optional(Box::new(ast::Type::Prim("i32".to_string()))),
            ast::Type::Pointer(Box::new(ast::Type::Prim("u8".to_string())), true),
            ast::Type::Witness(Box::new(ast::GenericArg::Expr(ast::Expr::Binary(
                ast::BinaryOp::Gt,
                Box::new(ident("n")),
                Box::new(int_lit(0)),
            )))),
        ],
        ret: Box::new(ast::Type::Cascade(Box::new(ast::Type::Path(
            vec!["Result".to_string()],
            vec![ast::GenericArg::Type(ast::Type::Prim("i32".to_string()))],
        )))),
        effects: vec!["io".to_string(), "net".to_string()],
    };

    let same = fn_type.clone();
    assert!(fn_type.alpha_eq(&same));

    let mut changed_effects = same.clone();
    if let ast::Type::Function { effects, .. } = &mut changed_effects {
        effects.pop();
    }
    assert!(!fn_type.alpha_eq(&changed_effects));

    assert!(ast::Type::SelfType.alpha_eq(&ast::Type::SelfType));
    assert!(ast::Type::Error.alpha_eq(&ast::Type::Error));
}

#[test]
fn pattern_alpha_eq_covers_structural_patterns() {
    let pat = ast::Pattern::Struct {
        path: ast::Type::Path(vec!["Node".to_string()], vec![]),
        fields: vec![
            (
                "head".to_string(),
                ast::Pattern::Tuple(vec![
                    ast::Pattern::Ident("x".to_string(), false, span()),
                    ast::Pattern::Wildcard,
                ]),
            ),
            (
                "tail".to_string(),
                ast::Pattern::Slice(vec![ast::Pattern::Rest("rest".to_string())]),
            ),
        ],
    };

    assert!(pat.alpha_eq(&pat.clone()));

    let changed = ast::Pattern::Struct {
        path: ast::Type::Path(vec!["Node".to_string()], vec![]),
        fields: vec![
            (
                "head".to_string(),
                ast::Pattern::Tuple(vec![ast::Pattern::Ident("x".to_string(), true, span())]),
            ),
            (
                "tail".to_string(),
                ast::Pattern::Slice(vec![ast::Pattern::Rest("rest".to_string())]),
            ),
        ],
    };

    assert!(!pat.alpha_eq(&changed));

    let alt = ast::Pattern::Or(vec![
        ast::Pattern::Variant(
            "Some".to_string(),
            vec![ast::Pattern::Ident("x".to_string(), false, span())],
        ),
        ast::Pattern::Literal(ast::Literal::Nil),
    ]);
    let alt_short = ast::Pattern::Or(vec![ast::Pattern::Variant("Some".to_string(), vec![])]);

    assert!(alt.alpha_eq(&alt.clone()));
    assert!(!alt.alpha_eq(&alt_short));
}

#[test]
fn expr_alpha_eq_handles_optional_control_flow_paths() {
    let arm = ast::Arm {
        pattern: ast::Pattern::Ident("v".to_string(), false, span()),
        guard: Some(ast::Expr::Binary(
            ast::BinaryOp::And,
            Box::new(ast::Expr::Binary(
                ast::BinaryOp::Gt,
                Box::new(ident("v")),
                Box::new(int_lit(0)),
            )),
            Box::new(ast::Expr::Binary(
                ast::BinaryOp::Lt,
                Box::new(ident("v")),
                Box::new(int_lit(10)),
            )),
        )),
        body: ident("v"),
        span: span(),
    };

    let given = ast::Expr::Given {
        cond: Box::new(ast::Expr::Literal(ast::Literal::Bool(true))),
        then_block: ast::Block {
            stmts: vec![ast::Stmt::Expr(ast::Expr::Branch {
                target: Box::new(ident("x")),
                arms: vec![arm],
            })],
            expr: Some(Box::new(int_lit(1))),
            span: span(),
        },
        else_expr: Some(Box::new(int_lit(0))),
    };

    let cascade_with_context = ast::Expr::Cascade {
        expr: Box::new(given.clone()),
        context: Some(Box::new(ident("ctx"))),
    };
    let cascade_without_context = ast::Expr::Cascade {
        expr: Box::new(given.clone()),
        context: None,
    };

    let seek_with_catch = ast::Expr::Seek {
        body: ast::Block {
            stmts: vec![ast::Stmt::Expr(cascade_with_context.clone())],
            expr: Some(Box::new(ast::Expr::Tide(Box::new(int_lit(5))))),
            span: span(),
        },
        catch_var: Some("err".to_string()),
        catch_body: Some(ast::Block {
            stmts: vec![ast::Stmt::Expr(ast::Expr::Break)],
            expr: Some(Box::new(ast::Expr::Next)),
            span: span(),
        }),
    };

    let seek_without_catch = ast::Expr::Seek {
        body: ast::Block {
            stmts: vec![ast::Stmt::Expr(cascade_with_context)],
            expr: Some(Box::new(ast::Expr::Tide(Box::new(int_lit(5))))),
            span: span(),
        },
        catch_var: None,
        catch_body: None,
    };

    assert!(seek_with_catch.alpha_eq(&seek_with_catch.clone()));
    assert!(!seek_with_catch.alpha_eq(&seek_without_catch));
    assert!(!cascade_without_context.alpha_eq(&given));
}

#[test]
fn expr_alpha_eq_covers_call_bind_block_zone_and_witness_new() {
    let call = ast::Expr::Call(
        Box::new(ast::Expr::Member(
            Box::new(ast::Expr::Path(
                vec!["math".to_string(), "sum".to_string()],
                vec![ast::GenericArg::Type(ast::Type::Prim("i32".to_string()))],
            )),
            "apply".to_string(),
            span(),
        )),
        vec![
            ast::Arg {
                label: Some("x".to_string()),
                value: int_lit(1),
                span: span(),
            },
            ast::Arg {
                label: None,
                value: ast::Expr::Unary(ast::UnaryOp::Neg, Box::new(int_lit(2))),
                span: span(),
            },
        ],
    );

    let bind = ast::Expr::Bind {
        params: vec!["x".to_string()],
        body: Box::new(call),
    };

    let struct_lit = ast::Expr::StructLiteral {
        path: ast::Type::Path(vec!["Point".to_string()], vec![]),
        fields: vec![
            ("x".to_string(), int_lit(1)),
            ("y".to_string(), ast::Expr::Raw(Box::new(int_lit(2)))),
            (
                "proof".to_string(),
                ast::Expr::WitnessNew(Box::new(ast::GenericArg::Type(ast::Type::Path(
                    vec!["NonZero".to_string()],
                    vec![ast::GenericArg::Type(ast::Type::Prim("i32".to_string()))],
                )))),
            ),
        ],
    };

    let zone_expr = ast::Expr::Zone {
        name: "arena".to_string(),
        body: ast::Block {
            stmts: vec![ast::Stmt::Let {
                pat: ast::Pattern::Ident("p".to_string(), false, span()),
                ty: Some(ast::Type::Prim("Point".to_string())),
                init: Some(struct_lit),
                span: span(),
            }],
            expr: Some(Box::new(ast::Expr::Return(Box::new(bind)))),
            span: span(),
        },
    };

    let zone_expr_same = zone_expr.clone();
    let zone_expr_other_name = ast::Expr::Zone {
        name: "other".to_string(),
        body: if let ast::Expr::Zone { body, .. } = &zone_expr {
            body.clone()
        } else {
            unreachable!()
        },
    };

    assert!(zone_expr.alpha_eq(&zone_expr_same));
    assert!(!zone_expr.alpha_eq(&zone_expr_other_name));
}

#[test]
fn loop_while_each_and_arm_alpha_eq_are_stable() {
    let loop_expr = ast::Expr::Loop(ast::Block {
        stmts: vec![ast::Stmt::Expr(ast::Expr::Next)],
        expr: None,
        span: span(),
    });

    let while_expr = ast::Expr::While {
        cond: Box::new(ast::Expr::Unary(ast::UnaryOp::Not, Box::new(ident("done")))),
        body: ast::Block {
            stmts: vec![ast::Stmt::Expr(ast::Expr::Break)],
            expr: None,
            span: span(),
        },
    };

    let each_expr = ast::Expr::Each {
        var: "item".to_string(),
        iter: Box::new(ast::Expr::Path(
            vec!["items".to_string()],
            vec![ast::GenericArg::Expr(int_lit(0))],
        )),
        body: ast::Block {
            stmts: vec![],
            expr: Some(Box::new(ident("item"))),
            span: span(),
        },
    };

    let arm = ast::Arm {
        pattern: ast::Pattern::Variant(
            "Some".to_string(),
            vec![ast::Pattern::Ident("v".to_string(), false, span())],
        ),
        guard: Some(ast::Expr::Literal(ast::Literal::Bool(true))),
        body: ident("v"),
        span: span(),
    };

    assert!(loop_expr.alpha_eq(&loop_expr.clone()));
    assert!(while_expr.alpha_eq(&while_expr.clone()));
    assert!(each_expr.alpha_eq(&each_expr.clone()));
    assert!(arm.alpha_eq(&arm.clone()));

    let each_other = ast::Expr::Each {
        var: "value".to_string(),
        iter: Box::new(ast::Expr::Path(
            vec!["items".to_string()],
            vec![ast::GenericArg::Expr(int_lit(0))],
        )),
        body: ast::Block {
            stmts: vec![],
            expr: Some(Box::new(ident("item"))),
            span: span(),
        },
    };
    assert!(!each_expr.alpha_eq(&each_other));
}

#[test]
fn literal_hash_and_equality_cover_all_variant_paths() {
    let mut int_hash = DefaultHasher::new();
    ast::Literal::Int(7).hash(&mut int_hash);

    let mut str_hash = DefaultHasher::new();
    ast::Literal::Str("izel".to_string()).hash(&mut str_hash);

    let mut bool_hash = DefaultHasher::new();
    ast::Literal::Bool(true).hash(&mut bool_hash);

    let mut nil_hash = DefaultHasher::new();
    ast::Literal::Nil.hash(&mut nil_hash);

    assert_eq!(
        ast::Literal::Str("same".to_string()),
        ast::Literal::Str("same".to_string())
    );
    assert_ne!(
        ast::Literal::Str("same".to_string()),
        ast::Literal::Str("different".to_string())
    );

    assert_ne!(
        ast::Literal::Str("same".to_string()),
        ast::Literal::Bool(true)
    );
}

#[test]
fn alpha_eq_option_and_mismatch_paths_are_exercised() {
    let block_none = ast::Block {
        stmts: vec![],
        expr: None,
        span: span(),
    };
    let block_some = ast::Block {
        stmts: vec![],
        expr: Some(Box::new(int_lit(1))),
        span: span(),
    };

    assert!(ast::Expr::Block(block_none.clone()).alpha_eq(&ast::Expr::Block(block_none.clone())));
    assert!(!block_none.alpha_eq(&block_some));

    let given_none = ast::Expr::Given {
        cond: Box::new(ast::Expr::Literal(ast::Literal::Bool(true))),
        then_block: block_none.clone(),
        else_expr: None,
    };
    let given_some = ast::Expr::Given {
        cond: Box::new(ast::Expr::Literal(ast::Literal::Bool(true))),
        then_block: block_none.clone(),
        else_expr: Some(Box::new(int_lit(0))),
    };
    assert!(given_none.alpha_eq(&given_none.clone()));
    assert!(!given_none.alpha_eq(&given_some));

    let cascade_none = ast::Expr::Cascade {
        expr: Box::new(int_lit(3)),
        context: None,
    };
    let cascade_some = ast::Expr::Cascade {
        expr: Box::new(int_lit(3)),
        context: Some(Box::new(int_lit(4))),
    };
    assert!(cascade_none.alpha_eq(&cascade_none.clone()));
    assert!(!cascade_none.alpha_eq(&cascade_some));

    let seek_none = ast::Expr::Seek {
        body: block_none.clone(),
        catch_var: None,
        catch_body: None,
    };
    let seek_some = ast::Expr::Seek {
        body: block_none.clone(),
        catch_var: None,
        catch_body: Some(block_none.clone()),
    };
    assert!(seek_none.alpha_eq(&seek_none.clone()));
    assert!(!seek_none.alpha_eq(&seek_some));

    let let_none = ast::Stmt::Let {
        pat: ast::Pattern::Ident("x".to_string(), false, span()),
        ty: None,
        init: None,
        span: span(),
    };
    let let_with_ty = ast::Stmt::Let {
        pat: ast::Pattern::Ident("x".to_string(), false, span()),
        ty: Some(ast::Type::Prim("i32".to_string())),
        init: None,
        span: span(),
    };
    let let_with_init = ast::Stmt::Let {
        pat: ast::Pattern::Ident("x".to_string(), false, span()),
        ty: None,
        init: Some(int_lit(1)),
        span: span(),
    };

    assert!(let_none.alpha_eq(&let_none.clone()));
    assert!(!let_none.alpha_eq(&let_with_ty));
    assert!(!let_none.alpha_eq(&let_with_init));
    assert!(!let_none.alpha_eq(&ast::Stmt::Expr(int_lit(1))));

    assert!(!ast::Pattern::Wildcard.alpha_eq(&ast::Pattern::Literal(ast::Literal::Nil)));

    assert!(
        !ast::Type::Prim("i32".to_string()).alpha_eq(&ast::Type::Optional(Box::new(
            ast::Type::Prim("i32".to_string())
        )))
    );

    assert!(!ast::GenericArg::Type(ast::Type::Prim("i32".to_string()))
        .alpha_eq(&ast::GenericArg::Expr(int_lit(0))));

    let arm_no_guard = ast::Arm {
        pattern: ast::Pattern::Wildcard,
        guard: None,
        body: int_lit(1),
        span: span(),
    };
    let arm_with_guard = ast::Arm {
        pattern: ast::Pattern::Wildcard,
        guard: Some(ast::Expr::Literal(ast::Literal::Bool(true))),
        body: int_lit(1),
        span: span(),
    };

    assert!(arm_no_guard.alpha_eq(&arm_no_guard.clone()));
    assert!(!arm_no_guard.alpha_eq(&arm_with_guard));
}
