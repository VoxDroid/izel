use crate::cst::{NodeKind, SyntaxElement, SyntaxNode};
use crate::Parser;
use izel_lexer::TokenKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    None,
    Pipeline,   // |>
    Coalesce,   // ??
    LogicalOr,  // or
    LogicalAnd, // and
    Equality,   // == != is
    Comparison, // < > <= >=
    BitOr,      // |
    BitXor,     // ^
    BitAnd,     // &
    Shift,      // << >>
    Sum,        // + -
    Product,    // * / %
    Unary,      // - not ~ * & &~
    Cast,       // as
    Type,       // !
    Call,       // . () [] :: ? {
}

impl Precedence {
    pub fn from_kind(kind: TokenKind) -> Self {
        match kind {
            TokenKind::Pipe => Precedence::Pipeline,
            TokenKind::QuestionQuestion => Precedence::Coalesce,
            TokenKind::Or => Precedence::LogicalOr,
            TokenKind::And => Precedence::LogicalAnd,
            TokenKind::EqEq | TokenKind::NotEq | TokenKind::Is => Precedence::Equality,
            TokenKind::Lt | TokenKind::Gt | TokenKind::Le | TokenKind::Ge => Precedence::Comparison,
            TokenKind::Caret => Precedence::BitXor,
            TokenKind::Ampersand => Precedence::BitAnd,
            TokenKind::Plus | TokenKind::Minus => Precedence::Sum,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Product,
            TokenKind::As => Precedence::Cast,
            TokenKind::Bang
            | TokenKind::Dot
            | TokenKind::OpenParen
            | TokenKind::OpenBracket
            | TokenKind::DoubleColon
            | TokenKind::OpenBrace
            | TokenKind::Question => Precedence::Call,
            _ => Precedence::None,
        }
    }
}

impl Parser {
    pub fn parse_expr(&mut self, min_precedence: Precedence) -> SyntaxNode {
        let mut lhs = self.parse_primary();

        loop {
            // Skip trivia but we don't necessarily have a place to put it here
            // without changing the structure significantly.
            // Let's just peer at the next non-trivia token.
            let mut offset = 0;
            while let Some(t) = self.tokens.get(self.pos + offset) {
                if t.kind == TokenKind::Whitespace || t.kind == TokenKind::Comment {
                    offset += 1;
                } else {
                    break;
                }
            }

            let kind = self
                .tokens
                .get(self.pos + offset)
                .map(|t| t.kind)
                .unwrap_or(TokenKind::Eof);
            let prec = Precedence::from_kind(kind);
            if prec <= min_precedence || prec == Precedence::None {
                break;
            }

            // Consume the trivia we skipped
            for _ in 0..offset {
                self.bump();
            }

            if kind == TokenKind::OpenParen {
                lhs = self.parse_call(lhs);
            } else if kind == TokenKind::DoubleColon {
                lhs = self.parse_path_or_turbofish(lhs);
            } else if kind == TokenKind::OpenBrace {
                lhs = self.parse_struct_literal_expr(lhs);
            } else if kind == TokenKind::Question {
                lhs = self.parse_optional_chaining(lhs);
            } else if kind == TokenKind::Dot {
                lhs = self.parse_member_access(lhs);
            } else if kind == TokenKind::Bang {
                lhs = self.parse_postfix_bang(lhs);
            } else {
                lhs = self.parse_binary(lhs, prec);
            }
        }

        lhs
    }

