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

pub use lexer::Lexer;
