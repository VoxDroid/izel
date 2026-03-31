use izel_parser::ast::{BinaryOp, Block, Expr, Literal, Pattern, Stmt, UnaryOp};
use izel_parser::eval::{eval_expr, ConstValue};
use izel_span::Span;
use std::collections::HashMap;

fn int(value: i128) -> Expr {
    Expr::Literal(Literal::Int(value))
}

fn float(value: f64) -> Expr {
    Expr::Literal(Literal::Float(value))
}

fn bool_lit(value: bool) -> Expr {
    Expr::Literal(Literal::Bool(value))
}

#[test]
fn eval_int_ops_cover_arithmetic_bitwise_and_comparisons() {
    let ctx = HashMap::new();

    let cases = vec![
        (
            Expr::Binary(BinaryOp::Add, Box::new(int(2)), Box::new(int(3))),
            ConstValue::Int(5),
        ),
        (
            Expr::Binary(BinaryOp::Sub, Box::new(int(9)), Box::new(int(4))),
            ConstValue::Int(5),
        ),
        (
            Expr::Binary(BinaryOp::Mul, Box::new(int(6)), Box::new(int(7))),
            ConstValue::Int(42),
        ),
        (
            Expr::Binary(BinaryOp::Div, Box::new(int(8)), Box::new(int(2))),
            ConstValue::Int(4),
        ),
        (
            Expr::Binary(BinaryOp::Rem, Box::new(int(10)), Box::new(int(4))),
            ConstValue::Int(2),
        ),
        (
            Expr::Binary(
                BinaryOp::BitOr,
                Box::new(int(0b1010)),
                Box::new(int(0b0101)),
            ),
            ConstValue::Int(0b1111),
        ),
        (
            Expr::Binary(
                BinaryOp::BitXor,
                Box::new(int(0b1100)),
                Box::new(int(0b1010)),
            ),
            ConstValue::Int(0b0110),
        ),
        (
            Expr::Binary(BinaryOp::Shl, Box::new(int(1)), Box::new(int(4))),
            ConstValue::Int(16),
        ),
        (
            Expr::Binary(BinaryOp::Shr, Box::new(int(32)), Box::new(int(3))),
            ConstValue::Int(4),
        ),
        (
            Expr::Binary(BinaryOp::Eq, Box::new(int(5)), Box::new(int(5))),
            ConstValue::Bool(true),
        ),
        (
            Expr::Binary(BinaryOp::Ne, Box::new(int(5)), Box::new(int(6))),
            ConstValue::Bool(true),
        ),
        (
            Expr::Binary(BinaryOp::Lt, Box::new(int(1)), Box::new(int(2))),
            ConstValue::Bool(true),
        ),
        (
            Expr::Binary(BinaryOp::Gt, Box::new(int(3)), Box::new(int(2))),
            ConstValue::Bool(true),
        ),
        (
            Expr::Binary(BinaryOp::Le, Box::new(int(2)), Box::new(int(2))),
            ConstValue::Bool(true),
        ),
        (
            Expr::Binary(BinaryOp::Ge, Box::new(int(2)), Box::new(int(2))),
            ConstValue::Bool(true),
        ),
    ];

    for (expr, expected) in cases {
        assert_eq!(eval_expr(&expr, &ctx), expected);
    }
}

#[test]
fn eval_invalid_int_ops_return_unknown() {
    let ctx = HashMap::new();

    let div_zero = Expr::Binary(BinaryOp::Div, Box::new(int(4)), Box::new(int(0)));
    let rem_zero = Expr::Binary(BinaryOp::Rem, Box::new(int(4)), Box::new(int(0)));
    let shift_too_large = Expr::Binary(BinaryOp::Shl, Box::new(int(1)), Box::new(int(128)));
    let shift_negative = Expr::Binary(BinaryOp::Shr, Box::new(int(1)), Box::new(int(-1)));

    assert_eq!(eval_expr(&div_zero, &ctx), ConstValue::Unknown);
    assert_eq!(eval_expr(&rem_zero, &ctx), ConstValue::Unknown);
    assert_eq!(eval_expr(&shift_too_large, &ctx), ConstValue::Unknown);
    assert_eq!(eval_expr(&shift_negative, &ctx), ConstValue::Unknown);
}

#[test]
fn eval_float_bool_string_nil_paths_and_unknown_mismatches() {
    let ctx = HashMap::new();

    let float_cmp = Expr::Binary(BinaryOp::Ge, Box::new(float(2.0)), Box::new(float(1.5)));
    let bool_ops = Expr::Binary(
        BinaryOp::Or,
        Box::new(bool_lit(false)),
        Box::new(bool_lit(true)),
    );
    let nil_eq = Expr::Binary(
        BinaryOp::Eq,
        Box::new(Expr::Literal(Literal::Nil)),
        Box::new(Expr::Literal(Literal::Nil)),
    );
    let mismatch = Expr::Binary(BinaryOp::Add, Box::new(int(1)), Box::new(bool_lit(true)));

    assert_eq!(eval_expr(&float_cmp, &ctx), ConstValue::Bool(true));
    assert_eq!(eval_expr(&bool_ops, &ctx), ConstValue::Bool(true));
    assert_eq!(eval_expr(&nil_eq, &ctx), ConstValue::Bool(true));
    assert_eq!(eval_expr(&mismatch, &ctx), ConstValue::Unknown);
}

