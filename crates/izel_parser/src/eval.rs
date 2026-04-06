use crate::ast::{BinaryOp, Expr, Literal, Pattern, Stmt, UnaryOp};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Int(i128),
    Float(f64),
    Bool(bool),
    String(String),
    Nil,
    Unknown,
}

pub fn eval_expr(expr: &Expr, context: &HashMap<String, ConstValue>) -> ConstValue {
    match expr {
        Expr::Literal(lit) => match lit {
            Literal::Int(i) => ConstValue::Int(*i),
            Literal::Float(f) => ConstValue::Float(*f),
            Literal::Bool(b) => ConstValue::Bool(*b),
            Literal::Str(s) => ConstValue::String(s.clone()),
            Literal::Nil => ConstValue::Nil,
        },
        Expr::Ident(name, _) => context.get(name).cloned().unwrap_or(ConstValue::Unknown),
        Expr::Block(block) => eval_block(block, context),
        Expr::Given {
            cond,
            then_block,
            else_expr,
        } => match eval_expr(cond, context) {
            ConstValue::Bool(true) => eval_block(then_block, context),
            ConstValue::Bool(false) => {
                if let Some(e) = else_expr {
                    eval_expr(e, context)
                } else {
                    ConstValue::Nil
                }
            }
            _ => ConstValue::Unknown,
        },
        Expr::Binary(op, left, right) => {
            let lv = eval_expr(left, context);
            let rv = eval_expr(right, context);
            match (lv, rv) {
                (ConstValue::Int(l), ConstValue::Int(r)) => match op {
                    BinaryOp::Add => ConstValue::Int(l.wrapping_add(r)),
                    BinaryOp::Sub => ConstValue::Int(l.wrapping_sub(r)),
                    BinaryOp::Mul => ConstValue::Int(l.wrapping_mul(r)),
                    BinaryOp::Div => {
                        if r != 0 {
                            ConstValue::Int(l / r)
                        } else {
                            ConstValue::Unknown
                        }
                    }
                    BinaryOp::Rem => {
                        if r != 0 {
                            ConstValue::Int(l % r)
                        } else {
                            ConstValue::Unknown
                        }
                    }
                    BinaryOp::BitAnd => ConstValue::Int(l & r),
                    BinaryOp::BitOr => ConstValue::Int(l | r),
                    BinaryOp::BitXor => ConstValue::Int(l ^ r),
                    BinaryOp::Shl => {
                        if (0..128).contains(&r) {
                            ConstValue::Int(l.wrapping_shl(r as u32))
                        } else {
                            ConstValue::Unknown
                        }
                    }
                    BinaryOp::Shr => {
                        if (0..128).contains(&r) {
                            ConstValue::Int(l.wrapping_shr(r as u32))
                        } else {
                            ConstValue::Unknown
                        }
                    }
                    BinaryOp::Eq => ConstValue::Bool(l == r),
                    BinaryOp::Ne => ConstValue::Bool(l != r),
                    BinaryOp::Lt => ConstValue::Bool(l < r),
                    BinaryOp::Gt => ConstValue::Bool(l > r),
                    BinaryOp::Le => ConstValue::Bool(l <= r),
                    BinaryOp::Ge => ConstValue::Bool(l >= r),
                    _ => ConstValue::Unknown,
                },
                (ConstValue::Float(l), ConstValue::Float(r)) => match op {
                    BinaryOp::Add => ConstValue::Float(l + r),
                    BinaryOp::Sub => ConstValue::Float(l - r),
                    BinaryOp::Mul => ConstValue::Float(l * r),
                    BinaryOp::Div => {
                        if r != 0.0 {
                            ConstValue::Float(l / r)
                        } else {
                            ConstValue::Unknown
                        }
                    }
                    BinaryOp::Eq => ConstValue::Bool(l == r),
                    BinaryOp::Ne => ConstValue::Bool(l != r),
                    BinaryOp::Lt => ConstValue::Bool(l < r),
                    BinaryOp::Gt => ConstValue::Bool(l > r),
                    BinaryOp::Le => ConstValue::Bool(l <= r),
                    BinaryOp::Ge => ConstValue::Bool(l >= r),
                    _ => ConstValue::Unknown,
                },
                (ConstValue::Bool(l), ConstValue::Bool(r)) => match op {
                    BinaryOp::And => ConstValue::Bool(l && r),
                    BinaryOp::Or => ConstValue::Bool(l || r),
                    BinaryOp::Eq => ConstValue::Bool(l == r),
                    BinaryOp::Ne => ConstValue::Bool(l != r),
                    _ => ConstValue::Unknown,
                },
                (ConstValue::String(l), ConstValue::String(r)) => match op {
                    BinaryOp::Add => ConstValue::String(format!("{}{}", l, r)),
                    BinaryOp::Eq => ConstValue::Bool(l == r),
                    BinaryOp::Ne => ConstValue::Bool(l != r),
                    _ => ConstValue::Unknown,
                },
                (ConstValue::Nil, ConstValue::Nil) => match op {
                    BinaryOp::Eq => ConstValue::Bool(true),
                    BinaryOp::Ne => ConstValue::Bool(false),
                    _ => ConstValue::Unknown,
                },
                _ => ConstValue::Unknown,
            }
        }
        Expr::Unary(op, inner) => {
            let v = eval_expr(inner, context);
            match v {
                ConstValue::Int(i) => match op {
                    UnaryOp::Neg => ConstValue::Int(i.wrapping_neg()),
                    UnaryOp::BitNot => ConstValue::Int(!i),
                    _ => ConstValue::Unknown,
                },
                ConstValue::Float(f) => match op {
                    UnaryOp::Neg => ConstValue::Float(-f),
                    _ => ConstValue::Unknown,
                },
                ConstValue::Bool(b) => match op {
                    UnaryOp::Not => ConstValue::Bool(!b),
                    _ => ConstValue::Unknown,
                },
                _ => ConstValue::Unknown,
            }
        }
        _ => ConstValue::Unknown,
    }
}

