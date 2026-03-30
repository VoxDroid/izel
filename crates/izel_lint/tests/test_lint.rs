use izel_diagnostics::warning;
use izel_lint::{Lint, LintContext, LintManager, NoOpLint};

struct PositiveLint;

impl Lint<i32> for PositiveLint {
    fn name(&self) -> &str {
        "positive_lint"
    }

    fn check(&self, ast: &i32, context: &mut LintContext) {
        if *ast > 0 {
            context.report(warning("value is positive"));
        }
    }
}

#[test]
fn lint_manager_runs_registered_lints() {
    let mut manager = LintManager::<i32>::new();
    manager.add(PositiveLint);

    let diagnostics = manager.run(&7);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message, "value is positive");
}

#[test]
fn noop_lint_reports_no_diagnostics() {
    let mut manager = LintManager::<i32>::new();
    manager.add(NoOpLint);

    let diagnostics = manager.run(&7);
    assert!(diagnostics.is_empty());
}
