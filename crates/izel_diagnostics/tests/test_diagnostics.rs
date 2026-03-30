use izel_diagnostics::{error, primary_label, secondary_label, warning, LabelStyle, Severity};
use izel_span::{BytePos, SourceId, Span};

#[test]
fn error_and_warning_helpers_set_expected_severity_and_message() {
    let err = error("boom");
    let warn = warning("careful");

    assert_eq!(err.severity, Severity::Error);
    assert_eq!(err.message, "boom");
    assert_eq!(warn.severity, Severity::Warning);
    assert_eq!(warn.message, "careful");
}

#[test]
fn label_helpers_map_span_to_file_and_range() {
    let span = Span::new(BytePos(3), BytePos(9), SourceId(12));
    let primary = primary_label(span, "primary message");
    let secondary = secondary_label(span, "secondary message");

    assert_eq!(primary.file_id, SourceId(12));
    assert_eq!(primary.range, 3..9);
    assert_eq!(primary.style, LabelStyle::Primary);
    assert_eq!(primary.message, "primary message");

    assert_eq!(secondary.file_id, SourceId(12));
    assert_eq!(secondary.range, 3..9);
    assert_eq!(secondary.style, LabelStyle::Secondary);
    assert_eq!(secondary.message, "secondary message");
}
