use crate::ast::Forge;
use crate::eval::{eval_expr, ConstValue};
use izel_diagnostics::{primary_label, Diagnostic};
use izel_span::Span;
use std::collections::HashMap;

pub struct ContractChecker;

impl ContractChecker {
    pub fn check_requires(forge: &Forge, args: &[ConstValue], call_span: Span) -> Vec<Diagnostic> {
        Self::check_requires_raw(&forge.name, &forge.params, &forge.requires, args, call_span)
    }

    pub fn check_requires_raw(
        name: &str,
        params: &[crate::ast::Param],
        requires: &[crate::ast::Expr],
        args: &[ConstValue],
        call_span: Span,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut context = HashMap::new();

        for (param, val) in params.iter().zip(args) {
            context.insert(param.name.clone(), val.clone());
        }

        for (i, req) in requires.iter().enumerate() {
            let res = eval_expr(req, &context);
            if let ConstValue::Bool(false) = res {
                diagnostics.push(
                    Diagnostic::error()
                        .with_message(format!("precondition violation for '{}'", name))
                        .with_code(format!("E-REQ-{}", i))
                        .with_labels(vec![primary_label(call_span, "requires condition not met")]),
                );
            }
        }
        diagnostics
    }

    pub fn check_ensures(
        forge: &Forge,
        ret_val: &ConstValue,
        ret_span: Span,
        params: &HashMap<String, ConstValue>,
    ) -> Vec<Diagnostic> {
        Self::check_ensures_raw(&forge.name, &forge.ensures, ret_val, ret_span, params)
    }

    pub fn check_ensures_raw(
        name: &str,
        ensures: &[crate::ast::Expr],
        ret_val: &ConstValue,
        ret_span: Span,
        params: &HashMap<String, ConstValue>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut context = params.clone();

        context.insert("result".to_string(), ret_val.clone());

        for (i, ens) in ensures.iter().enumerate() {
            let res = eval_expr(ens, &context);
            if let ConstValue::Bool(false) = res {
                diagnostics.push(
                    Diagnostic::error()
                        .with_message(format!("postcondition violation for '{}'", name))
                        .with_code(format!("E-ENS-{}", i))
                        .with_labels(vec![primary_label(ret_span, "ensures condition not met")]),
                );
            }
        }
        diagnostics
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;

    #[test]
    fn test_temporal_constraints() {
        // Mock a function with @requires(n > 0)
        let n_ident = Box::new(ast::Expr::Ident("n".to_string(), Span::dummy()));
        let zero_lit = Box::new(ast::Expr::Literal(ast::Literal::Int(0)));
        let req = ast::Expr::Binary(ast::BinaryOp::Gt, n_ident.clone(), zero_lit.clone());

        let forge = ast::Forge {
            name: "test".to_string(),
            visibility: ast::Visibility::Hidden,
            generic_params: vec![],
            params: vec![ast::Param {
                name: "n".to_string(),
                ty: ast::Type::Prim("i32".to_string()),
                default_value: None,
                is_variadic: false,
                span: Span::dummy(),
            }],
            ret_type: ast::Type::Prim("i32".to_string()),
            effects: vec![],
            attributes: vec![],
            requires: vec![req],
            ensures: vec![],
            is_flow: false,
            body: None,
            span: Span::dummy(),
        };

        // 1. Check requires with n = 1 (Success)
        let args = vec![ConstValue::Int(1)];
        let diags = ContractChecker::check_requires(&forge, &args, Span::dummy());
        assert!(diags.is_empty());

        // 2. Check requires with n = 0 (Failure)
        let args = vec![ConstValue::Int(0)];
        let diags = ContractChecker::check_requires(&forge, &args, Span::dummy());
        assert!(!diags.is_empty());
        assert_eq!(diags[0].message, "precondition violation for 'test'");

        // 3. Check ensures(result > 0) with result = 1 (Success)
        let res_ident = Box::new(ast::Expr::Ident("result".to_string(), Span::dummy()));
        let ens = ast::Expr::Binary(ast::BinaryOp::Gt, res_ident, zero_lit.clone());
        let forge_ens = ast::Forge {
            ensures: vec![ens],
            ..forge.clone()
        };
        let diags = ContractChecker::check_ensures(
            &forge_ens,
            &ConstValue::Int(1),
            Span::dummy(),
            &HashMap::new(),
        );
        assert!(diags.is_empty());

        // 4. Check ensures(result > 0) with result = 0 (Failure)
        let diags = ContractChecker::check_ensures(
            &forge_ens,
            &ConstValue::Int(0),
            Span::dummy(),
            &HashMap::new(),
        );
        assert!(!diags.is_empty());
        assert_eq!(diags[0].message, "postcondition violation for 'test'");
    }
}
