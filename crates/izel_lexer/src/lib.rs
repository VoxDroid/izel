//! Lexical analyzer for the Izel programming language.

use izel_span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TokenKind {
    // Keywords
    Forge,
    Shape,
    Scroll,
    Weave,
    Ward,
    Echo,
    Branch,
    Given,
    Else,
    Loop,
    Each,
    While,
    Break,
    Next,
    Give,
    Let,
    Raw,
    Bridge,
    Flow,
    Tide,
    Zone,
    Dual,
    Seek,
    Catch,
    Draw,
    Open,
    Hidden,
    Pkg,
    Pure,
    Sole,
    SelfKw,
    SelfType,
    True,
    False,
    Nil,
    As,
    In,
    Of,
    Is,
    Not,
    And,
    Or,
    Comptime,
    Static,
    Extern,
    Type,
    Alias,
    Impl,
    For,
    Bind,

    // Identifiers
    Ident,

    // Literals
    Int { base: Base, empty_int: bool },
    Float,
    Str { terminated: bool },
    InterpolatedStr { terminated: bool },
    ByteStr { terminated: bool },
    Char { terminated: bool },
    Byte { terminated: bool },

    // Sigils/Punctuation
    Tilde,          // ~
    Bang,           // !
    At,             // @
    Pipe,           // |>
    Bar,            // |
    DoubleColon,    // ::
    Arrow,          // ->
    FatArrow,       // =>
    DotDot,         // ..
    DotDotEq,       // ..=
    Dot,            // .
    Question,       // ?
    Pound,          // #
    Equal,          // =
    Plus,           // +
    Minus,          // -
    Star,           // *
    Slash,          // /
    Percent,        // %
    Caret,          // ^
    Ampersand,      // &
    Lt,             // <
    Gt,             // >
    Le,             // <=
    Ge,             // >=
    EqEq,           // ==
    NotEq,          // !=
    OrOr,           // or (already keyword)
    AndAnd,         // and (already keyword)
    
    // Delimiters
    OpenParen,      // (
    CloseParen,     // )
    OpenBrace,      // {
    CloseBrace,     // }
    OpenBracket,    // [
    CloseBracket,   // ]
    Comma,          // ,
    Semicolon,      // ;
    Colon,          // :

    // Special
    Whitespace,
    Comment,
    Unknown,
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Base {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

pub mod cursor;
pub mod lexer;
pub mod string_reader;

pub use lexer::Lexer;

#[cfg(test)]
mod tests {
    use super::*;
    use izel_span::SourceId;

    fn lex(source: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(source, SourceId(0));
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            if token.kind == TokenKind::Eof {
                break;
            }
            if token.kind != TokenKind::Whitespace {
                tokens.push(token.kind);
            }
        }
        tokens
    }

    #[test]
    fn test_strings() {
        assert_eq!(lex("\"hello\""), vec![TokenKind::Str { terminated: true }]);
        assert_eq!(lex("\"hello\\nworld\""), vec![TokenKind::Str { terminated: true }]);
        assert_eq!(lex("\"\\u{1F600}\""), vec![TokenKind::Str { terminated: true }]);
        assert_eq!(lex("\"unterminated"), vec![TokenKind::Str { terminated: false }]);
    }

    #[test]
    fn test_chars() {
        assert_eq!(lex("'a'"), vec![TokenKind::Char { terminated: true }]);
        assert_eq!(lex("'\\n'"), vec![TokenKind::Char { terminated: true }]);
        assert_eq!(lex("'\\u{1F600}'"), vec![TokenKind::Char { terminated: true }]);
    }
}