#[test]
fn eval_unary_paths_and_identifier_lookup() {
    let mut ctx = HashMap::new();
    ctx.insert("x".to_string(), ConstValue::Int(7));

    let neg = Expr::Unary(UnaryOp::Neg, Box::new(int(5)));
    let not = Expr::Unary(UnaryOp::Not, Box::new(bool_lit(false)));
    let float_neg = Expr::Unary(UnaryOp::Neg, Box::new(float(1.25)));
    let unknown_ident = Expr::Ident("missing".to_string(), Span::dummy());

    assert_eq!(eval_expr(&neg, &ctx), ConstValue::Int(-5));
    assert_eq!(eval_expr(&not, &ctx), ConstValue::Bool(true));
    assert_eq!(eval_expr(&float_neg, &ctx), ConstValue::Float(-1.25));
    assert_eq!(
        eval_expr(&Expr::Ident("x".to_string(), Span::dummy()), &ctx),
        ConstValue::Int(7)
    );
    assert_eq!(eval_expr(&unknown_ident, &ctx), ConstValue::Unknown);
}

#[test]
fn eval_given_and_block_error_paths_return_unknown_or_nil() {
    let ctx = HashMap::new();

    let missing_init_block = Expr::Block(Block {
        stmts: vec![Stmt::Let {
            pat: Pattern::Ident("x".to_string(), false, Span::dummy()),
            ty: None,
            init: None,
            span: Span::dummy(),
        }],
        expr: None,
        span: Span::dummy(),
    });

    let non_ident_pattern_block = Expr::Block(Block {
        stmts: vec![Stmt::Let {
            pat: Pattern::Wildcard,
            ty: None,
            init: Some(int(1)),
            span: Span::dummy(),
        }],
        expr: None,
        span: Span::dummy(),
    });

    let unknown_stmt_block = Expr::Block(Block {
        stmts: vec![Stmt::Expr(Expr::Call(
            Box::new(Expr::Ident("f".to_string(), Span::dummy())),
            vec![],
        ))],
        expr: None,
        span: Span::dummy(),
    });

    let given_false_without_else = Expr::Given {
        cond: Box::new(bool_lit(false)),
        then_block: Block {
            stmts: vec![],
            expr: Some(Box::new(int(1))),
            span: Span::dummy(),
        },
        else_expr: None,
    };

    let given_non_bool = Expr::Given {
        cond: Box::new(int(1)),
        then_block: Block {
            stmts: vec![],
            expr: Some(Box::new(int(1))),
            span: Span::dummy(),
        },
        else_expr: Some(Box::new(int(2))),
    };

    assert_eq!(eval_expr(&missing_init_block, &ctx), ConstValue::Unknown);
    assert_eq!(
        eval_expr(&non_ident_pattern_block, &ctx),
        ConstValue::Unknown
    );
    assert_eq!(eval_expr(&unknown_stmt_block, &ctx), ConstValue::Unknown);
    assert_eq!(eval_expr(&given_false_without_else, &ctx), ConstValue::Nil);
    assert_eq!(eval_expr(&given_non_bool, &ctx), ConstValue::Unknown);
}

