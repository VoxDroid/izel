use izel_lexer::cursor::Cursor;
use izel_lexer::string_reader::eat_escape;

fn remainder_after_escape(input_after_backslash: &str) -> char {
    let mut cursor = Cursor::new(input_after_backslash);
    eat_escape(&mut cursor);
    cursor.first()
}

#[test]
fn simple_escapes_consume_single_character() {
    assert_eq!(remainder_after_escape("nX"), 'X');
    assert_eq!(remainder_after_escape("rX"), 'X');
    assert_eq!(remainder_after_escape("tX"), 'X');
    assert_eq!(remainder_after_escape("\\X"), 'X');
    assert_eq!(remainder_after_escape("\"X"), 'X');
    assert_eq!(remainder_after_escape("'X"), 'X');
    assert_eq!(remainder_after_escape("0X"), 'X');
}

#[test]
fn unicode_escape_consumes_braced_hex_sequence_when_present() {
    assert_eq!(remainder_after_escape("u{41}X"), 'X');
    assert_eq!(remainder_after_escape("u{4_1}X"), 'X');
}

#[test]
fn unicode_escape_without_closing_brace_leaves_cursor_at_first_invalid_char() {
    assert_eq!(remainder_after_escape("u{41X"), 'X');
}

#[test]
fn unicode_escape_without_open_brace_consumes_only_u() {
    assert_eq!(remainder_after_escape("uX"), 'X');
}

#[test]
fn hex_escape_consumes_up_to_two_hex_digits() {
    assert_eq!(remainder_after_escape("x4fX"), 'X');
    assert_eq!(remainder_after_escape("x4X"), 'X');
}

#[test]
fn hex_escape_with_non_hex_followup_stops_immediately() {
    assert_eq!(remainder_after_escape("xZX"), 'Z');
}

#[test]
fn unknown_escape_consumes_escape_codepoint_only() {
    assert_eq!(remainder_after_escape("qX"), 'X');
}
