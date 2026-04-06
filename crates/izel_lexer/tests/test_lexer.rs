use izel_lexer::{Base, Lexer, TokenKind};
use izel_span::SourceId;

fn lex_kinds(source: &str) -> Vec<TokenKind> {
    let mut lexer = Lexer::new(source, SourceId(0));
    let mut kinds = Vec::new();

    loop {
        let token = lexer.next_token();
        if token.kind == TokenKind::Eof {
            break;
        }
        if token.kind != TokenKind::Whitespace {
            kinds.push(token.kind);
        }
    }

    kinds
}

#[test]
fn lexes_numeric_literals_across_supported_bases() {
    let kinds = lex_kinds("0b1010 0o77 0xFF 42");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Int {
                base: Base::Binary,
                empty_int: false,
            },
            TokenKind::Int {
                base: Base::Octal,
                empty_int: false,
            },
            TokenKind::Int {
                base: Base::Hexadecimal,
                empty_int: false,
            },
            TokenKind::Int {
                base: Base::Decimal,
                empty_int: false,
            },
        ]
    );
}

#[test]
fn malformed_hex_prefix_does_not_panic_and_stream_progresses() {
    let kinds = lex_kinds("0xG");
    assert!(matches!(
        kinds.first(),
        Some(TokenKind::Int {
            base: Base::Hexadecimal,
            ..
        })
    ));
    assert!(kinds.contains(&TokenKind::Ident));
}

#[test]
fn unterminated_string_literal_is_reported_in_token_kind() {
    let kinds = lex_kinds("\"unterminated");
    assert_eq!(kinds, vec![TokenKind::Str { terminated: false }]);
}

#[test]
fn nested_block_and_doc_comments_are_tokenized() {
    let kinds = lex_kinds("/~ outer /~ nested ~/ outer ~/\n/// docs");
    let comment_count = kinds.iter().filter(|k| **k == TokenKind::Comment).count();
    assert_eq!(comment_count, 2);
}

#[test]
fn lexes_max_u128_binary_literal_without_panicking() {
    let literal = format!("0b{}", "1".repeat(128));
    let kinds = lex_kinds(&literal);

    assert_eq!(
        kinds,
        vec![TokenKind::Int {
            base: Base::Binary,
            empty_int: false,
        }]
    );
}

#[test]
fn trailing_underscore_number_still_tokenizes_as_integer_for_regression_tracking() {
    let kinds = lex_kinds("1_000_");
    assert_eq!(kinds.len(), 1);
    assert!(matches!(
        kinds.first(),
        Some(TokenKind::Int {
            base: Base::Decimal,
            ..
        })
    ));
}

#[test]
fn raw_string_with_nested_quotes_is_tokenized() {
    let kinds = lex_kinds(r##"r#" "quotes" "#"##);
    assert_eq!(kinds, vec![TokenKind::Str { terminated: true }]);
}

#[test]
fn multiline_interpolated_string_stays_single_token() {
    let kinds = lex_kinds("`sum: {x +\n y}`");
    assert_eq!(kinds, vec![TokenKind::InterpolatedStr { terminated: true }]);
}

#[test]
fn invalid_unicode_escape_does_not_panic_lexer() {
    let kinds = lex_kinds("\"\\u{XYZZY}\"");
    assert_eq!(kinds, vec![TokenKind::Str { terminated: true }]);
}

#[test]
fn doc_comments_at_eof_are_emitted_as_comment_tokens() {
    let kinds = lex_kinds("/// outer doc\n//! inner doc");
    let comment_count = kinds.iter().filter(|k| **k == TokenKind::Comment).count();
    assert_eq!(comment_count, 2);
}

#[test]
fn lexes_remaining_operator_and_fallback_token_paths() {
    let kinds = lex_kinds("..= ^ §");
    assert_eq!(
        kinds,
        vec![TokenKind::DotDotEq, TokenKind::Caret, TokenKind::Unknown]
    );
}

#[test]
fn lexes_byte_literal_and_byte_string_escape_paths() {
    let kinds = lex_kinds("b'\\n' b\"\\t\"");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Byte { terminated: true },
            TokenKind::ByteStr { terminated: true }
        ]
    );
}

#[test]
fn raw_and_interpolated_string_edge_paths_are_tokenized() {
    let kinds = lex_kinds("`\\n` r# r#\"unterminated");
    assert_eq!(
        kinds,
        vec![
            TokenKind::InterpolatedStr { terminated: true },
            TokenKind::Unknown,
            TokenKind::Str { terminated: false },
        ]
    );
}

#[test]
fn unterminated_char_and_byte_literal_paths_are_tokenized() {
    let kinds = lex_kinds("' b'");
    assert_eq!(
        kinds,
        vec![
            TokenKind::Char { terminated: false },
            TokenKind::Byte { terminated: false },
        ]
    );
}

#[test]
fn lone_quote_char_path_sets_unterminated_char_token() {
    let kinds = lex_kinds("'");
    assert_eq!(kinds, vec![TokenKind::Char { terminated: false }]);
}

#[test]
fn terminated_char_literal_path_is_tokenized() {
    let kinds = lex_kinds("'a'");
    assert_eq!(kinds, vec![TokenKind::Char { terminated: true }]);
}