    fn parse_primary(&mut self) -> SyntaxNode {
        let mut children = self.eat_trivia();
        let kind = self.current_kind();
        match kind {
            TokenKind::Int { .. }
            | TokenKind::Float
            | TokenKind::Str { .. }
            | TokenKind::InterpolatedStr { .. }
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Nil => {
                children.push(SyntaxElement::Token(self.bump()));
                SyntaxNode::new(NodeKind::Literal, children)
            }
            TokenKind::Ident | TokenKind::SelfKw | TokenKind::SelfType => {
                let token = self.bump();
                children.push(SyntaxElement::Token(token));

                let mut offset = 0;
                while let Some(t) = self.tokens.get(self.pos + offset) {
                    if t.kind == TokenKind::Whitespace || t.kind == TokenKind::Comment {
                        offset += 1;
                    } else {
                        break;
                    }
                }

                if self
                    .tokens
                    .get(self.pos + offset)
                    .is_some_and(|next| next.kind == TokenKind::Bang)
                {
                    let mut after_bang_offset = offset + 1;
                    while let Some(next) = self.tokens.get(self.pos + after_bang_offset) {
                        if next.kind == TokenKind::Whitespace || next.kind == TokenKind::Comment {
                            after_bang_offset += 1;
                        } else {
                            break;
                        }
                    }

                    let is_macro_invocation = self
                        .tokens
                        .get(self.pos + after_bang_offset)
                        .is_some_and(|next| {
                            matches!(next.kind, TokenKind::OpenParen | TokenKind::OpenBracket)
                        });

                    if is_macro_invocation {
                        for _ in 0..offset {
                            children.push(SyntaxElement::Token(self.bump()));
                        }

                        children.push(SyntaxElement::Token(self.bump())); // !
                        children.extend(self.eat_trivia());

                        let open = self.current_kind();
                        let close = if open == TokenKind::OpenParen {
                            TokenKind::CloseParen
                        } else {
                            TokenKind::CloseBracket
                        };

                        children.push(SyntaxElement::Token(self.bump()));
                        children.extend(self.eat_trivia());

                        while self.current_kind() != close && self.current_kind() != TokenKind::Eof
                        {
                            let mut arg_children = self.eat_trivia();
                            arg_children
                                .push(SyntaxElement::Node(self.parse_expr(Precedence::None)));

                            children.push(SyntaxElement::Node(SyntaxNode::new(
                                NodeKind::Arg,
                                arg_children,
                            )));
                            children.extend(self.eat_trivia());

                            if self.current_kind() == TokenKind::Comma {
                                children.push(SyntaxElement::Token(self.bump()));
                                children.extend(self.eat_trivia());
                            }
                        }

                        if self.current_kind() == close {
                            children.push(SyntaxElement::Token(self.bump()));
                        }

                        return SyntaxNode::new(NodeKind::MacroCall, children);
                    }
                }

                SyntaxNode::new(NodeKind::Ident, children)
            }
            TokenKind::OpenParen => {
                children.push(SyntaxElement::Token(self.bump()));
                children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
                children.extend(self.eat_trivia());
                if self.current_kind() == TokenKind::CloseParen {
                    children.push(SyntaxElement::Token(self.bump()));
                }
                SyntaxNode::new(NodeKind::ParenExpr, children)
            }
            TokenKind::Minus
            | TokenKind::Not
            | TokenKind::Tilde
            | TokenKind::Star
            | TokenKind::Tide
            | TokenKind::Ampersand
            | TokenKind::AmpersandTilde => {
                children.push(SyntaxElement::Token(self.bump()));
                children.push(SyntaxElement::Node(self.parse_expr(Precedence::Unary)));
                SyntaxNode::new(NodeKind::UnaryExpr, children)
            }
            TokenKind::Given => {
                children.push(SyntaxElement::Token(self.bump()));
                self.parse_given_expr(children)
            }
            TokenKind::Branch => {
                children.push(SyntaxElement::Token(self.bump()));
                self.parse_branch_expr(children)
            }
            TokenKind::Loop => {
                children.push(SyntaxElement::Token(self.bump()));
                self.parse_loop_expr(children)
            }
            TokenKind::While => {
                children.push(SyntaxElement::Token(self.bump()));
                self.parse_while_expr(children)
            }
            TokenKind::Each => {
                children.push(SyntaxElement::Token(self.bump()));
                self.parse_each_expr(children)
            }
            TokenKind::Zone => {
                children.push(SyntaxElement::Token(self.bump()));
                self.parse_zone_expr(children)
            }
            TokenKind::Bind => {
                children.push(SyntaxElement::Token(self.bump()));
                self.parse_bind_expr(children)
            }
            TokenKind::Seek => {
                children.push(SyntaxElement::Token(self.bump()));
                self.parse_seek_expr(children)
            }
            TokenKind::Raw => self.parse_raw_expr(),
            _ => {
                children.push(SyntaxElement::Token(self.bump()));
                SyntaxNode::new(NodeKind::Error, children)
            }
        }
    }

