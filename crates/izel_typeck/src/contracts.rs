use izel_diagnostics::{primary_label, Diagnostic};
use izel_parser::ast::Expr;
use izel_parser::eval::{eval_expr, ConstValue};
use izel_span::Span;
use std::collections::HashMap;

pub struct ContractChecker;

impl ContractChecker {
    /// Check @requires using raw components (for call-site verification from Scheme data).
    pub fn check_requires_from_scheme(
        name: &str,
        param_names: &[String],
        requires: &[Expr],
        args: &[ConstValue],
        call_span: Span,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut context: HashMap<String, ConstValue> = HashMap::new();

        for (pname, val) in param_names.iter().zip(args) {
            context.insert(pname.clone(), val.clone());
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

    /// Check @ensures using raw components (for post-body verification from Scheme data).
    pub fn check_ensures_from_scheme(
        name: &str,
        ensures: &[Expr],
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
