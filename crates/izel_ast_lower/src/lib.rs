use izel_parser::cst::{SyntaxNode, SyntaxElement, NodeKind};
use izel_parser::ast;
use izel_lexer::TokenKind;

pub struct Lowerer<'a> {
    source: &'a str,
}

impl<'a> Lowerer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source }
    }

    pub fn lower_expr(&self, node: &SyntaxNode) -> ast::Expr {
        match node.kind {
            NodeKind::Literal => {
                if let Some(SyntaxElement::Token(token)) = node.children.first() {
                    match &token.kind {
                        TokenKind::Int { .. } => {
                             let text = &self.source[token.span.lo.0 as usize..token.span.hi.0 as usize];
                             let val = text.replace("_", "").parse::<i128>().unwrap_or(0);
                             return ast::Expr::Literal(ast::Literal::Int(val));
                        }
                        TokenKind::True => return ast::Expr::Literal(ast::Literal::Bool(true)),
                        TokenKind::False => return ast::Expr::Literal(ast::Literal::Bool(false)),
                        TokenKind::Nil => return ast::Expr::Literal(ast::Literal::Nil),
                        _ => {}
                    }
                }
                ast::Expr::Literal(ast::Literal::Nil)
            }
            NodeKind::Ident => {
                 if let Some(SyntaxElement::Token(token)) = node.children.first() {
                      let text = &self.source[token.span.lo.0 as usize..token.span.hi.0 as usize].to_string();
                      return ast::Expr::Ident(text.clone(), token.span);
                 }
                 ast::Expr::Literal(ast::Literal::Nil)
            }
            NodeKind::BinaryExpr => {
                 // Simplistic binary lowering
                 let lhs = self.lower_element(&node.children[0]);
                 let _op_tok = &node.children[1];
                 let rhs = self.lower_element(&node.children[2]);
                 
                 // TODO: Map op_tok to BinaryOp
                 ast::Expr::Binary(ast::BinaryOp::Add, Box::new(lhs), Box::new(rhs))
            }
            _ => ast::Expr::Literal(ast::Literal::Nil),
        }
    }

    fn lower_element(&self, element: &SyntaxElement) -> ast::Expr {
        match element {
            SyntaxElement::Node(node) => self.lower_expr(node),
            _ => ast::Expr::Literal(ast::Literal::Nil),
        }
    }
}
