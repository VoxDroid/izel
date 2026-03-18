use crate::ast::{BinaryOp, Expr, Literal, UnaryOp};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Int(i128),
    Bool(bool),
    String(String),
    Unknown,
}

pub fn eval_expr(expr: &Expr, context: &HashMap<String, ConstValue>) -> ConstValue {
    match expr {
        Expr::Literal(lit) => match lit {
            Literal::Int(i) => ConstValue::Int(*i),
            Literal::Bool(b) => ConstValue::Bool(*b),
            Literal::Str(s) => ConstValue::String(s.clone()),
            _ => ConstValue::Unknown,
        },
        Expr::Ident(name, _) => context.get(name).cloned().unwrap_or(ConstValue::Unknown),
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
                _ => ConstValue::Unknown,
            }
        }
        Expr::Unary(op, inner) => {
            let v = eval_expr(inner, context);
            match v {
                ConstValue::Int(i) => match op {
                    UnaryOp::Neg => ConstValue::Int(i.wrapping_neg()),
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