    fn parse_binary(&mut self, lhs: SyntaxNode, prec: Precedence) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(lhs)];
        children.extend(self.eat_trivia());
        children.push(SyntaxElement::Token(self.bump())); // Operator
        children.push(SyntaxElement::Node(self.parse_expr(prec)));
        SyntaxNode::new(NodeKind::BinaryExpr, children)
    }

    fn parse_call(&mut self, lhs: SyntaxNode) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(lhs)];
        children.extend(self.eat_trivia());
        children.push(SyntaxElement::Token(self.bump())); // (
        while self.current_kind() != TokenKind::CloseParen && self.current_kind() != TokenKind::Eof
        {
            let mut arg_children = self.eat_trivia();

            // Check if it's a named argument: label : expr
            let mut is_named = false;
            if self.is_naming_ident() {
                let mut offset = 1;
                while let Some(t) = self.tokens.get(self.pos + offset) {
                    if t.kind == TokenKind::Whitespace || t.kind == TokenKind::Comment {
                        offset += 1;
                    } else {
                        break;
                    }
                }
                is_named = self
                    .tokens
                    .get(self.pos + offset)
                    .is_some_and(|t| t.kind == TokenKind::Colon);
            }

            if is_named {
                arg_children.push(SyntaxElement::Token(self.bump())); // label
                arg_children.extend(self.eat_trivia());
                arg_children.push(SyntaxElement::Token(self.bump())); // :
                arg_children.extend(self.eat_trivia());
                arg_children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
            } else {
                arg_children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
            }

            children.push(SyntaxElement::Node(SyntaxNode::new(
                NodeKind::Arg,
                arg_children,
            )));
            children.extend(self.eat_trivia().into_iter());
            if self.current_kind() == TokenKind::Comma {
                children.push(SyntaxElement::Token(self.bump()));
                children.extend(self.eat_trivia().into_iter());
            }
        }
        if self.current_kind() == TokenKind::CloseParen {
            children.push(SyntaxElement::Token(self.bump()));
        }
        SyntaxNode::new(NodeKind::CallExpr, children)
    }

    fn parse_path_or_turbofish(&mut self, lhs: SyntaxNode) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(lhs)];
        children.extend(self.eat_trivia());
        children.push(SyntaxElement::Token(self.bump())); // ::
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Lt {
            children.push(SyntaxElement::Node(self.parse_generic_args()));
        } else if self.current_kind() == TokenKind::Ident {
            children.push(SyntaxElement::Token(self.bump())); // component
        }
        SyntaxNode::new(NodeKind::PathExpr, children)
    }

    fn parse_struct_literal_expr(&mut self, lhs: SyntaxNode) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(lhs)];
        children.extend(self.eat_trivia());
        children.push(SyntaxElement::Token(self.bump()));
        while self.current_kind() != TokenKind::CloseBrace && self.current_kind() != TokenKind::Eof
        {
            // simple field: expr
            children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
            children.extend(self.eat_trivia().into_iter());
            if self.current_kind() == TokenKind::Colon {
                children.push(SyntaxElement::Token(self.bump()));
                children.extend(self.eat_trivia().into_iter());
                children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
            }
            children.extend(self.eat_trivia().into_iter());
            if self.current_kind() == TokenKind::Comma {
                children.push(SyntaxElement::Token(self.bump()));
            }
            children.extend(self.eat_trivia().into_iter());
        }
        if self.current_kind() == TokenKind::CloseBrace {
            children.push(SyntaxElement::Token(self.bump()));
        }
        SyntaxNode::new(NodeKind::StructLiteral, children)
    }

    fn parse_member_access(&mut self, lhs: SyntaxNode) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(lhs)];
        children.extend(self.eat_trivia());
        children.push(SyntaxElement::Token(self.bump())); // .
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Ident {
            children.push(SyntaxElement::Token(self.bump()));
        }
        SyntaxNode::new(NodeKind::MemberExpr, children)
    }

    pub(crate) fn parse_postfix_bang(&mut self, lhs: SyntaxNode) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(lhs)];
        children.extend(self.eat_trivia());
        children.push(SyntaxElement::Token(self.bump())); // !

        let mut peek_trivia = Vec::new();
        let mut offset = 0;
        while let Some(t) = self.tokens.get(self.pos + offset) {
            if t.kind == TokenKind::Whitespace || t.kind == TokenKind::Comment {
                peek_trivia.push(SyntaxElement::Token(*t));
                offset += 1;
            } else {
                break;
            }
        }

        if self
            .tokens
            .get(self.pos + offset)
            .is_some_and(|t| t.kind == TokenKind::Or)
        {
            // Consume trivia and 'or'
            for _ in 0..offset {
                children.push(SyntaxElement::Token(self.bump()));
            }
            children.push(SyntaxElement::Token(self.bump())); // or

            // Parse the context expression
            let context_expr = self.parse_expr(Precedence::None);
            children.push(SyntaxElement::Node(context_expr));
        }

        SyntaxNode::new(NodeKind::CascadeExpr, children)
    }

    fn parse_optional_chaining(&mut self, lhs: SyntaxNode) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(lhs)];
        children.extend(self.eat_trivia());
        children.push(SyntaxElement::Token(self.bump())); // ?
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Dot {
            children.push(SyntaxElement::Token(self.bump())); // .
            children.extend(self.eat_trivia());
            if self.current_kind() == TokenKind::Ident {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }
        SyntaxNode::new(NodeKind::MemberExpr, children)
    }

    pub fn parse_zone_expr(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // zone name
        }
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Node(self.parse_block()));
        }
        SyntaxNode::new(NodeKind::ZoneExpr, children)
    }

    pub fn parse_seek_expr(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia());
        children.push(SyntaxElement::Node(self.parse_block()));
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Catch {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia());
            if self.is_naming_ident() {
                children.push(SyntaxElement::Token(self.bump())); // error name
            }
            children.extend(self.eat_trivia());
            children.push(SyntaxElement::Node(self.parse_block()));
        }
        SyntaxNode::new(NodeKind::SeekExpr, children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use izel_lexer::Lexer;

    fn parse_test_expr(src: &str, min_prec: Precedence) -> SyntaxNode {
        let source_id = izel_span::SourceId(0);
        let mut lexer = Lexer::new(src, source_id);
        let mut tokens = Vec::new();
        loop {
            let t = lexer.next_token();
            if t.kind == TokenKind::Eof {
                tokens.push(t);
                break;
            }
            tokens.push(t);
        }
        let mut parser = Parser::new(tokens, src.to_string());
        parser.parse_expr(min_prec)
    }

    #[test]
    fn test_parse_cascade_bang_only() {
        let node = parse_test_expr("foo!", Precedence::None);
        assert_eq!(node.kind, NodeKind::CascadeExpr);
        // Expect: expr, !
        assert_eq!(node.children.len(), 2);
    }

    #[test]
    fn test_parse_cascade_bang_or_msg() {
        let node = parse_test_expr("foo! or \"error\"", Precedence::None);
        assert_eq!(node.kind, NodeKind::CascadeExpr);
        // Exact layout depends on trivia handling, but it will have > 2 children
        assert!(node.children.len() > 2);
    }

    #[test]
    fn test_parse_macro_call_here() {
        let node = parse_test_expr("here!()", Precedence::None);
        assert_eq!(node.kind, NodeKind::MacroCall);
    }

    #[test]
    fn test_precedence_maps_bitwise_tokens() {
        assert_eq!(Precedence::from_kind(TokenKind::Caret), Precedence::BitXor);
        assert_eq!(
            Precedence::from_kind(TokenKind::Ampersand),
            Precedence::BitAnd
        );
    }

    #[test]
    fn test_parse_macro_call_with_whitespace_and_bracket_args() {
        let node = parse_test_expr("here ! [x, y]", Precedence::None);
        assert_eq!(node.kind, NodeKind::MacroCall);

        let arg_count = node
            .children
            .iter()
            .filter(|child| matches!(child, SyntaxElement::Node(n) if n.kind == NodeKind::Arg))
            .count();
        assert_eq!(arg_count, 2);
    }

    #[test]
    fn test_parse_ident_bang_without_macro_invocation() {
        let node = parse_test_expr("here!oops", Precedence::None);
        assert_eq!(node.kind, NodeKind::CascadeExpr);
    }

    #[test]
    fn test_parse_call_named_arg_path_and_struct_literal() {
        let call = parse_test_expr("f(label: 1, 2)", Precedence::None);
        assert_eq!(call.kind, NodeKind::CallExpr);

        let path = parse_test_expr("pkg::item", Precedence::None);
        assert_eq!(path.kind, NodeKind::PathExpr);

        let struct_lit = parse_test_expr("Point { x: 1, y: 2 }", Precedence::None);
        assert_eq!(struct_lit.kind, NodeKind::StructLiteral);
    }
}
