pub mod cst;
pub mod expr;

use izel_lexer::{Token, TokenKind};
use crate::cst::{NodeKind, SyntaxElement, SyntaxNode};
use crate::expr::Precedence;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parses the entire token stream into a SourceFile CST node.
    pub fn parse_source_file(&mut self) -> SyntaxNode {
        let mut children = Vec::new();
        
        while self.current_kind() != TokenKind::Eof {
            children.push(SyntaxElement::Node(self.parse_decl()));
        }
        
        // Add EOF and any remaining items
        children.push(SyntaxElement::Token(self.bump()));
        
        SyntaxNode::new(NodeKind::SourceFile, children)
    }


    pub fn parse_decl(&mut self) -> SyntaxNode {
        let mut children = self.eat_trivia();
        match self.current_kind() {
            TokenKind::Forge => {
                children.push(SyntaxElement::Token(self.bump())); // forge
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::Ident {
                    children.push(SyntaxElement::Token(self.bump())); // name
                }
                
                // Params
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::OpenParen {
                    children.push(SyntaxElement::Token(self.bump()));
                    // Basic param parsing
                    children.extend(self.eat_trivia().into_iter());
                    while self.current_kind() != TokenKind::CloseParen && self.current_kind() != TokenKind::Eof {
                         children.push(SyntaxElement::Token(self.bump()));
                    }
                    if self.current_kind() == TokenKind::CloseParen {
                        children.push(SyntaxElement::Token(self.bump()));
                    }
                }

                // Return type
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::Arrow {
                    children.push(SyntaxElement::Token(self.bump()));
                    children.extend(self.eat_trivia().into_iter());
                    while self.current_kind() != TokenKind::OpenBrace && self.current_kind() != TokenKind::Eof {
                         children.push(SyntaxElement::Token(self.bump()));
                    }
                }

                // Block
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::OpenBrace {
                    children.push(SyntaxElement::Node(self.parse_block()));
                }

                SyntaxNode::new(NodeKind::ForgeDecl, children)
            }
            _ => self.parse_stmt(),
        }
    }

    pub fn parse_stmt(&mut self) -> SyntaxNode {
        let mut children = self.eat_trivia();
        match self.current_kind() {
            TokenKind::Let | TokenKind::Tilde => {
                children.push(SyntaxElement::Token(self.bump())); // let or ~
                children.extend(self.eat_trivia().into_iter());
                if self.current_kind() == TokenKind::Ident {
                    children.push(SyntaxElement::Token(self.bump())); // name
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
            TokenKind::OpenBrace => SyntaxNode::new(NodeKind::Block, vec![SyntaxElement::Node(self.parse_block())]),
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
            while self.current_kind() != TokenKind::CloseBrace && self.current_kind() != TokenKind::Eof {
                children.push(SyntaxElement::Node(self.parse_stmt()));
                children.extend(self.eat_trivia().into_iter());
            }
            if self.current_kind() == TokenKind::CloseBrace {
                children.push(SyntaxElement::Token(self.bump()));
            }
        }
        SyntaxNode::new(NodeKind::Block, children)
    }

    fn eat_trivia(&mut self) -> Vec<SyntaxElement> {
        let mut trivia = Vec::new();
        while self.current_kind() == TokenKind::Whitespace || self.current_kind() == TokenKind::Comment {
            trivia.push(SyntaxElement::Token(self.bump()));
        }
        trivia
    }

    fn current_kind(&self) -> TokenKind {
        self.tokens.get(self.pos).map(|t| t.kind).unwrap_or(TokenKind::Eof)
    }

    fn bump(&mut self) -> Token {
        let token = self.tokens.get(self.pos).cloned().unwrap_or_else(|| {
              Token::new(TokenKind::Eof, izel_span::Span::new(izel_span::BytePos(0), izel_span::BytePos(0), izel_span::SourceId(0)))
        });
        if token.kind != TokenKind::Eof {
            self.pos += 1;
        }
        token
    }
}
