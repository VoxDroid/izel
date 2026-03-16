use crate::cursor::{Cursor, EOF_CHAR};
use crate::{Base, Token, TokenKind};
use izel_span::{BytePos, SourceId, Span};

pub struct Lexer<'a> {
    source: &'a str,
    cursor: Cursor<'a>,
    source_id: SourceId,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, source_id: SourceId) -> Self {
        Self {
            source,
            cursor: Cursor::new(source),
            source_id,
        }
    }

    pub fn next_token(&mut self) -> Token {
        let start = self.pos();
        if self.cursor.is_eof() {
            return Token::new(TokenKind::Eof, self.make_span(start, start));
        }

        let first_char = self.cursor.first();
        
        // Handle whitespace
        if first_char.is_whitespace() {
            self.cursor.bump();
            self.cursor.eat_while(|c| c.is_whitespace());
            return Token::new(TokenKind::Whitespace, self.make_span(start, self.pos()));
        }

        // Handle comments
        if first_char == '/' {
            match self.cursor.second() {
                '/' => {
                    self.cursor.bump();
                    self.cursor.bump();
                    self.cursor.eat_while(|c| c != '\n' && c != EOF_CHAR);
                    return Token::new(TokenKind::Comment, self.make_span(start, self.pos()));
                }
                '~' => {
                    self.cursor.bump();
                    self.cursor.bump();
                    self.eat_multi_line_comment();
                    return Token::new(TokenKind::Comment, self.make_span(start, self.pos()));
                }
                _ => {}
            }
        }

        // Standard tokens
        let first = self.cursor.bump().unwrap();
        let kind = match first {
            // Sigils & Punctuation
            '~' => TokenKind::Tilde,
            '!' => {
                if self.cursor.first() == '=' {
                    self.cursor.bump();
                    TokenKind::NotEq
                } else {
                    TokenKind::Bang
                }
            }
            '@' => TokenKind::At,
            '|' => {
                if self.cursor.first() == '>' {
                    self.cursor.bump();
                    TokenKind::Pipe
                } else {
                    TokenKind::Bar
                }
            }
            ':' => {
                if self.cursor.first() == ':' {
                    self.cursor.bump();
                    TokenKind::DoubleColon
                } else {
                    TokenKind::Colon
                }
            }
            '-' => {
                if self.cursor.first() == '>' {
                    self.cursor.bump();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            '=' => {
                if self.cursor.first() == '>' {
                    self.cursor.bump();
                    TokenKind::FatArrow
                } else if self.cursor.first() == '=' {
                    self.cursor.bump();
                    TokenKind::EqEq
                } else {
                    TokenKind::Equal
                }
            }
            '.' => {
                if self.cursor.first() == '.' {
                    self.cursor.bump();
                    if self.cursor.first() == '=' {
                        self.cursor.bump();
                        TokenKind::DotDotEq
                    } else {
                        TokenKind::DotDot
                    }
                } else {
                    TokenKind::Dot
                }
            }
            '?' => TokenKind::Question,
            '#' => TokenKind::Pound,
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '^' => TokenKind::Caret,
            '&' => TokenKind::Ampersand,
            '<' => {
                if self.cursor.first() == '=' {
                    self.cursor.bump();
                    TokenKind::Le
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.cursor.first() == '=' {
                    self.cursor.bump();
                    TokenKind::Ge
                } else {
                    TokenKind::Gt
                }
            }
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            '{' => TokenKind::OpenBrace,
            '}' => TokenKind::CloseBrace,
            '[' => TokenKind::OpenBracket,
            ']' => TokenKind::CloseBracket,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,

            // Identifiers, Keywords, Raw/Byte Strings
            c if is_ident_start(c) => self.lex_ident_or_keyword(first),

            // Numeric Literals
            c if c.is_ascii_digit() => self.lex_number(first),

            // Strings & Chars
            '"' => self.lex_string(),
            '\'' => self.lex_char(),
            '`' => TokenKind::Unknown, // Interpolated string start handled separately if needed

            _ => TokenKind::Unknown,
        };

        let end = self.pos();
        Token::new(kind, self.make_span(start, end))
    }

    fn lex_ident_or_keyword(&mut self, first: char) -> TokenKind {
        let (_, text) = self.eat_ident_text(first);

        // Check for raw/byte prefixes
        if text == "r" && (self.cursor.first() == '"' || self.cursor.first() == '#') {
            return self.lex_raw_string();
        }
        if text == "b" && (self.cursor.first() == '"' || self.cursor.first() == '\'') {
            let next = self.cursor.bump().unwrap();
            if next == '"' {
                return self.lex_byte_string();
            } else {
                return self.lex_byte_literal();
            }
        }

        match text {
            "forge"    => TokenKind::Forge,
            "shape"    => TokenKind::Shape,
            "scroll"   => TokenKind::Scroll,
            "weave"    => TokenKind::Weave,
            "ward"     => TokenKind::Ward,
            "echo"     => TokenKind::Echo,
            "branch"   => TokenKind::Branch,
            "given"    => TokenKind::Given,
            "else"     => TokenKind::Else,
            "loop"     => TokenKind::Loop,
            "each"     => TokenKind::Each,
            "while"    => TokenKind::While,
            "break"    => TokenKind::Break,
            "next"     => TokenKind::Next,
            "give"     => TokenKind::Give,
            "let"      => TokenKind::Let,
            "raw"      => TokenKind::Raw,
            "bridge"   => TokenKind::Bridge,
            "flow"     => TokenKind::Flow,
            "tide"     => TokenKind::Tide,
            "zone"     => TokenKind::Zone,
            "dual"     => TokenKind::Dual,
            "seek"     => TokenKind::Seek,
            "catch"    => TokenKind::Catch,
            "draw"     => TokenKind::Draw,
            "open"     => TokenKind::Open,
            "hidden"   => TokenKind::Hidden,
            "pkg"      => TokenKind::Pkg,
            "pure"     => TokenKind::Pure,
            "sole"     => TokenKind::Sole,
            "self"     => TokenKind::SelfKw,
            "Self"     => TokenKind::SelfType,
            "true"     => TokenKind::True,
            "false"    => TokenKind::False,
            "nil"      => TokenKind::Nil,
            "as"       => TokenKind::As,
            "in"       => TokenKind::In,
            "of"       => TokenKind::Of,
            "is"       => TokenKind::Is,
            "not"      => TokenKind::Not,
            "and"      => TokenKind::And,
            "or"       => TokenKind::Or,
            "comptime" => TokenKind::Comptime,
            "static"   => TokenKind::Static,
            "extern"   => TokenKind::Extern,
            "type"     => TokenKind::Type,
            "alias"    => TokenKind::Alias,
            "impl"     => TokenKind::Impl,
            "for"      => TokenKind::For,
            "bind"     => TokenKind::Bind,
            _          => TokenKind::Ident,
        }
    }

    fn eat_ident_text(&mut self, first: char) -> (usize, &'a str) {
        let start = self.cursor.pos_within(self.source) - first.len_utf8();
        self.cursor.eat_while(is_ident_continue);
        let end = self.cursor.pos_within(self.source);
        (start, &self.source[start..end])
    }

    fn lex_number(&mut self, first: char) -> TokenKind {
        let mut base = Base::Decimal;
        if first == '0' {
            match self.cursor.first() {
                'x' => { self.cursor.bump(); base = Base::Hexadecimal; }
                'b' => { self.cursor.bump(); base = Base::Binary; }
                'o' => { self.cursor.bump(); base = Base::Octal; }
                _ => {}
            }
        }

        self.cursor.eat_while(|c| c.is_ascii_digit() || c == '_' || (base == Base::Hexadecimal && c.is_ascii_hexdigit()));
        
        if self.cursor.first() == '.' && self.cursor.second() != '.' {
            self.cursor.bump();
            self.cursor.eat_while(|c| c.is_ascii_digit() || c == '_');
            TokenKind::Float
        } else {
            TokenKind::Int { base, empty_int: false }
        }
    }

    fn lex_string(&mut self) -> TokenKind {
        let mut terminated = false;
        while let Some(c) = self.cursor.bump() {
            if c == '"' {
                terminated = true;
                break;
            }
            if c == '\\' {
                self.cursor.bump(); // Skip any char for now, simple escape handler
            }
        }
        TokenKind::Str { terminated }
    }

    fn lex_byte_string(&mut self) -> TokenKind {
        let mut terminated = false;
        while let Some(c) = self.cursor.bump() {
            if c == '"' {
                terminated = true;
                break;
            }
            if c == '\\' {
                self.cursor.bump();
            }
        }
        TokenKind::ByteStr { terminated }
    }

    fn lex_char(&mut self) -> TokenKind {
        let mut terminated = false;
        if let Some(c) = self.cursor.bump() {
            if c == '\\' {
                self.cursor.bump();
            }
            if self.cursor.first() == '\'' {
                self.cursor.bump();
                terminated = true;
            }
        }
        TokenKind::Char { terminated }
    }

    fn lex_byte_literal(&mut self) -> TokenKind {
        let mut terminated = false;
        if let Some(c) = self.cursor.bump() {
            if c == '\\' {
                self.cursor.bump();
            }
            if self.cursor.first() == '\'' {
                self.cursor.bump();
                terminated = true;
            }
        }
        TokenKind::Byte { terminated }
    }

    fn lex_raw_string(&mut self) -> TokenKind {
        let mut hashes = 0;
        while self.cursor.first() == '#' {
            self.cursor.bump();
            hashes += 1;
        }

        if self.cursor.first() != '"' {
            return TokenKind::Unknown;
        }
        self.cursor.bump();

        let mut terminated = false;
        loop {
            match self.cursor.bump() {
                Some('"') => {
                    let mut closing_hashes = 0;
                    while self.cursor.first() == '#' && closing_hashes < hashes {
                        self.cursor.bump();
                        closing_hashes += 1;
                    }
                    if closing_hashes == hashes {
                        terminated = true;
                        break;
                    }
                }
                Some(_) => {}
                None => break,
            }
        }
        TokenKind::Str { terminated }
    }

    fn eat_multi_line_comment(&mut self) {
        let mut depth = 1;
        while let Some(c) = self.cursor.bump() {
            match c {
                '/' if self.cursor.first() == '~' => {
                    self.cursor.bump();
                    depth += 1;
                }
                '~' if self.cursor.first() == '/' => {
                    self.cursor.bump();
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    fn pos(&self) -> BytePos {
        BytePos(self.cursor.pos_within(self.source) as u32)
    }

    fn make_span(&self, start: BytePos, end: BytePos) -> Span {
        Span::new(start, end, self.source_id)
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
