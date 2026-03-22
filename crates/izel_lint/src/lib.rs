use izel_diagnostics::Diagnostic;

/// A static analysis lint pass.
pub trait Lint<AST> {
    fn name(&self) -> &str;
    fn check(&self, ast: &AST, context: &mut LintContext);
}

/// Context for reporting lint diagnostics.
pub struct LintContext {
    pub diagnostics: Vec<Diagnostic>,
}

impl Default for LintContext {
    fn default() -> Self {
        Self::new()
    }
}

impl LintContext {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn report(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }
}

/// Manages the execution of multiple lints.
pub struct LintManager<AST> {
    pub lints: Vec<Box<dyn Lint<AST>>>,
}

impl<AST> Default for LintManager<AST> {
    fn default() -> Self {
        Self::new()
    }
}

impl<AST> LintManager<AST> {
    pub fn new() -> Self {
        Self { lints: Vec::new() }
    }

    pub fn add<L: 'static + Lint<AST>>(&mut self, lint: L) {
        self.lints.push(Box::new(lint));
    }

    pub fn run(&self, ast: &AST) -> Vec<Diagnostic> {
        let mut context = LintContext::new();
        for lint in &self.lints {
            lint.check(ast, &mut context);
        }
        context.diagnostics
    }
}

/// An example lint that does nothing (can be expanded).
pub struct NoOpLint;

impl<AST> Lint<AST> for NoOpLint {
    fn name(&self) -> &str {
        "no_op"
    }

    fn check(&self, _ast: &AST, _context: &mut LintContext) {}
}