#[test]
fn eval_additional_branch_and_block_paths() {
    let ctx = HashMap::new();

    let given_false_with_else = Expr::Given {
        cond: Box::new(bool_lit(false)),
        then_block: Block {
            stmts: vec![],
            expr: Some(Box::new(int(1))),
            span: Span::dummy(),
        },
        else_expr: Some(Box::new(int(9))),
    };
    assert_eq!(eval_expr(&given_false_with_else, &ctx), ConstValue::Int(9));

    let float_cases = vec![
        (
            Expr::Binary(BinaryOp::Sub, Box::new(float(4.0)), Box::new(float(1.5))),
            ConstValue::Float(2.5),
        ),
        (
            Expr::Binary(BinaryOp::Mul, Box::new(float(3.0)), Box::new(float(2.0))),
            ConstValue::Float(6.0),
        ),
        (
            Expr::Binary(BinaryOp::Div, Box::new(float(8.0)), Box::new(float(2.0))),
            ConstValue::Float(4.0),
        ),
        (
            Expr::Binary(BinaryOp::Eq, Box::new(float(1.0)), Box::new(float(1.0))),
            ConstValue::Bool(true),
        ),
        (
            Expr::Binary(BinaryOp::Ne, Box::new(float(1.0)), Box::new(float(2.0))),
            ConstValue::Bool(true),
        ),
        (
            Expr::Binary(BinaryOp::Lt, Box::new(float(1.0)), Box::new(float(2.0))),
            ConstValue::Bool(true),
        ),
        (
            Expr::Binary(BinaryOp::Gt, Box::new(float(2.0)), Box::new(float(1.0))),
            ConstValue::Bool(true),
        ),
        (
            Expr::Binary(BinaryOp::Le, Box::new(float(2.0)), Box::new(float(2.0))),
            ConstValue::Bool(true),
        ),
    ];
    for (expr, expected) in float_cases {
        assert_eq!(eval_expr(&expr, &ctx), expected);
    }
    assert_eq!(
        eval_expr(
            &Expr::Binary(BinaryOp::And, Box::new(int(1)), Box::new(int(2))),
            &ctx
        ),
        ConstValue::Unknown
    );
    assert_eq!(
        eval_expr(
            &Expr::Binary(BinaryOp::Div, Box::new(float(1.0)), Box::new(float(0.0))),
            &ctx
        ),
        ConstValue::Unknown
    );
    assert_eq!(
        eval_expr(
            &Expr::Binary(BinaryOp::Rem, Box::new(float(1.0)), Box::new(float(1.0))),
            &ctx
        ),
        ConstValue::Unknown
    );

    assert_eq!(
        eval_expr(
            &Expr::Binary(
                BinaryOp::And,
                Box::new(bool_lit(true)),
                Box::new(bool_lit(false))
            ),
            &ctx
        ),
        ConstValue::Bool(false)
    );
    assert_eq!(
        eval_expr(
            &Expr::Binary(
                BinaryOp::Eq,
                Box::new(bool_lit(true)),
                Box::new(bool_lit(true))
            ),
            &ctx
        ),
        ConstValue::Bool(true)
    );
    assert_eq!(
        eval_expr(
            &Expr::Binary(
                BinaryOp::Ne,
                Box::new(bool_lit(true)),
                Box::new(bool_lit(false))
            ),
            &ctx
        ),
        ConstValue::Bool(true)
    );
    assert_eq!(
        eval_expr(
            &Expr::Binary(
                BinaryOp::Add,
                Box::new(bool_lit(true)),
                Box::new(bool_lit(false))
            ),
            &ctx
        ),
        ConstValue::Unknown
    );

    let s1 = Expr::Literal(Literal::Str("a".to_string()));
    let s2 = Expr::Literal(Literal::Str("b".to_string()));
    assert_eq!(
        eval_expr(
            &Expr::Binary(BinaryOp::Eq, Box::new(s1.clone()), Box::new(s1)),
            &ctx
        ),
        ConstValue::Bool(true)
    );
    assert_eq!(
        eval_expr(
            &Expr::Binary(BinaryOp::Ne, Box::new(s2.clone()), Box::new(s2)),
            &ctx
        ),
        ConstValue::Bool(false)
    );
    assert_eq!(
        eval_expr(
            &Expr::Binary(
                BinaryOp::Or,
                Box::new(Expr::Literal(Literal::Str("x".to_string()))),
                Box::new(Expr::Literal(Literal::Str("y".to_string())))
            ),
            &ctx
        ),
        ConstValue::Unknown
    );

    let nil = Expr::Literal(Literal::Nil);
    assert_eq!(
        eval_expr(
            &Expr::Binary(BinaryOp::Ne, Box::new(nil.clone()), Box::new(nil)),
            &ctx
        ),
        ConstValue::Bool(false)
    );
    assert_eq!(
        eval_expr(
            &Expr::Binary(
                BinaryOp::Add,
                Box::new(Expr::Literal(Literal::Nil)),
                Box::new(Expr::Literal(Literal::Nil))
            ),
            &ctx
        ),
        ConstValue::Unknown
    );

    assert_eq!(
        eval_expr(&Expr::Unary(UnaryOp::Not, Box::new(int(1))), &ctx),
        ConstValue::Unknown
    );
    assert_eq!(
        eval_expr(&Expr::Unary(UnaryOp::Not, Box::new(float(1.0))), &ctx),
        ConstValue::Unknown
    );
    assert_eq!(
        eval_expr(&Expr::Unary(UnaryOp::Neg, Box::new(bool_lit(true))), &ctx),
        ConstValue::Unknown
    );
    assert_eq!(
        eval_expr(
            &Expr::Unary(
                UnaryOp::Neg,
                Box::new(Expr::Literal(Literal::Str("nope".to_string())))
            ),
            &ctx
        ),
        ConstValue::Unknown
    );

    let block_nil = Expr::Block(Block {
        stmts: vec![Stmt::Expr(int(1))],
        expr: None,
        span: Span::dummy(),
    });
    assert_eq!(eval_expr(&block_nil, &ctx), ConstValue::Nil);

    let unknown_init = Expr::Block(Block {
        stmts: vec![Stmt::Let {
            pat: Pattern::Ident("x".to_string(), false, Span::dummy()),
            ty: None,
            init: Some(Expr::Ident("missing".to_string(), Span::dummy())),
            span: Span::dummy(),
        }],
        expr: Some(Box::new(int(1))),
        span: Span::dummy(),
    });
    assert_eq!(eval_expr(&unknown_init, &ctx), ConstValue::Unknown);
}