fn eval_block(block: &crate::ast::Block, context: &HashMap<String, ConstValue>) -> ConstValue {
    let mut scoped = context.clone();

    for stmt in &block.stmts {
        match stmt {
            Stmt::Let { pat, init, .. } => {
                let Some(init_expr) = init else {
                    return ConstValue::Unknown;
                };

                let value = eval_expr(init_expr, &scoped);
                if value == ConstValue::Unknown {
                    return ConstValue::Unknown;
                }

                if let Pattern::Ident(name, _, _) = pat {
                    scoped.insert(name.clone(), value);
                } else {
                    return ConstValue::Unknown;
                }
            }
            Stmt::Expr(expr) => {
                if eval_expr(expr, &scoped) == ConstValue::Unknown {
                    return ConstValue::Unknown;
                }
            }
        }
    }

    if let Some(expr) = &block.expr {
        eval_expr(expr, &scoped)
    } else {
        ConstValue::Nil
    }
}

#[cfg(test)]
mod tests {
    use super::{eval_expr, ConstValue};
    use crate::ast::{BinaryOp, Block, Expr, Literal, Pattern, Stmt, UnaryOp};
    use izel_span::Span;

    #[test]
    fn eval_supports_float_and_string_ops() {
        let ctx = std::collections::HashMap::new();
        let float_add = Expr::Binary(
            BinaryOp::Add,
            Box::new(Expr::Literal(Literal::Float(1.5))),
            Box::new(Expr::Literal(Literal::Float(2.5))),
        );
        let str_add = Expr::Binary(
            BinaryOp::Add,
            Box::new(Expr::Literal(Literal::Str("iz".to_string()))),
            Box::new(Expr::Literal(Literal::Str("el".to_string()))),
        );

        assert_eq!(eval_expr(&float_add, &ctx), ConstValue::Float(4.0));
        assert_eq!(
            eval_expr(&str_add, &ctx),
            ConstValue::String("izel".to_string())
        );
    }

    #[test]
    fn eval_supports_given_and_block_scopes() {
        let ctx = std::collections::HashMap::new();
        let block = Block {
            stmts: vec![Stmt::Let {
                pat: Pattern::Ident("x".to_string(), false, Span::dummy()),
                ty: None,
                init: Some(Expr::Literal(Literal::Int(7))),
                span: Span::dummy(),
            }],
            expr: Some(Box::new(Expr::Ident("x".to_string(), Span::dummy()))),
            span: Span::dummy(),
        };
        let given = Expr::Given {
            cond: Box::new(Expr::Literal(Literal::Bool(true))),
            then_block: block,
            else_expr: Some(Box::new(Expr::Literal(Literal::Int(0)))),
        };

        assert_eq!(eval_expr(&given, &ctx), ConstValue::Int(7));
    }

