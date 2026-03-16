//! Lexical analyzer for the Izel programming language.

use izel_span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Forge,
    Shape,
    Let,
    // ... more to come
    
    // Identifiers
    Ident(String),
    
    // Literals
    Int(i64),
    
    // Eof
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Lexer<'a> {
    pub source: &'a str,
    // ... DFA state
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source }
    }
}
