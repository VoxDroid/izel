use codespan_reporting::files::Files;
use izel_span::{BytePos, SourceId, SourceMap, Span};

#[test]
fn span_dummy_has_zero_positions() {
    let span = Span::dummy();
    assert_eq!(span.lo, BytePos(0));
    assert_eq!(span.hi, BytePos(0));
    assert_eq!(span.source_id, SourceId(0));
}

#[test]
fn span_to_combines_bounds_within_same_source() {
    let a = Span::new(BytePos(2), BytePos(4), SourceId(3));
    let b = Span::new(BytePos(10), BytePos(14), SourceId(3));
    let combined = a.to(b);

    assert_eq!(combined.lo, BytePos(2));
    assert_eq!(combined.hi, BytePos(14));
    assert_eq!(combined.source_id, SourceId(3));
}

#[test]
fn source_map_add_and_line_queries_work() {
    let mut map = SourceMap::new();
    let id = map.add("sample.iz".to_string(), "alpha\nbeta\n".to_string());

    let file = map.get_file(id).expect("file should exist");
    assert_eq!(file.name, "sample.iz");
    assert_eq!(file.source, "alpha\nbeta\n");

    assert_eq!(<SourceMap as Files>::line_index(&map, id, 0).unwrap(), 0);
    assert_eq!(<SourceMap as Files>::line_index(&map, id, 5).unwrap(), 0);
    assert_eq!(<SourceMap as Files>::line_index(&map, id, 6).unwrap(), 1);
    assert_eq!(
        <SourceMap as Files>::line_range(&map, id, 1).unwrap(),
        6..11
    );

    assert_eq!(<SourceMap as Files>::name(&map, id).unwrap(), "sample.iz");
    assert_eq!(
        <SourceMap as Files>::source(&map, id).unwrap(),
        "alpha\nbeta\n"
    );

    assert!(<SourceMap as Files>::name(&map, SourceId(99)).is_err());
    assert!(<SourceMap as Files>::source(&map, SourceId(99)).is_err());
    assert!(<SourceMap as Files>::line_index(&map, SourceId(99), 0).is_err());
    assert!(<SourceMap as Files>::line_range(&map, SourceId(99), 0).is_err());

    let err = <SourceMap as Files>::line_range(&map, id, 99).unwrap_err();
    assert!(matches!(
        err,
        codespan_reporting::files::Error::LineTooLarge { .. }
    ));
}
