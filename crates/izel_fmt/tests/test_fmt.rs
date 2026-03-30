use izel_fmt::format_source;

#[test]
fn format_source_is_idempotent() {
    let source = "forge main(){let x=1+2;give x}";

    let once = format_source(source);
    let twice = format_source(&once);

    assert_eq!(once, twice);
}

#[test]
fn format_source_normalizes_operator_spacing_and_newline() {
    let source = "forge main(){give 1+2}";
    let formatted = format_source(source);

    assert!(formatted.contains("1 + 2"));
    assert!(formatted.ends_with('\n'));
}
