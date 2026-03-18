pub mod ast;
pub mod contracts;
pub mod cst;
pub mod eval;
pub mod expr;

use crate::cst::{NodeKind, SyntaxElement, SyntaxNode};
use crate::expr::Precedence;
use izel_lexer::{Token, TokenKind};

pub struct Parser {
    pub tokens: Vec<Token>,
    pub pos: usize,
    pub source: String,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0, source: String::new() }
    }

    /// Parses the entire token stream into a SourceFile CST node.
    pub fn parse_source_file(&mut self) -> SyntaxNode {
        let mut children = vec![];
        while self.current_kind() != TokenKind::Eof {
            let start_pos = self.pos;
            let decl = self.parse_decl();
            children.push(SyntaxElement::Node(decl));
            children.extend(self.eat_trivia().into_iter());

            if self.pos <= start_pos {
                self.bump();
            }
        }
        SyntaxNode::new(NodeKind::SourceFile, children)
    }

    pub fn parse_decl(&mut self) -> SyntaxNode {
        let mut children = self.eat_trivia();

        // Handle attributes (@attr or @attr(args) or #[attr])
        if self.current_kind() == TokenKind::At || self.current_kind() == TokenKind::Pound {
            children.push(SyntaxElement::Node(self.parse_attributes()));
            children.extend(self.eat_trivia().into_iter());
        }

        // Handle modifiers (open/hidden/pure/sole/etc)
        while matches!(
            self.current_kind(),
            TokenKind::Open | TokenKind::Hidden | TokenKind::Pure | TokenKind::Sole
        ) {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia().into_iter());
        }

        match self.current_kind() {
            TokenKind::Forge => {
                children.push(SyntaxElement::Token(self.bump())); // forge
                self.parse_forge_after_keyword(children)
            }
            TokenKind::Shape => {
                children.push(SyntaxElement::Token(self.bump())); // shape
                self.parse_shape_after_keyword(children)
            }
            TokenKind::Scroll => {
                children.push(SyntaxElement::Token(self.bump())); // scroll
                self.parse_scroll_after_keyword(children)
            }
            TokenKind::Ward => {
                children.push(SyntaxElement::Token(self.bump())); // ward
                self.parse_ward_after_keyword(children)
            }
            TokenKind::Dual => {
                children.push(SyntaxElement::Token(self.bump())); // dual
                self.parse_dual_after_keyword(children)
            }
            TokenKind::Weave => {
                children.push(SyntaxElement::Token(self.bump())); // weave
                self.parse_weave_after_keyword(children)
            }
            TokenKind::Impl => {
                children.push(SyntaxElement::Token(self.bump())); // impl
                self.parse_impl_after_keywords(children)
            }
            TokenKind::Type => {
                children.push(SyntaxElement::Token(self.bump())); // type
                self.parse_type_after_keyword(children)
            }
            TokenKind::Draw => {
                children.push(SyntaxElement::Token(self.bump())); // draw
                self.parse_draw_after_keyword(children)
            }
            _ => self.parse_stmt_after_trivia(children),
        }
    }

    fn parse_type_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // name
        }
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::Equal {
            children.push(SyntaxElement::Token(self.bump())); // =
            children.extend(self.eat_trivia().into_iter());
            children.push(SyntaxElement::Node(self.parse_type()));
        }
        SyntaxNode::new(NodeKind::TypeAlias, children)
    }

    fn parse_forge_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // name
        }

        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::Lt {
            children.push(SyntaxElement::Node(self.parse_generic_params()));
        }

        // Params
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::OpenParen {
            children.push(SyntaxElement::Token(self.bump())); // (
            children.extend(self.eat_trivia().into_iter());
            while self.current_kind() != TokenKind::CloseParen
                && self.current_kind() != TokenKind::Eof
            {
                let start = self.pos;
                children.push(SyntaxElement::Node(self.parse_param()));
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::Comma {
                    children.push(SyntaxElement::Token(self.bump()));
                    children.extend(self.eat_trivia().into_iter());
                }
                if self.pos == start {
                    self.bump();
                } // Safety bump
            }
            if self.current_kind() == TokenKind::CloseParen {
                children.push(SyntaxElement::Token(self.bump())); // )
            }
        }

        // Return type
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::Arrow {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia().into_iter());
            children.push(SyntaxElement::Node(self.parse_type()));
        }

        // Effects
        children.extend(self.eat_trivia().into_iter());
        children.extend(self.parse_effects().into_iter());

        // Block
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Node(self.parse_block()));
        }

        SyntaxNode::new(NodeKind::ForgeDecl, children)
    }

    fn parse_shape_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::Impl {
            children.push(SyntaxElement::Token(self.bump())); // impl
            return self.parse_impl_after_keywords(children);
        }

        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // name
        }

        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::Lt {
            children.push(SyntaxElement::Node(self.parse_generic_params()));
        }

        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Token(self.bump()));
            while self.current_kind() != TokenKind::CloseBrace
                && self.current_kind() != TokenKind::Eof
            {
                let start = self.pos;
                children.push(SyntaxElement::Node(self.parse_field()));
                children.extend(self.eat_trivia().into_iter());
                if self.pos == start {
                    self.bump();
                } // Safety bump
            }
            if self.current_kind() == TokenKind::CloseBrace {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }

        SyntaxNode::new(NodeKind::ShapeDecl, children)
    }

    fn parse_impl_after_keywords(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // Target type
        }

        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Token(self.bump()));
            while self.current_kind() != TokenKind::CloseBrace
                && self.current_kind() != TokenKind::Eof
            {
                let start = self.pos;
                children.push(SyntaxElement::Node(self.parse_decl()));
                children.extend(self.eat_trivia().into_iter());
                if self.pos == start {
                    self.bump();
                } // Safety bump
            }
            if self.current_kind() == TokenKind::CloseBrace {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }

        SyntaxNode::new(NodeKind::ImplBlock, children)
    }

    fn parse_scroll_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // name
        }

        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Token(self.bump()));
            while self.current_kind() != TokenKind::CloseBrace
                && self.current_kind() != TokenKind::Eof
            {
                let start = self.pos;
                children.push(SyntaxElement::Node(self.parse_variant()));
                children.extend(self.eat_trivia().into_iter());
                if self.pos == start {
                    self.bump();
                } // Safety bump
            }
            if self.current_kind() == TokenKind::CloseBrace {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }

        SyntaxNode::new(NodeKind::ScrollDecl, children)
    }

    fn parse_variant(&mut self) -> SyntaxNode {
        let mut children = self.eat_trivia();
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump()));

            children.extend(self.eat_trivia().into_iter());
            // Optional data Circle { radius: f64 } or Point(f64, f64)
            if self.current_kind() == TokenKind::OpenBrace {
                // Reuse field parsing or block? Let's just consume for now
                children.push(SyntaxElement::Token(self.bump()));
                while self.current_kind() != TokenKind::CloseBrace
                    && self.current_kind() != TokenKind::Eof
                {
                    let t = self.bump();
                    children.push(SyntaxElement::Token(t));
                    if t.kind == TokenKind::Eof {
                        break;
                    }
                }
                if self.current_kind() == TokenKind::CloseBrace {
                    children.push(SyntaxElement::Token(self.bump()));
                }
            } else if self.current_kind() == TokenKind::OpenParen {
                children.push(SyntaxElement::Token(self.bump()));
                while self.current_kind() != TokenKind::CloseParen
                    && self.current_kind() != TokenKind::Eof
                {
                    let t = self.bump();
                    children.push(SyntaxElement::Token(t));
                    if t.kind == TokenKind::Eof {
                        break;
                    }
                }
                if self.current_kind() == TokenKind::CloseParen {
                    children.push(SyntaxElement::Token(self.bump()));
                }
            }
        }
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::Comma {
            children.push(SyntaxElement::Token(self.bump()));
        }
        SyntaxNode::new(NodeKind::Variant, children)
    }

    pub fn parse_type(&mut self) -> SyntaxNode {
        let mut children = self.eat_trivia();
        match self.current_kind() {
            TokenKind::Question => {
                children.push(SyntaxElement::Token(self.bump()));
                children.push(SyntaxElement::Node(self.parse_type()));
                SyntaxNode::new(NodeKind::OptionalType, children)
            }
            TokenKind::Star => {
                children.push(SyntaxElement::Token(self.bump()));
                if self.current_kind() == TokenKind::Tilde {
                    children.push(SyntaxElement::Token(self.bump()));
                }
                children.push(SyntaxElement::Node(self.parse_type()));
                SyntaxNode::new(NodeKind::PointerType, children)
            }
            TokenKind::Raw => self.parse_raw_expr(),
            _ => {
                let mut res = self.parse_expr(Precedence::Call);
                self.eat_trivia();
                if self.current_kind() == TokenKind::Bang {
                    res = self.parse_postfix_bang(res);
                }

                // If we have Lt after an Ident, it's likely a generic type
                if res.kind == NodeKind::Ident && self.current_kind() == TokenKind::Lt {
                    let mut children = vec![SyntaxElement::Node(res)];
                    children.push(SyntaxElement::Node(self.parse_generic_args()));
                    res = SyntaxNode::new(NodeKind::Type, children);
                }

                res
            }
        }
    }

    fn parse_ward_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // name
        }

        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Token(self.bump()));
            while self.current_kind() != TokenKind::CloseBrace
                && self.current_kind() != TokenKind::Eof
            {
                let start = self.pos;
                children.push(SyntaxElement::Node(self.parse_decl()));
                children.extend(self.eat_trivia().into_iter());
                if self.pos == start {
                    self.bump();
                } // Safety bump
            }
            if self.current_kind() == TokenKind::CloseBrace {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }

        SyntaxNode::new(NodeKind::WardDecl, children)
    }

    fn parse_dual_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::Shape {
            children.push(SyntaxElement::Token(self.bump())); // shape
        }

        children.extend(self.eat_trivia().into_iter());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // name
        }

        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::Lt {
            children.push(SyntaxElement::Node(self.parse_generic_params()));
        }

        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Token(self.bump()));
            while self.current_kind() != TokenKind::CloseBrace
                && self.current_kind() != TokenKind::Eof
            {
                let start = self.pos;
                children.push(SyntaxElement::Node(self.parse_decl()));
                children.extend(self.eat_trivia().into_iter());
                if self.pos == start {
                    self.bump();
                } // Safety bump
            }
            if self.current_kind() == TokenKind::CloseBrace {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }

        SyntaxNode::new(NodeKind::DualDecl, children)
    }

    fn parse_weave_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // weave name
        }

        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::For {
            children.push(SyntaxElement::Token(self.bump()));
            return self.parse_impl_after_keywords(children); // Treat weave...for as an ImplBlock too
        }

        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Token(self.bump()));
            while self.current_kind() != TokenKind::CloseBrace
                && self.current_kind() != TokenKind::Eof
            {
                let start = self.pos;
                children.push(SyntaxElement::Node(self.parse_decl()));
                children.extend(self.eat_trivia().into_iter());
                if self.pos == start {
                    self.bump();
                } // Safety bump
            }
            if self.current_kind() == TokenKind::CloseBrace {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }

        SyntaxNode::new(NodeKind::WeaveDecl, children)
    }

    fn parse_draw_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        // Simple path parsing: ident (:: ident)* (:: *)?
        loop {
            if self.is_naming_ident() || self.current_kind() == TokenKind::Star {
                children.push(SyntaxElement::Token(self.bump()));
            } else {
                break;
            }
            children.extend(self.eat_trivia().into_iter());
            if self.current_kind() == TokenKind::DoubleColon {
                children.push(SyntaxElement::Token(self.bump()));
                children.extend(self.eat_trivia().into_iter());
            } else {
                break;
            }
        }
        if self.current_kind() == TokenKind::Semicolon {
            children.push(SyntaxElement::Token(self.bump()));
        }
        SyntaxNode::new(NodeKind::DrawDecl, children)
    }

    fn parse_generic_params(&mut self) -> SyntaxNode {
        let mut children = self.eat_trivia();
        let open = self.current_kind();
        if open == TokenKind::Lt || open == TokenKind::OpenBracket {
            children.push(SyntaxElement::Token(self.bump()));
            let close = if open == TokenKind::Lt {
                TokenKind::Gt
            } else {
                TokenKind::CloseBracket
            };

            while self.current_kind() != close && self.current_kind() != TokenKind::Eof {
                let start = self.pos;
                let mut param_children = self.eat_trivia();
                if self.is_naming_ident() {
                    param_children.push(SyntaxElement::Token(self.bump()));
                    param_children.extend(self.eat_trivia().into_iter());
                    if self.current_kind() == TokenKind::Colon {
                        param_children.push(SyntaxElement::Token(self.bump()));
                        param_children.extend(self.eat_trivia().into_iter());
                        if self.is_naming_ident() {
                            param_children.push(SyntaxElement::Token(self.bump()));
                        }
                    }
                }
                children.push(SyntaxElement::Node(SyntaxNode::new(
                    NodeKind::GenericParam,
                    param_children,
                )));
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::Comma {
                    children.push(SyntaxElement::Token(self.bump()));
                }
                children.extend(self.eat_trivia().into_iter());
                if self.pos == start {
                    self.bump();
                } // Safety bump
            }
            if self.current_kind() == close {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }
        SyntaxNode::new(NodeKind::GenericParams, children)
    }

    fn parse_generic_args(&mut self) -> SyntaxNode {
        let mut children = self.eat_trivia();
        let open = self.current_kind();
        if open == TokenKind::Lt || open == TokenKind::OpenBracket {
            children.push(SyntaxElement::Token(self.bump()));
            let close = if open == TokenKind::Lt {
                TokenKind::Gt
            } else {
                TokenKind::CloseBracket
            };

            while self.current_kind() != close && self.current_kind() != TokenKind::Eof {
                let start = self.pos;
                let arg = self.parse_expr(Precedence::Comparison);
                children.push(SyntaxElement::Node(SyntaxNode::new(
                    NodeKind::GenericArg,
                    vec![SyntaxElement::Node(arg)],
                )));
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::Comma {
                    children.push(SyntaxElement::Token(self.bump()));
                }
                children.extend(self.eat_trivia().into_iter());
                if self.pos == start {
                    self.bump();
                } // Safety bump
            }
            if self.current_kind() == close {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }
        SyntaxNode::new(NodeKind::GenericArgs, children)
    }

    fn parse_field(&mut self) -> SyntaxNode {
        let mut children = self.eat_trivia();
        // visibility
        if matches!(self.current_kind(), TokenKind::Open | TokenKind::Hidden) {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia().into_iter());
        }
        // Name
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia().into_iter());
        }
        // Colon
        if self.current_kind() == TokenKind::Colon {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia().into_iter());
        }
        // Type
        children.extend(self.eat_trivia().into_iter());
        children.push(SyntaxElement::Node(self.parse_type()));
        children.extend(self.eat_trivia().into_iter());

        if self.current_kind() == TokenKind::Comma {
            children.push(SyntaxElement::Token(self.bump()));
        }
        SyntaxNode::new(NodeKind::Field, children)
    }

    fn parse_param(&mut self) -> SyntaxNode {
        let mut children = self.eat_trivia();

        // Handle mutability/references prefix: ~ or & or &~
        while self.current_kind() == TokenKind::Tilde || self.current_kind() == TokenKind::Ampersand
        {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia().into_iter());
        }

        if self.is_naming_ident() || self.current_kind() == TokenKind::SelfKw {
            children.push(SyntaxElement::Token(self.bump())); // name or self
            children.extend(self.eat_trivia().into_iter());
            if self.current_kind() == TokenKind::Colon {
                children.push(SyntaxElement::Token(self.bump()));
                children.extend(self.eat_trivia().into_iter());
                children.push(SyntaxElement::Node(self.parse_type()));
            }
        } else {
            // Error recovery: consume one token to avoid infinite loop
            if self.current_kind() != TokenKind::Eof {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }
        SyntaxNode::new(NodeKind::ParamPart, children)
    }

    pub fn parse_stmt(&mut self) -> SyntaxNode {
        let children = self.eat_trivia();
        self.parse_stmt_after_trivia(children)
    }

    fn parse_stmt_after_trivia(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        match self.current_kind() {
            TokenKind::Let | TokenKind::Tilde => {
                children.push(SyntaxElement::Token(self.bump())); // let or ~
                children.extend(self.eat_trivia().into_iter());
                if self.is_naming_ident() {
                    children.push(SyntaxElement::Token(self.bump())); // name
                }
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::Colon {
                    children.push(SyntaxElement::Token(self.bump()));
                    children.extend(self.eat_trivia().into_iter());
                    children.push(SyntaxElement::Node(self.parse_type()));
                }
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::Equal {
                    children.push(SyntaxElement::Token(self.bump()));
                    children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
                }
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::Semicolon {
                    children.push(SyntaxElement::Token(self.bump()));
                }
                SyntaxNode::new(NodeKind::LetStmt, children)
            }
            TokenKind::OpenBrace => SyntaxNode::new(
                NodeKind::Block,
                vec![SyntaxElement::Node(self.parse_block())],
            ),
            _ => {
                children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::Semicolon {
                    children.push(SyntaxElement::Token(self.bump()));
                }
                SyntaxNode::new(NodeKind::ExprStmt, children)
            }
        }
    }

    pub fn parse_block(&mut self) -> SyntaxNode {
        let mut children = self.eat_trivia();
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia().into_iter());
            while self.current_kind() != TokenKind::CloseBrace
                && self.current_kind() != TokenKind::Eof
            {
                let start = self.pos;
                children.push(SyntaxElement::Node(self.parse_stmt()));
                children.extend(self.eat_trivia().into_iter());
                if self.pos == start {
                    self.bump();
                } // Safety bump
            }
            if self.current_kind() == TokenKind::CloseBrace {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }
        SyntaxNode::new(NodeKind::Block, children)
    }

    pub fn parse_given_expr(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Node(self.parse_block()));
        }
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::Else {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia().into_iter());
            if self.current_kind() == TokenKind::Given {
                let next_given_children = vec![SyntaxElement::Token(self.bump())];
                children.push(SyntaxElement::Node(
                    self.parse_given_expr(next_given_children),
                ));
            } else if self.current_kind() == TokenKind::OpenBrace {
                children.push(SyntaxElement::Node(self.parse_block()));
            }
        }
        SyntaxNode::new(NodeKind::GivenExpr, children)
    }

    pub fn parse_branch_expr(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Token(self.bump()));
            while self.current_kind() != TokenKind::CloseBrace
                && self.current_kind() != TokenKind::Eof
            {
                let start = self.pos;
                children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::FatArrow {
                    children.push(SyntaxElement::Token(self.bump()));
                    children.extend(self.eat_trivia().into_iter());
                    children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
                }
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::Comma {
                    children.push(SyntaxElement::Token(self.bump()));
                }
                children.extend(self.eat_trivia().into_iter());
                if self.pos == start {
                    self.bump();
                } // Safety bump
            }
            if self.current_kind() == TokenKind::CloseBrace {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }
        SyntaxNode::new(NodeKind::BranchExpr, children)
    }

    pub fn parse_loop_expr(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Node(self.parse_block()));
        }
        SyntaxNode::new(NodeKind::LoopExpr, children)
    }

    pub fn parse_while_expr(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Node(self.parse_block()));
        }
        SyntaxNode::new(NodeKind::WhileExpr, children)
    }

    pub fn parse_each_expr(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::In {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia().into_iter());
            children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
        }
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Node(self.parse_block()));
        }
        SyntaxNode::new(NodeKind::EachExpr, children)
    }

    pub fn parse_bind_expr(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia().into_iter());
        if self.current_kind() == TokenKind::Bar {
            children.push(SyntaxElement::Token(self.bump())); // |
            while self.current_kind() != TokenKind::Bar && self.current_kind() != TokenKind::Eof {
                children.push(SyntaxElement::Token(self.bump())); // Simple param consumption
            }
            if self.current_kind() == TokenKind::Bar {
                children.push(SyntaxElement::Token(self.bump())); // |
            }
        }
        children.extend(self.eat_trivia().into_iter());
        children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
        SyntaxNode::new(NodeKind::BindExpr, children)
    }

    fn eat_trivia(&mut self) -> Vec<SyntaxElement> {
        let mut trivia = Vec::new();
        while self.current_kind() == TokenKind::Whitespace
            || self.current_kind() == TokenKind::Comment
        {
            trivia.push(SyntaxElement::Token(self.bump()));
        }
        trivia
    }

    fn current_kind(&self) -> TokenKind {
        self.tokens
            .get(self.pos)
            .map(|t| t.kind)
            .unwrap_or(TokenKind::Eof)
    }

    fn bump(&mut self) -> Token {
        let token = self.tokens.get(self.pos).cloned().unwrap_or_else(|| {
            Token::new(
                TokenKind::Eof,
                izel_span::Span::new(
                    izel_span::BytePos(0),
                    izel_span::BytePos(0),
                    izel_span::SourceId(0),
                ),
            )
        });
        if token.kind != TokenKind::Eof {
            self.pos += 1;
        }
        token
    }

    fn is_naming_ident(&self) -> bool {
        match self.current_kind() {
            TokenKind::Ident
            | TokenKind::Next
            | TokenKind::Loop
            | TokenKind::Each
            | TokenKind::While
            | TokenKind::Break
            | TokenKind::Give
            | TokenKind::Type
            | TokenKind::Forge
            | TokenKind::Sole
            | TokenKind::Pure
            | TokenKind::Open
            | TokenKind::Hidden
            | TokenKind::Draw
            | TokenKind::Seek
            | TokenKind::Catch
            | TokenKind::Flow
            | TokenKind::Tide
            | TokenKind::Zone
            | TokenKind::Bridge
            | TokenKind::Raw
            | TokenKind::Echo
            | TokenKind::Ward
            | TokenKind::Scroll
            | TokenKind::Dual
            | TokenKind::Alias
            | TokenKind::Pkg
            | TokenKind::Comptime
            | TokenKind::Static
            | TokenKind::Extern
            | TokenKind::Bind => true,
            _ => false,
        }
    }

    fn parse_effects(&mut self) -> Vec<SyntaxElement> {
        let mut results = vec![];
        while self.current_kind() == TokenKind::Bang {
            let start = self.pos;
            let mut inner = vec![SyntaxElement::Token(self.bump())];
            if self.is_naming_ident() || self.current_kind() == TokenKind::Pure {
                inner.push(SyntaxElement::Token(self.bump()));
            }
            results.push(SyntaxElement::Node(SyntaxNode::new(
                NodeKind::Effect,
                inner,
            )));
            results.extend(self.eat_trivia().into_iter());
            if self.pos == start {
                self.bump();
            } // Safety bump
        }
        results
    }

    fn parse_raw_expr(&mut self) -> SyntaxNode {
        let mut children = vec![];
        children.push(SyntaxElement::Token(self.bump())); // raw
        children.extend(self.eat_trivia().into_iter());
        children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
        SyntaxNode::new(NodeKind::RawExpr, children)
    }

    fn parse_attributes(&mut self) -> SyntaxNode {
        let mut children = vec![];
        while self.current_kind() == TokenKind::At || self.current_kind() == TokenKind::Pound {
            let start = self.pos;
            children.push(SyntaxElement::Node(self.parse_attribute()));
            children.extend(self.eat_trivia().into_iter());
            if self.pos == start {
                self.bump();
            } // Safety bump
        }
        SyntaxNode::new(NodeKind::Attributes, children)
    }

    fn parse_attribute(&mut self) -> SyntaxNode {
        let mut children = self.eat_trivia();
        match self.current_kind() {
            TokenKind::At => {
                children.push(SyntaxElement::Token(self.bump())); // @
                children.extend(self.eat_trivia().into_iter());
                if self.is_naming_ident() {
                    children.push(SyntaxElement::Token(self.bump())); // name
                }
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::OpenParen {
                    children.push(SyntaxElement::Token(self.bump())); // (
                    while self.current_kind() != TokenKind::CloseParen
                        && self.current_kind() != TokenKind::Eof
                    {
                        children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
                        children.extend(self.eat_trivia().into_iter());
                        if self.current_kind() == TokenKind::Comma {
                            children.push(SyntaxElement::Token(self.bump()));
                            children.extend(self.eat_trivia().into_iter());
                        }
                    }
                    if self.current_kind() == TokenKind::CloseParen {
                        children.push(SyntaxElement::Token(self.bump())); // )
                    }
                }
            }
            TokenKind::Pound => {
                children.push(SyntaxElement::Token(self.bump())); // #
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::OpenBracket {
                    children.push(SyntaxElement::Token(self.bump())); // [
                    children.extend(self.eat_trivia().into_iter());
                    if self.is_naming_ident() {
                        children.push(SyntaxElement::Token(self.bump())); // name
                    }
                    children.extend(self.eat_trivia().into_iter());
                    if self.current_kind() == TokenKind::OpenParen {
                        children.push(SyntaxElement::Token(self.bump())); // (
                        while self.current_kind() != TokenKind::CloseParen
                            && self.current_kind() != TokenKind::Eof
                        {
                            children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
                            children.extend(self.eat_trivia().into_iter());
                            if self.current_kind() == TokenKind::Comma {
                                children.push(SyntaxElement::Token(self.bump()));
                                children.extend(self.eat_trivia().into_iter());
                            }
                        }
                        if self.current_kind() == TokenKind::CloseParen {
                            children.push(SyntaxElement::Token(self.bump())); // )
                        }
                        children.extend(self.eat_trivia().into_iter());
                    }
                    if self.current_kind() == TokenKind::CloseBracket {
                        children.push(SyntaxElement::Token(self.bump())); // ]
                    }
                }
            }
            _ => {}
        }
        SyntaxNode::new(NodeKind::Attribute, children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use izel_lexer::Lexer;

    fn parse_test(src: &str) -> SyntaxNode {
        let mut lexer = Lexer::new(src, izel_span::SourceId(0));
        let mut tokens = Vec::new();
        loop {
            let t = lexer.next_token();
            if t.kind == TokenKind::Eof {
                tokens.push(t);
                break;
            }
            tokens.push(t);
        }
        let mut parser = Parser::new(tokens);
        parser.source = src.to_string();
        parser.parse_decl()
    }

    #[test]
    fn test_parse_dual_decl() {
        let node = parse_test("dual shape JsonFormat<T> { forge encode(&self, val: &T) }");
        assert_eq!(node.kind, NodeKind::DualDecl);
        assert!(node.children.len() > 6); // dual, shape, name, generics, {, forge...
    }

    #[test]
    fn test_parse_bracket_attribute() {
        let node = parse_test("#[intrinsic(\"i32_abs\")] forge abs(x: i32) -> i32");
        assert_eq!(node.kind, NodeKind::ForgeDecl);
        
        let has_attr = node.children.iter().any(|child| {
            if let SyntaxElement::Node(n) = child {
                n.kind == NodeKind::Attributes && n.children.iter().any(|attr_child| {
                    if let SyntaxElement::Node(an) = attr_child {
                        an.kind == NodeKind::Attribute
                    } else { false }
                })
            } else { false }
        });
        assert!(has_attr, "Should have Attribute node");
    }
}
