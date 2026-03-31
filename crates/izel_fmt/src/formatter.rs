use izel_lexer::{Lexer, TokenKind};
use izel_parser::cst::{NodeKind, SyntaxElement, SyntaxNode};
use izel_parser::Parser;
use izel_span::SourceId;

/// Formats the given Izel source string into a deterministically styled string.
pub fn format_source(source: &str) -> String {
    let mut lexer = Lexer::new(source, SourceId(0));
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        let kind = token.kind;
        tokens.push(token);
        if kind == TokenKind::Eof {
            break;
        }
    }

    let mut parser = Parser::new(tokens, source.to_string());
    let root = parser.parse_source_file();

    let mut formatter = Formatter::new(source);
    formatter.format_node(&root);
    formatter.finish()
}

struct Formatter<'a> {
    source: &'a str,
    output: String,
    indent_level: usize,
    needs_indent: bool,
    last_token: Option<TokenKind>,
}

impl<'a> Formatter<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            output: String::new(),
            indent_level: 0,
            needs_indent: true,
            last_token: None,
        }
    }

    fn finish(self) -> String {
        self.output.trim_end().to_string() + "\n"
    }

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    fn push_str(&mut self, s: &str) {
        if self.needs_indent && !s.trim().is_empty() {
            self.output.push_str(&"    ".repeat(self.indent_level));
            self.needs_indent = false;
        }
        self.output.push_str(s);
        if s.ends_with('\n') {
            self.needs_indent = true;
        }
    }

    fn push_newline(&mut self) {
        if !self.output.ends_with('\n') {
            self.output.push('\n');
            self.needs_indent = true;
        }
    }

    fn push_blank_line(&mut self) {
        if !self.output.ends_with("\n\n") {
            if !self.output.ends_with('\n') {
                self.output.push('\n');
            }
            self.output.push('\n');
            self.needs_indent = true;
        }
    }

    fn push_space(&mut self) {
        if !self.output.ends_with(' ') && !self.output.ends_with('\n') {
            self.output.push(' ');
        }
    }

    fn format_node(&mut self, node: &SyntaxNode) {
        match node.kind {
            NodeKind::SourceFile => {
                for (i, child) in node.children.iter().enumerate() {
                    self.format_element(child);
                    if let SyntaxElement::Node(_) = child {
                        if i < node.children.len() - 1 {
                            self.push_blank_line();
                        }
                    }
                }
            }
            NodeKind::Block
            | NodeKind::ShapeDecl
            | NodeKind::ScrollDecl
            | NodeKind::WeaveDecl
            | NodeKind::ImplBlock => {
                for child in &node.children {
                    match child {
                        SyntaxElement::Token(t) if t.kind == TokenKind::OpenBrace => {
                            self.push_space();
                            self.format_token(t);
                            self.indent();
                            self.push_newline();
                        }
                        SyntaxElement::Token(t) if t.kind == TokenKind::CloseBrace => {
                            self.dedent();
                            if !self.output.ends_with('\n') {
                                self.push_newline();
                            }
                            self.format_token(t);
                        }
                        SyntaxElement::Token(t) if t.kind == TokenKind::Comma => {
                            self.format_token(t);
                            self.push_newline();
                        }
                        SyntaxElement::Token(t) if t.kind == TokenKind::Semicolon => {
                            self.format_token(t);
                            self.push_newline();
                        }
                        _ => {
                            self.format_element(child);

                            // For statements in a block, add newline if not already there
                            if node.kind == NodeKind::Block
                                && matches!(child, SyntaxElement::Node(n) if
                                n.kind == NodeKind::LetStmt || n.kind == NodeKind::ExprStmt ||
                                n.kind == NodeKind::GivenExpr || n.kind == NodeKind::WhileExpr ||
                                n.kind == NodeKind::LoopExpr)
                                && !self.output.ends_with('\n')
                            {
                                self.push_newline();
                            }
                        }
                    }
                }
            }
            NodeKind::Field
            | NodeKind::Variant
            | NodeKind::LetStmt
            | NodeKind::Param
            | NodeKind::GenericParam => {
                for child in &node.children {
                    self.format_element(child);
                }
            }
            _ => {
                for child in &node.children {
                    self.format_element(child);
                }
            }
        }
    }

    fn format_element(&mut self, element: &SyntaxElement) {
        match element {
            SyntaxElement::Node(n) => self.format_node(n),
            SyntaxElement::Token(t) => {
                if t.kind == TokenKind::Whitespace {
                    return;
                }
                if t.kind == TokenKind::Comment {
                    let text = &self.source[t.span.lo.0 as usize..t.span.hi.0 as usize];
                    self.push_str(text);
                    self.push_newline();
                    return;
                }
                self.format_token(t);
            }
        }
    }

    fn format_token(&mut self, token: &izel_lexer::Token) {
        let text = &self.source[token.span.lo.0 as usize..token.span.hi.0 as usize];

        // Exclude Eof
        if token.kind == TokenKind::Eof {
            return;
        }

        let is_keyword = matches!(
            token.kind,
            TokenKind::Shape
                | TokenKind::Forge
                | TokenKind::Scroll
                | TokenKind::Weave
                | TokenKind::Ward
                | TokenKind::Impl
                | TokenKind::Dual
                | TokenKind::Alias
                | TokenKind::Let
                | TokenKind::Open
                | TokenKind::Hidden
                | TokenKind::Given
                | TokenKind::Else
                | TokenKind::Loop
                | TokenKind::While
                | TokenKind::Each
                | TokenKind::In
                | TokenKind::Branch
                | TokenKind::Break
                | TokenKind::Next
                | TokenKind::Give
                | TokenKind::Pure
        );

        let space_before = matches!(
            token.kind,
            TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Equal
                | TokenKind::EqEq
                | TokenKind::NotEq
                | TokenKind::Lt
                | TokenKind::Gt
                | TokenKind::Le
                | TokenKind::Ge
                | TokenKind::Arrow
                | TokenKind::FatArrow
        );

        let space_after =
            space_before || is_keyword || matches!(token.kind, TokenKind::Comma | TokenKind::Colon);

        // Don't add space before if previous token was parenthesis or similar structure? Wait, space_before handles operators.
        if space_before {
            self.push_space();
        }

        self.push_str(text);

        if space_after {
            self.push_space();
        }

        self.last_token = Some(token.kind);
    }
}