    #[test]
    fn eval_supports_bitwise_and_unary_ops() {
        let ctx = std::collections::HashMap::new();
        let bit = Expr::Binary(
            BinaryOp::BitAnd,
            Box::new(Expr::Literal(Literal::Int(6))),
            Box::new(Expr::Literal(Literal::Int(3))),
        );
        let unary = Expr::Unary(UnaryOp::BitNot, Box::new(Expr::Literal(Literal::Int(0))));

        assert_eq!(eval_expr(&bit, &ctx), ConstValue::Int(2));
        assert_eq!(eval_expr(&unary, &ctx), ConstValue::Int(!0));
    }

    #[test]
    fn eval_covers_unknown_and_guard_branches() {
        let ctx = std::collections::HashMap::new();

        let int_div_zero = Expr::Binary(
            BinaryOp::Div,
            Box::new(Expr::Literal(Literal::Int(8))),
            Box::new(Expr::Literal(Literal::Int(0))),
        );
        assert_eq!(eval_expr(&int_div_zero, &ctx), ConstValue::Unknown);

        let int_rem_zero = Expr::Binary(
            BinaryOp::Rem,
            Box::new(Expr::Literal(Literal::Int(8))),
            Box::new(Expr::Literal(Literal::Int(0))),
        );
        assert_eq!(eval_expr(&int_rem_zero, &ctx), ConstValue::Unknown);

        let shl_oob = Expr::Binary(
            BinaryOp::Shl,
            Box::new(Expr::Literal(Literal::Int(1))),
            Box::new(Expr::Literal(Literal::Int(128))),
        );
        assert_eq!(eval_expr(&shl_oob, &ctx), ConstValue::Unknown);

        let shr_oob = Expr::Binary(
            BinaryOp::Shr,
            Box::new(Expr::Literal(Literal::Int(1))),
            Box::new(Expr::Literal(Literal::Int(128))),
        );
        assert_eq!(eval_expr(&shr_oob, &ctx), ConstValue::Unknown);

        let float_div_zero = Expr::Binary(
            BinaryOp::Div,
            Box::new(Expr::Literal(Literal::Float(1.0))),
            Box::new(Expr::Literal(Literal::Float(0.0))),
        );
        assert_eq!(eval_expr(&float_div_zero, &ctx), ConstValue::Unknown);

        let given_false_no_else = Expr::Given {
            cond: Box::new(Expr::Literal(Literal::Bool(false))),
            then_block: Block {
                stmts: vec![],
                expr: Some(Box::new(Expr::Literal(Literal::Int(1)))),
                span: Span::dummy(),
            },
            else_expr: None,
        };
        assert_eq!(eval_expr(&given_false_no_else, &ctx), ConstValue::Nil);

        let unsupported_unary_for_bool =
            Expr::Unary(UnaryOp::Neg, Box::new(Expr::Literal(Literal::Bool(true))));
        assert_eq!(
            eval_expr(&unsupported_unary_for_bool, &ctx),
            ConstValue::Unknown
        );

        let block_with_missing_init = Expr::Block(Block {
            stmts: vec![Stmt::Let {
                pat: Pattern::Ident("x".to_string(), false, Span::dummy()),
                ty: None,
                init: None,
                span: Span::dummy(),
            }],
            expr: Some(Box::new(Expr::Ident("x".to_string(), Span::dummy()))),
            span: Span::dummy(),
        });
        assert_eq!(
            eval_expr(&block_with_missing_init, &ctx),
            ConstValue::Unknown
        );

        let block_with_non_ident_pattern = Expr::Block(Block {
            stmts: vec![Stmt::Let {
                pat: Pattern::Tuple(vec![]),
                ty: None,
                init: Some(Expr::Literal(Literal::Int(1))),
                span: Span::dummy(),
            }],
            expr: None,
            span: Span::dummy(),
        });
        assert_eq!(
            eval_expr(&block_with_non_ident_pattern, &ctx),
            ConstValue::Unknown
        );
    }
}
