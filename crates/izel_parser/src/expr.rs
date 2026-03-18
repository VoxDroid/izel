use crate::Parser;
use crate::cst::{NodeKind, SyntaxElement, SyntaxNode};
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
            TokenKind::Bang | TokenKind::Dot | TokenKind::OpenParen | TokenKind::OpenBracket | TokenKind::DoubleColon | TokenKind::OpenBrace | TokenKind::Question => Precedence::Call,
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

            let kind = self.tokens.get(self.pos + offset).map(|t| t.kind).unwrap_or(TokenKind::Eof);
            let prec = Precedence::from_kind(kind);
            if prec <= min_precedence || prec == Precedence::None {
                break;
            }

            // Consume the trivia we skipped
            for _ in 0..offset { self.bump(); }
            
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
            TokenKind::Int { .. } | TokenKind::Float | TokenKind::Str { .. } | TokenKind::InterpolatedStr { .. } | TokenKind::True | TokenKind::False | TokenKind::Nil => {
                children.push(SyntaxElement::Token(self.bump()));
                SyntaxNode::new(NodeKind::Literal, children)
            }
            TokenKind::Ident | TokenKind::SelfKw | TokenKind::SelfType => {
                children.push(SyntaxElement::Token(self.bump()));
                SyntaxNode::new(NodeKind::Ident, children)
            }
            TokenKind::OpenParen => {
                children.push(SyntaxElement::Token(self.bump()));
                children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::CloseParen {
                    children.push(SyntaxElement::Token(self.bump()));
                }
                SyntaxNode::new(NodeKind::ParenExpr, children)
            }
            TokenKind::Minus | TokenKind::Not | TokenKind::Tilde | TokenKind::Star | TokenKind::Ampersand | TokenKind::AmpersandTilde => {
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
            TokenKind::Bind => {
                children.push(SyntaxElement::Token(self.bump()));
                self.parse_bind_expr(children)
            }
            TokenKind::Raw => {
                self.parse_raw_expr()
            }
            _ => {
                children.push(SyntaxElement::Token(self.bump()));
                SyntaxNode::new(NodeKind::Error, children)
            }
        }
    }

    fn parse_binary(&mut self, lhs: SyntaxNode, prec: Precedence) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(lhs)];
        children.extend(self.eat_trivia().into_iter());
        children.push(SyntaxElement::Token(self.bump())); // Operator
        children.push(SyntaxElement::Node(self.parse_expr(prec)));
        SyntaxNode::new(NodeKind::BinaryExpr, children)
    }

    fn parse_call(&mut self, lhs: SyntaxNode) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(lhs)];
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::OpenParen {
            children.push(SyntaxElement::Token(self.bump())); // (
            while self.current_kind() != TokenKind::CloseParen && self.current_kind() != TokenKind::Eof {
                children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::Comma {
                    children.push(SyntaxElement::Token(self.bump()));
                    children.extend(self.eat_trivia().into_iter());
                }
            }
            if self.current_kind() == TokenKind::CloseParen {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }
        SyntaxNode::new(NodeKind::CallExpr, children)
    }

    fn parse_path_or_turbofish(&mut self, lhs: SyntaxNode) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(lhs)];
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::DoubleColon {
            children.push(SyntaxElement::Token(self.bump())); // ::
            children.extend(self.eat_trivia().into_iter());
            if self.current_kind() == TokenKind::Lt {
                children.push(SyntaxElement::Node(self.parse_generic_args()));
            } else if self.current_kind() == TokenKind::Ident {
                children.push(SyntaxElement::Token(self.bump())); // component
            }
        }
        SyntaxNode::new(NodeKind::PathExpr, children)
    }

    fn parse_struct_literal_expr(&mut self, lhs: SyntaxNode) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(lhs)];
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Token(self.bump()));
            while self.current_kind() != TokenKind::CloseBrace && self.current_kind() != TokenKind::Eof {
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
        }
        SyntaxNode::new(NodeKind::StructLiteral, children)
    }

    fn parse_member_access(&mut self, lhs: SyntaxNode) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(lhs)];
        children.extend(self.eat_trivia().into_iter());
        children.push(SyntaxElement::Token(self.bump())); // .
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::Ident {
            children.push(SyntaxElement::Token(self.bump()));
        }
        SyntaxNode::new(NodeKind::MemberExpr, children)
    }

    pub(crate) fn parse_postfix_bang(&mut self, lhs: SyntaxNode) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(lhs)];
        children.extend(self.eat_trivia().into_iter());
        children.push(SyntaxElement::Token(self.bump())); // !
        SyntaxNode::new(NodeKind::UnaryExpr, children)
    }

    fn parse_optional_chaining(&mut self, lhs: SyntaxNode) -> SyntaxNode {
        let mut children = vec![SyntaxElement::Node(lhs)];
        children.extend(self.eat_trivia().into_iter());
        children.push(SyntaxElement::Token(self.bump())); // ?
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::Dot {
             children.push(SyntaxElement::Token(self.bump())); // .
             children.extend(self.eat_trivia().into_iter());
             if self.current_kind() == TokenKind::Ident {
                  children.push(SyntaxElement::Token(self.bump()));
             }
        }
        SyntaxNode::new(NodeKind::MemberExpr, children)
    }
}
