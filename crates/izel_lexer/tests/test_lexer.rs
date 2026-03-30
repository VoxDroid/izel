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
