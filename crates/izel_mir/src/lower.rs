use crate::*;
use izel_parser::cst::{SyntaxNode, NodeKind, SyntaxElement};
use izel_lexer::TokenKind;

pub struct MirLowerer<'a> {
    source: &'a str,
    body: MirBody,
    current_block: BlockId,
}

impl<'a> MirLowerer<'a> {
    pub fn new(source: &'a str) -> Self {
        let body = MirBody::new();
        let entry = body.entry;
        Self { source, body, current_block: entry }
    }

    pub fn lower_forge(&mut self, node: &SyntaxNode) -> MirBody {
        // Find the block
        for child in &node.children {
            if let SyntaxElement::Node(child_node) = child {
                if child_node.kind == NodeKind::Block {
                    self.lower_block(child_node);
                }
            }
        }
        
        // Ensure terminator
        let block = self.body.blocks.node_weight_mut(self.current_block).unwrap();
        if block.terminator.is_none() {
            block.terminator = Some(Terminator::Return);
        }

        std::mem::replace(&mut self.body, MirBody::new())
    }

    fn lower_block(&mut self, node: &SyntaxNode) {
        for child in &node.children {
            if let SyntaxElement::Node(child_node) = child {
                match child_node.kind {
                    NodeKind::LetStmt => {
                        self.lower_let(child_node);
                    }
                    NodeKind::ExprStmt => {
                        self.lower_expr_stmt(child_node);
                    }
                    NodeKind::Block => {
                        self.lower_block(child_node);
                    }
                    _ => {}
                }
            }
        }
    }

    fn lower_let(&mut self, node: &SyntaxNode) {
        let mut _name = "unnamed".to_string();
        let mut value_node = None;

        for child in &node.children {
            if let SyntaxElement::Token(token) = child {
                if token.kind == TokenKind::Ident {
                    let span = token.span;
                    _name = self.source[span.lo.0 as usize..span.hi.0 as usize].to_string();
                }
            } else if let SyntaxElement::Node(child_node) = child {
                value_node = Some(child_node);
            }
        }

        if let Some(expr) = value_node {
            let rvalue = self.lower_expr(expr);
            // StorageLive
            let local = Local(self.body.blocks.node_weight(self.current_block).unwrap().instructions.len()); 
            // This is just a placeholder local index
            
            let instr = Instruction::Assign(
                Place { local },
                rvalue
            );
            self.body.blocks.node_weight_mut(self.current_block).unwrap().instructions.push(instr);
        }
    }

    fn lower_expr_stmt(&mut self, node: &SyntaxNode) {
        for child in &node.children {
            if let SyntaxElement::Node(child_node) = child {
                self.lower_expr(child_node);
            }
        }
    }

    fn lower_expr(&mut self, node: &SyntaxNode) -> Rvalue {
        match node.kind {
            NodeKind::Literal => {
                // Return a constant
                Rvalue::Use(Operand::Constant(Constant::Int(0))) // Stub
            }
            NodeKind::BinaryExpr => {
                // Recurse
                Rvalue::Use(Operand::Constant(Constant::Int(0))) // Stub
            }
            _ => Rvalue::Use(Operand::Constant(Constant::Int(0))),
        }
    }
}
