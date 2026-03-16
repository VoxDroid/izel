use crate::Parser;
use crate::cst::{NodeKind, SyntaxElement, SyntaxNode};
use izel_lexer::TokenKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    None,
    Pipeline,   // |>
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
    Cast,       // as !
    Call,       // . () [] ::
}

impl Precedence {
    pub fn from_kind(kind: TokenKind) -> Self {
        match kind {
            TokenKind::Pipe => Precedence::Pipeline,
            TokenKind::Or => Precedence::LogicalOr,
            TokenKind::And => Precedence::LogicalAnd,
            TokenKind::EqEq | TokenKind::NotEq | TokenKind::Is => Precedence::Equality,
            TokenKind::Lt | TokenKind::Gt | TokenKind::Le | TokenKind::Ge => Precedence::Comparison,
            TokenKind::Caret => Precedence::BitXor,
            TokenKind::Ampersand => Precedence::BitAnd,
            TokenKind::Plus | TokenKind::Minus => Precedence::Sum,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Product,
            TokenKind::As | TokenKind::Bang => Precedence::Cast,
            TokenKind::Dot | TokenKind::OpenParen | TokenKind::OpenBracket | TokenKind::DoubleColon => Precedence::Call,
            _ => Precedence::None,
        }
    }
}

impl Parser {
    pub fn parse_expr(&mut self, min_precedence: Precedence) -> SyntaxNode {
        let mut lhs = self.parse_primary();
        
        loop {
            let kind = self.current_kind();
            let prec = Precedence::from_kind(kind);
            if prec <= min_precedence || prec == Precedence::None {
                break;
            }
            
            if kind == TokenKind::OpenParen {
                lhs = self.parse_call(lhs);
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
            TokenKind::Int { .. } | TokenKind::Float | TokenKind::Str { .. } | TokenKind::True | TokenKind::False | TokenKind::Nil => {
                children.push(SyntaxElement::Token(self.bump()));
                SyntaxNode::new(NodeKind::Literal, children)
            }
            TokenKind::Ident => {
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
            TokenKind::Minus | TokenKind::Not | TokenKind::Tilde | TokenKind::Star | TokenKind::Ampersand => {
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
}
