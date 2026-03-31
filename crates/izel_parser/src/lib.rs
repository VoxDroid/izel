#![allow(clippy::match_like_matches_macro, clippy::useless_conversion)]
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
    pub fn new(tokens: Vec<Token>, source: String) -> Self {
        Self {
            tokens,
            pos: 0,
            source,
        }
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
            children.extend(self.eat_trivia());
        }

        // Handle modifiers (open/hidden/pure/sole/etc)
        while matches!(
            self.current_kind(),
            TokenKind::Open
                | TokenKind::Hidden
                | TokenKind::Pkg
                | TokenKind::Pure
                | TokenKind::Sole
                | TokenKind::Flow
                | TokenKind::Comptime
                | TokenKind::Extern
        ) {
            let kind = self.current_kind();
            children.push(SyntaxElement::Token(self.bump()));
            if kind == TokenKind::Pkg && self.current_kind() == TokenKind::OpenParen {
                children.push(SyntaxElement::Token(self.bump())); // (
                while self.current_kind() != TokenKind::CloseParen
                    && self.current_kind() != TokenKind::Eof
                {
                    if matches!(
                        self.current_kind(),
                        TokenKind::Ident | TokenKind::DoubleColon
                    ) {
                        children.push(SyntaxElement::Token(self.bump()));
                    } else {
                        break;
                    }
                }
                if self.current_kind() == TokenKind::CloseParen {
                    children.push(SyntaxElement::Token(self.bump())); // )
                }
            }
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
            TokenKind::Macro => {
                children.push(SyntaxElement::Token(self.bump())); // macro
                self.parse_macro_after_keyword(children)
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
            TokenKind::Static => {
                children.push(SyntaxElement::Token(self.bump())); // static
                self.parse_static_after_keyword(children)
            }
            TokenKind::Echo => {
                children.push(SyntaxElement::Token(self.bump())); // echo
                self.parse_echo_after_keyword(children)
            }
            TokenKind::Bridge => {
                children.push(SyntaxElement::Token(self.bump())); // bridge
                self.parse_bridge_after_keyword(children)
            }
            _ => self.parse_stmt_after_trivia(children),
        }
    }

    fn parse_type_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // name
        }
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Equal {
            children.push(SyntaxElement::Token(self.bump())); // =
            children.extend(self.eat_trivia());
            children.push(SyntaxElement::Node(self.parse_type()));
        }
        SyntaxNode::new(NodeKind::TypeAlias, children)
    }

    fn parse_macro_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // macro name
        }

        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Bang {
            children.push(SyntaxElement::Token(self.bump())); // !
        }

        children.extend(self.eat_trivia());
        if matches!(
            self.current_kind(),
            TokenKind::OpenParen | TokenKind::OpenBracket
        ) {
            let open = self.current_kind();
            let close = if open == TokenKind::OpenParen {
                TokenKind::CloseParen
            } else {
                TokenKind::CloseBracket
            };

            children.push(SyntaxElement::Token(self.bump())); // ( or [
            children.extend(self.eat_trivia());

            while self.current_kind() != close && self.current_kind() != TokenKind::Eof {
                let start = self.pos;
                if self.current_kind() == TokenKind::DotDot {
                    children.push(SyntaxElement::Token(self.bump())); // ..
                    children.extend(self.eat_trivia());
                }

                if self.is_naming_ident() {
                    children.push(SyntaxElement::Token(self.bump())); // param name
                } else {
                    children.push(SyntaxElement::Token(self.bump()));
                }

                children.extend(self.eat_trivia());
                if self.current_kind() == TokenKind::Comma {
                    children.push(SyntaxElement::Token(self.bump()));
                    children.extend(self.eat_trivia());
                }

                if self.pos == start {
                    self.bump();
                }
            }

            if self.current_kind() == close {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }

        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Node(self.parse_block()));
        }

        SyntaxNode::new(NodeKind::MacroDecl, children)
    }

    fn parse_echo_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Node(self.parse_block()));
        }
        SyntaxNode::new(NodeKind::EchoDecl, children)
    }

    fn parse_bridge_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia());
        if matches!(self.current_kind(), TokenKind::Str { .. }) {
            children.push(SyntaxElement::Token(self.bump())); // abi string
        }
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Token(self.bump())); // {
            while self.current_kind() != TokenKind::CloseBrace
                && self.current_kind() != TokenKind::Eof
            {
                children.push(SyntaxElement::Node(self.parse_decl()));
                children.extend(self.eat_trivia());
            }
            if self.current_kind() == TokenKind::CloseBrace {
                children.push(SyntaxElement::Token(self.bump())); // }
            }
        }
        SyntaxNode::new(NodeKind::BridgeDecl, children)
    }

    fn parse_forge_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // name
        }

        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Lt {
            children.push(SyntaxElement::Node(self.parse_generic_params()));
        }

        // Params
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::OpenParen {
            children.push(SyntaxElement::Token(self.bump())); // (
            children.extend(self.eat_trivia());
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
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Arrow {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia());
            children.push(SyntaxElement::Node(self.parse_type()));
        }

        // Effects
        children.extend(self.eat_trivia());
        children.extend(self.parse_effects());

        // Block
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Node(self.parse_block()));
        }

        SyntaxNode::new(NodeKind::ForgeDecl, children)
    }

    fn parse_shape_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Impl {
            children.push(SyntaxElement::Token(self.bump())); // impl
            return self.parse_impl_after_keywords(children);
        }

        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // name
        }

        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Lt {
            children.push(SyntaxElement::Node(self.parse_generic_params()));
        }

        children.extend(self.eat_trivia());
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
        children.extend(self.eat_trivia());
        children.push(SyntaxElement::Node(self.parse_type()));
        children.extend(self.eat_trivia());

        children.extend(self.eat_trivia());
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

    pub fn parse_pattern(&mut self) -> SyntaxNode {
        let mut children = self.eat_trivia();
        match self.current_kind() {
            TokenKind::Tilde => {
                children.push(SyntaxElement::Token(self.bump())); // ~
                children.extend(self.eat_trivia());
                if self.is_naming_ident() {
                    children.push(SyntaxElement::Token(self.bump())); // name
                }
            }
            TokenKind::OpenParen => {
                children.push(SyntaxElement::Token(self.bump())); // (
                children.extend(self.eat_trivia());
                while self.current_kind() != TokenKind::CloseParen
                    && self.current_kind() != TokenKind::Eof
                {
                    children.push(SyntaxElement::Node(self.parse_pattern()));
                    children.extend(self.eat_trivia());
                    if self.current_kind() == TokenKind::Comma {
                        children.push(SyntaxElement::Token(self.bump()));
                        children.extend(self.eat_trivia());
                    }
                }
                if self.current_kind() == TokenKind::CloseParen {
                    children.push(SyntaxElement::Token(self.bump()));
                }
            }
            TokenKind::OpenBracket => {
                children.push(SyntaxElement::Token(self.bump())); // [
                children.extend(self.eat_trivia());
                while self.current_kind() != TokenKind::CloseBracket
                    && self.current_kind() != TokenKind::Eof
                {
                    // Check for rest spread
                    if self.current_kind() == TokenKind::DotDot {
                        children.push(SyntaxElement::Token(self.bump())); // ...
                        children.extend(self.eat_trivia());
                        if self.is_naming_ident() {
                            children.push(SyntaxElement::Token(self.bump())); // rest name
                        }
                    } else {
                        children.push(SyntaxElement::Node(self.parse_pattern()));
                    }
                    children.extend(self.eat_trivia());
                    if self.current_kind() == TokenKind::Comma {
                        children.push(SyntaxElement::Token(self.bump()));
                        children.extend(self.eat_trivia());
                    }
                }
                if self.current_kind() == TokenKind::CloseBracket {
                    children.push(SyntaxElement::Token(self.bump()));
                }
            }
            TokenKind::Int { .. }
            | TokenKind::Str { .. }
            | TokenKind::InterpolatedStr { .. }
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Nil => {
                children.push(SyntaxElement::Token(self.bump())); // Literal pattern
            }
            _ => {
                // Must be struct or variant or simple ident
                if self.is_naming_ident() {
                    children.push(SyntaxElement::Token(self.bump()));
                    children.extend(self.eat_trivia());
                    // Path or Struct
                    if self.current_kind() == TokenKind::DoubleColon
                        || self.current_kind() == TokenKind::OpenBrace
                    {
                        // Very simplified struct pattern parser
                        while self.current_kind() == TokenKind::DoubleColon {
                            children.push(SyntaxElement::Token(self.bump()));
                            children.extend(self.eat_trivia());
                            if self.is_naming_ident() {
                                children.push(SyntaxElement::Token(self.bump()));
                            }
                            children.extend(self.eat_trivia());
                        }
                        if self.current_kind() == TokenKind::OpenBrace {
                            children.push(SyntaxElement::Token(self.bump()));
                            children.extend(self.eat_trivia());
                            while self.current_kind() != TokenKind::CloseBrace
                                && self.current_kind() != TokenKind::Eof
                            {
                                // field logic
                                children.push(SyntaxElement::Node(self.parse_pattern()));
                                children.extend(self.eat_trivia());
                                if self.current_kind() == TokenKind::Comma {
                                    children.push(SyntaxElement::Token(self.bump()));
                                    children.extend(self.eat_trivia());
                                }
                            }
                            if self.current_kind() == TokenKind::CloseBrace {
                                children.push(SyntaxElement::Token(self.bump()));
                            }
                        }
                    } else if self.current_kind() == TokenKind::OpenParen {
                        // Variant with data `Some(x)`
                        children.push(SyntaxElement::Token(self.bump()));
                        children.extend(self.eat_trivia());
                        while self.current_kind() != TokenKind::CloseParen
                            && self.current_kind() != TokenKind::Eof
                        {
                            children.push(SyntaxElement::Node(self.parse_pattern()));
                            children.extend(self.eat_trivia());
                            if self.current_kind() == TokenKind::Comma {
                                children.push(SyntaxElement::Token(self.bump()));
                                children.extend(self.eat_trivia());
                            }
                        }
                        if self.current_kind() == TokenKind::CloseParen {
                            children.push(SyntaxElement::Token(self.bump()));
                        }
                    }
                }
            }
        }

        children.extend(self.eat_trivia());
        // Handle Or patterns e.g. Pat | Pat
        if self.current_kind() == TokenKind::Pipe {
            children.push(SyntaxElement::Token(self.bump()));
            children.push(SyntaxElement::Node(self.parse_pattern()));
        }

        SyntaxNode::new(NodeKind::Pattern, children)
    }

    fn parse_scroll_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // name
        }

        children.extend(self.eat_trivia());
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

        // Visibility
        while matches!(
            self.current_kind(),
            TokenKind::Open | TokenKind::Hidden | TokenKind::Pkg
        ) {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia());
        }

        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump()));

            children.extend(self.eat_trivia());
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
        children.extend(self.eat_trivia());
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
        children.extend(self.eat_trivia());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // name
        }

        children.extend(self.eat_trivia());
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
        children.extend(self.eat_trivia());
        let mut is_shape = false;
        if self.current_kind() == TokenKind::Shape {
            children.push(SyntaxElement::Token(self.bump())); // shape
            is_shape = true;
        }

        children.extend(self.eat_trivia());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // name
        }

        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Lt {
            children.push(SyntaxElement::Node(self.parse_generic_params()));
        }

        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Token(self.bump()));
            while self.current_kind() != TokenKind::CloseBrace
                && self.current_kind() != TokenKind::Eof
            {
                let start = self.pos;
                if is_shape {
                    // Peek if it's a declaration (starting with forge, shape, etc)
                    if matches!(
                        self.current_kind(),
                        TokenKind::Forge
                            | TokenKind::Shape
                            | TokenKind::Scroll
                            | TokenKind::Dual
                            | TokenKind::Weave
                            | TokenKind::Ward
                            | TokenKind::Impl
                            | TokenKind::Type
                            | TokenKind::Draw
                    ) {
                        children.push(SyntaxElement::Node(self.parse_decl()));
                    } else {
                        children.push(SyntaxElement::Node(self.parse_field()));
                    }
                } else {
                    children.push(SyntaxElement::Node(self.parse_decl()));
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

        SyntaxNode::new(NodeKind::DualDecl, children)
    }

    fn parse_weave_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia());
        // Could be a name (definition) or a path (implementation)
        // If it's followed by 'for', it's an implementation.
        // But parse_type handles both.
        children.push(SyntaxElement::Node(self.parse_type()));
        children.extend(self.eat_trivia());

        // Support inheritance: weave Drawable : Parent1 + Parent2
        if self.current_kind() == TokenKind::Colon {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia());
            while self.current_kind() != TokenKind::OpenBrace
                && self.current_kind() != TokenKind::Eof
            {
                children.push(SyntaxElement::Node(self.parse_type()));
                children.extend(self.eat_trivia());
                if self.current_kind() == TokenKind::Plus {
                    children.push(SyntaxElement::Token(self.bump()));
                    children.extend(self.eat_trivia());
                } else {
                    break;
                }
            }
        }

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
        children.extend(self.eat_trivia());
        // Simple path parsing: ident (:: ident)* (:: *)?
        loop {
            if self.is_naming_ident() || self.current_kind() == TokenKind::Star {
                children.push(SyntaxElement::Token(self.bump()));
            } else {
                break;
            }
            children.extend(self.eat_trivia());
            if self.current_kind() == TokenKind::DoubleColon
                || self.current_kind() == TokenKind::Slash
            {
                children.push(SyntaxElement::Token(self.bump()));
                children.extend(self.eat_trivia());
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

        // Visibility
        while matches!(
            self.current_kind(),
            TokenKind::Open | TokenKind::Hidden | TokenKind::Pkg
        ) {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia());
        }

        // Name
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia());
        }
        // Colon
        if self.current_kind() == TokenKind::Colon {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia());
        }
        // Type
        children.extend(self.eat_trivia());
        children.push(SyntaxElement::Node(self.parse_type()));
        children.extend(self.eat_trivia());

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

        // Variadic .. prefix
        if self.current_kind() == TokenKind::DotDot {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia().into_iter());
        }

        if self.is_naming_ident() || self.current_kind() == TokenKind::SelfKw {
            children.push(SyntaxElement::Token(self.bump())); // name or self
            children.extend(self.eat_trivia());
            if self.current_kind() == TokenKind::Colon {
                children.push(SyntaxElement::Token(self.bump()));
                children.extend(self.eat_trivia());

                // Old variadic position (removed)

                children.push(SyntaxElement::Node(self.parse_type()));
                children.extend(self.eat_trivia());
            }
            if self.current_kind() == TokenKind::Equal {
                children.push(SyntaxElement::Token(self.bump()));
                children.extend(self.eat_trivia());
                children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
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
                children.extend(self.eat_trivia());
                children.push(SyntaxElement::Node(self.parse_pattern()));
                children.extend(self.eat_trivia());
                if self.current_kind() == TokenKind::Colon {
                    children.push(SyntaxElement::Token(self.bump()));
                    children.extend(self.eat_trivia());
                    children.push(SyntaxElement::Node(self.parse_type()));
                }
                children.extend(self.eat_trivia());
                if self.current_kind() == TokenKind::Equal {
                    children.push(SyntaxElement::Token(self.bump()));
                    children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
                }
                children.extend(self.eat_trivia());
                if self.current_kind() == TokenKind::Semicolon {
                    children.push(SyntaxElement::Token(self.bump()));
                }
                SyntaxNode::new(NodeKind::LetStmt, children)
            }
            TokenKind::Give => {
                children.push(SyntaxElement::Token(self.bump())); // give
                children.extend(self.eat_trivia());
                children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
                children.extend(self.eat_trivia());
                if self.current_kind() == TokenKind::Semicolon {
                    children.push(SyntaxElement::Token(self.bump()));
                }
                SyntaxNode::new(NodeKind::GiveStmt, children)
            }
            TokenKind::OpenBrace => SyntaxNode::new(
                NodeKind::Block,
                vec![SyntaxElement::Node(self.parse_block())],
            ),
            _ => {
                children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
                children.extend(self.eat_trivia());
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
            children.extend(self.eat_trivia());
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
        children.extend(self.eat_trivia());
        children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Node(self.parse_block()));
        }
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Else {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia());
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
        children.extend(self.eat_trivia());
        children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Token(self.bump()));
            while self.current_kind() != TokenKind::CloseBrace
                && self.current_kind() != TokenKind::Eof
            {
                let start = self.pos;
                children.push(SyntaxElement::Node(self.parse_pattern()));
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::Given {
                    // Guard
                    children.push(SyntaxElement::Token(self.bump()));
                    children.extend(self.eat_trivia().into_iter());
                    children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
                    children.extend(self.eat_trivia().into_iter());
                }
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
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Node(self.parse_block()));
        }
        SyntaxNode::new(NodeKind::LoopExpr, children)
    }

    pub fn parse_while_expr(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia());
        children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Node(self.parse_block()));
        }
        SyntaxNode::new(NodeKind::WhileExpr, children)
    }

    pub fn parse_each_expr(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia());
        children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::In {
            children.push(SyntaxElement::Token(self.bump()));
            children.extend(self.eat_trivia());
            children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
        }
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Node(self.parse_block()));
        }
        SyntaxNode::new(NodeKind::EachExpr, children)
    }

    fn parse_static_after_keyword(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Tilde {
            children.push(SyntaxElement::Token(self.bump())); // ~
        }
        children.extend(self.eat_trivia());
        if self.is_naming_ident() {
            children.push(SyntaxElement::Token(self.bump())); // name
        }
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Colon {
            children.push(SyntaxElement::Token(self.bump())); // :
            children.extend(self.eat_trivia());
            children.push(SyntaxElement::Node(self.parse_type()));
        }
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Equal {
            children.push(SyntaxElement::Token(self.bump())); // =
            children.extend(self.eat_trivia());
            children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
        }
        SyntaxNode::new(NodeKind::StaticDecl, children)
    }

    pub fn parse_bind_expr(&mut self, mut children: Vec<SyntaxElement>) -> SyntaxNode {
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::Bar {
            children.push(SyntaxElement::Token(self.bump())); // |
            while self.current_kind() != TokenKind::Bar && self.current_kind() != TokenKind::Eof {
                children.push(SyntaxElement::Token(self.bump())); // Simple param consumption
            }
            if self.current_kind() == TokenKind::Bar {
                children.push(SyntaxElement::Token(self.bump())); // |
            }
        }
        children.extend(self.eat_trivia());
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
        children.extend(self.eat_trivia());
        if self.current_kind() == TokenKind::OpenBrace {
            children.push(SyntaxElement::Node(self.parse_block()));
        } else {
            children.push(SyntaxElement::Node(self.parse_expr(Precedence::None)));
        }
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
                children.extend(self.eat_trivia());
                if self.is_naming_ident() {
                    children.push(SyntaxElement::Token(self.bump())); // name
                }
                children.extend(self.eat_trivia());
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
                children.extend(self.eat_trivia());
                if self.current_kind() == TokenKind::OpenBracket {
                    children.push(SyntaxElement::Token(self.bump())); // [
                    children.extend(self.eat_trivia());
                    if self.is_naming_ident() {
                        children.push(SyntaxElement::Token(self.bump())); // name
                    }
                    children.extend(self.eat_trivia());
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
                        children.extend(self.eat_trivia());
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
        let mut parser = Parser::new(tokens, src.to_string());
        parser.parse_decl()
    }

    fn parse_source_test(src: &str) -> SyntaxNode {
        let mut lexer = Lexer::new(src, izel_span::SourceId(0));
        let mut tokens = Vec::new();
        loop {
            let t = lexer.next_token();
            let kind = t.kind;
            tokens.push(t);
            if kind == TokenKind::Eof {
                break;
            }
        }
        let mut parser = Parser::new(tokens, src.to_string());
        parser.parse_source_file()
    }

    fn parse_pattern_test(src: &str) -> SyntaxNode {
        let mut lexer = Lexer::new(src, izel_span::SourceId(0));
        let mut tokens = Vec::new();
        loop {
            let t = lexer.next_token();
            let kind = t.kind;
            tokens.push(t);
            if kind == TokenKind::Eof {
                break;
            }
        }
        let mut parser = Parser::new(tokens, src.to_string());
        parser.parse_pattern()
    }

    fn parse_type_test(src: &str) -> SyntaxNode {
        let mut lexer = Lexer::new(src, izel_span::SourceId(0));
        let mut tokens = Vec::new();
        loop {
            let t = lexer.next_token();
            let kind = t.kind;
            tokens.push(t);
            if kind == TokenKind::Eof {
                break;
            }
        }
        let mut parser = Parser::new(tokens, src.to_string());
        parser.parse_type()
    }

    fn contains_kind(node: &SyntaxNode, needle: NodeKind) -> bool {
        if node.kind == needle {
            return true;
        }

        node.children.iter().any(|child| match child {
            SyntaxElement::Node(n) => contains_kind(n, needle),
            SyntaxElement::Token(_) => false,
        })
    }

    fn count_kind(node: &SyntaxNode, needle: NodeKind) -> usize {
        let mut total = usize::from(node.kind == needle);
        for child in &node.children {
            if let SyntaxElement::Node(n) = child {
                total += count_kind(n, needle);
            }
        }
        total
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
                n.kind == NodeKind::Attributes
                    && n.children.iter().any(|attr_child| {
                        if let SyntaxElement::Node(an) = attr_child {
                            an.kind == NodeKind::Attribute
                        } else {
                            false
                        }
                    })
            } else {
                false
            }
        });
        assert!(has_attr, "Should have Attribute node");
    }

    #[test]
    fn test_parse_macro_decl() {
        let node = parse_test("macro add_one(x) { x + 1 }");
        assert_eq!(node.kind, NodeKind::MacroDecl);
        assert!(
            node.children
                .iter()
                .any(|c| matches!(c, SyntaxElement::Node(n) if n.kind == NodeKind::Block)),
            "macro declaration should contain a body block"
        );
    }

    #[test]
    fn test_parse_decl_with_pkg_modifier_generic_effects_and_block() {
        let node =
            parse_test("pkg(core::io) forge run<T: Trait>(~self, ..args: i32 = 1) { give 0 }");
        assert_eq!(node.kind, NodeKind::ForgeDecl);
        assert!(contains_kind(&node, NodeKind::GenericParams));
        assert!(contains_kind(&node, NodeKind::ParamPart));
        assert!(contains_kind(&node, NodeKind::Block));
    }

    #[test]
    fn test_parse_pattern_forms_cover_core_branches() {
        for src in [
            "~name",
            "(left, right)",
            "[head, ..tail]",
            "Point::Wrap { x, y }",
            "Some(value)",
            "1 | 2",
            "nil",
        ] {
            let node = parse_pattern_test(src);
            assert_eq!(node.kind, NodeKind::Pattern);
        }
    }

    #[test]
    fn test_parse_type_forms_cover_pointer_optional_raw_and_generic_args() {
        assert_eq!(parse_type_test("*~i32").kind, NodeKind::PointerType);
        assert_eq!(parse_type_test("?i32").kind, NodeKind::OptionalType);
        assert_eq!(parse_type_test("raw { give 1 }").kind, NodeKind::RawExpr);

        let generic = parse_type_test("Result<i32, str>");
        assert!(contains_kind(&generic, NodeKind::GenericArgs));
    }

    #[test]
    fn test_parse_source_covers_shape_impl_weave_dual_ward_draw_and_bind() {
        let root = parse_source_test(
            r#"
shape impl Widget { forge draw(self) { give } }
dual Codec { forge encode(self) { give } }
weave Renderable: Debug + Display { forge render(self) { give } }
weave Renderable for Widget { forge render(self) { give } }
ward Core { forge helper() { let f = bind |x, y| x + y; give 0 } }
draw std/io::*;
"#,
        );

        assert_eq!(root.kind, NodeKind::SourceFile);
        assert!(contains_kind(&root, NodeKind::ImplBlock));
        assert!(contains_kind(&root, NodeKind::DualDecl));
        assert!(contains_kind(&root, NodeKind::WeaveDecl));
        assert!(contains_kind(&root, NodeKind::WardDecl));
        assert!(contains_kind(&root, NodeKind::DrawDecl));
        assert!(contains_kind(&root, NodeKind::BindExpr));
    }

    #[test]
    fn test_parse_decl_accepts_stacked_at_and_bracket_attributes() {
        let node = parse_test("@intrinsic(\"x\", 1) #[bench] forge f() { give }");
        assert_eq!(node.kind, NodeKind::ForgeDecl);
        assert!(contains_kind(&node, NodeKind::Attributes));
        assert!(count_kind(&node, NodeKind::Attribute) >= 2);
    }

    #[test]
    fn test_bump_returns_synthetic_eof_when_token_stream_is_empty() {
        let mut parser = Parser::new(vec![], String::new());
        let token = parser.bump();

        assert_eq!(token.kind, TokenKind::Eof);
        assert_eq!(parser.pos, 0);
    }
}
