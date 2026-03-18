//! Types and utilities for high-fidelity error reporting.

use codespan_reporting::diagnostic::{Diagnostic as CsDiagnostic, Label as CsLabel};
pub use codespan_reporting::diagnostic::{LabelStyle, Severity};
use izel_span::Span;

pub type Diagnostic = CsDiagnostic<izel_span::SourceId>;
pub type Label = CsLabel<izel_span::SourceId>;

pub fn error(msg: impl Into<String>) -> Diagnostic {
    Diagnostic::error().with_message(msg)
}

pub fn warning(msg: impl Into<String>) -> Diagnostic {
    Diagnostic::warning().with_message(msg)
}

pub fn primary_label(span: Span, msg: impl Into<String>) -> Label {
    Label::primary(span.source_id, span.lo.0 as usize..span.hi.0 as usize).with_message(msg)
}

pub fn secondary_label(span: Span, msg: impl Into<String>) -> Label {
    Label::secondary(span.source_id, span.lo.0 as usize..span.hi.0 as usize).with_message(msg)
}

pub fn emit(source_map: &izel_span::SourceMap, diagnostic: &Diagnostic) {
    use codespan_reporting::term;
    use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

    let writer = StandardStream::stderr(ColorChoice::Auto);
    let config = term::Config::default();

    term::emit(&mut writer.lock(), &config, source_map, diagnostic)
        .expect("failed to emit diagnostic");
}