#[cfg(test)]
mod tests {
    use super::Formatter;
    use izel_lexer::{Token, TokenKind};
    use izel_parser::cst::{NodeKind, SyntaxElement, SyntaxNode};
    use izel_span::{BytePos, SourceId, Span};

    fn token(kind: TokenKind, lo: u32, hi: u32) -> Token {
        Token::new(kind, Span::new(BytePos(lo), BytePos(hi), SourceId(0)))
    }

    #[test]
    fn formatter_helpers_update_newline_tracking() {
        let mut formatter = Formatter::new("line");

        formatter.push_str("line\n");
        assert!(formatter.needs_indent);

        formatter.push_str("next");
        formatter.push_blank_line();
        assert!(formatter.output.ends_with("\n\n"));
        assert!(formatter.needs_indent);

        let before = formatter.output.clone();
        formatter.push_blank_line();
        assert_eq!(formatter.output, before);
    }

    #[test]
    fn formatter_handles_comment_tokens_as_full_lines() {
        let mut formatter = Formatter::new("//x");
        formatter.format_element(&SyntaxElement::Token(token(TokenKind::Comment, 0, 3)));

        assert_eq!(formatter.output, "//x\n");
    }

    #[test]
    fn formatter_block_tokens_cover_comma_and_semicolon_paths() {
        let node = SyntaxNode::new(
            NodeKind::Block,
            vec![
                SyntaxElement::Token(token(TokenKind::OpenBrace, 0, 1)),
                SyntaxElement::Token(token(TokenKind::Comma, 1, 2)),
                SyntaxElement::Token(token(TokenKind::Semicolon, 2, 3)),
                SyntaxElement::Token(token(TokenKind::CloseBrace, 3, 4)),
            ],
        );
        let mut formatter = Formatter::new("{,;}");

        formatter.format_node(&node);

        assert!(formatter.output.contains(", \n") || formatter.output.contains(",\n"));
        assert!(formatter.output.contains(";\n"));
    }

    #[test]
    fn formatter_ignores_eof_token() {
        let mut formatter = Formatter::new("");
        formatter.format_token(&token(TokenKind::Eof, 0, 0));

        assert!(formatter.output.is_empty());
    }
}
