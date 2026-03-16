use crate::*;
use izel_parser::cst::{SyntaxNode, NodeKind, SyntaxElement};
use izel_lexer::TokenKind;

pub struct MirLowerer<'a> {
    source: &'a str,
    body: MirBody,
    current_block: BlockId,
    scopes: Vec<std::collections::HashMap<String, Local>>,
}

impl<'a> MirLowerer<'a> {
    pub fn new(source: &'a str) -> Self {
        let body = MirBody::new();
        let entry = body.entry;
        Self { 
            source, 
            body, 
            current_block: entry,
            scopes: vec![std::collections::HashMap::new()],
        }
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
        self.scopes.push(std::collections::HashMap::new());
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
        self.scopes.pop();
    }

    fn lower_let(&mut self, node: &SyntaxNode) {
        let mut name = "unnamed".to_string();
        let mut value_node = None;

        for child in &node.children {
            if let SyntaxElement::Token(token) = child {
                if token.kind == TokenKind::Ident {
                    let span = token.span;
                    name = self.source[span.lo.0 as usize..span.hi.0 as usize].to_string();
                }
            } else if let SyntaxElement::Node(child_node) = child {
                value_node = Some(child_node);
            }
        }

        if let Some(expr) = value_node {
            let rvalue = self.lower_expr(expr);
            
            let local = Local(self.body.locals.len());
            self.body.locals.push(LocalData { name: name.clone() });
            
            self.scopes.last_mut().unwrap().insert(name, local);
            
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
                let token = node.children.iter().find_map(|e| {
                    if let SyntaxElement::Token(t) = e { Some(t) } else { None }
                }).expect("Literal node should have a token");
                
                let text = &self.source[token.span.lo.0 as usize..token.span.hi.0 as usize];
                match token.kind {
                    TokenKind::Int { .. } => {
                        let val: i128 = text.replace('_', "").parse().unwrap_or(0);
                        Rvalue::Use(Operand::Constant(Constant::Int(val)))
                    }
                    TokenKind::Float => {
                        let val: f64 = text.replace('_', "").parse().unwrap_or(0.0);
                        Rvalue::Use(Operand::Constant(Constant::Float(val)))
                    }
                    TokenKind::True => Rvalue::Use(Operand::Constant(Constant::Bool(true))),
                    TokenKind::False => Rvalue::Use(Operand::Constant(Constant::Bool(false))),
                    TokenKind::Str { .. } => {
                        let s = text.trim_matches('"').to_string();
                        Rvalue::Use(Operand::Constant(Constant::Str(s)))
                    }
                    _ => Rvalue::Use(Operand::Constant(Constant::Int(0))),
                }
            }
            NodeKind::Ident => {
                let text = &self.source[node.span().lo.0 as usize..node.span().hi.0 as usize];
                for scope in self.scopes.iter().rev() {
                    if let Some(&local) = scope.get(text) {
                        return Rvalue::Use(Operand::Copy(Place { local }));
                    }
                }
                Rvalue::Use(Operand::Constant(Constant::Int(0))) 
            }
            NodeKind::ParenExpr => {
                let inner = node.children.iter().find_map(|e| {
                    if let SyntaxElement::Node(n) = e { Some(n) } else { None }
                }).expect("ParenExpr should have an inner expression");
                self.lower_expr(inner)
            }
            NodeKind::BinaryExpr => {
                let mut left = None;
                let mut op = None;
                let mut right = None;

                for child in &node.children {
                    match child {
                        SyntaxElement::Node(n) => {
                            if left.is_none() { left = Some(n); } else { right = Some(n); }
                        }
                        SyntaxElement::Token(t) => {
                            op = Some(match t.kind {
                                TokenKind::Plus => BinOp::Add,
                                TokenKind::Minus => BinOp::Sub,
                                TokenKind::Star => BinOp::Mul,
                                TokenKind::Slash => BinOp::Div,
                                TokenKind::EqEq => BinOp::Eq,
                                TokenKind::NotEq => BinOp::Ne,
                                TokenKind::Lt => BinOp::Lt,
                                TokenKind::Le => BinOp::Le,
                                TokenKind::Gt => BinOp::Gt,
                                TokenKind::Ge => BinOp::Ge,
                                _ => BinOp::Add, // Should not happen with correct parser
                            });
                        }
                    }
                }

                if let (Some(l), Some(o), Some(r)) = (left, op, right) {
                    let lr = self.lower_expr(l);
                    let rr = self.lower_expr(r);
                    
                    // We need operands for BinOp. Create temporary locals if needed?
                    // For now, let's assume we can simplify this or use a more complex Rvalue.
                    // Actually MIR Rvalue::BinaryOp takes Operands.
                    // I'll need to emit instructions to move Rvalues into locals.
                    
                    let l_op = self.rvalue_to_operand(lr);
                    let r_op = self.rvalue_to_operand(rr);
                    
                    Rvalue::BinaryOp(o, l_op, r_op)
                } else {
                    Rvalue::Use(Operand::Constant(Constant::Int(0)))
                }
            }
            _ => Rvalue::Use(Operand::Constant(Constant::Int(0))),
        }
    }

    fn rvalue_to_operand(&mut self, rvalue: Rvalue) -> Operand {
        match rvalue {
            Rvalue::Use(op) => op,
            _ => {
                let local = Local(self.body.locals.len());
                self.body.locals.push(LocalData { name: format!("tmp{}", local.0) });
                let instr = Instruction::Assign(Place { local }, rvalue);
                self.body.blocks.node_weight_mut(self.current_block).unwrap().instructions.push(instr);
                Operand::Move(Place { local })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use izel_lexer::Lexer;
    use izel_parser::Parser;
    use izel_span::SourceId;

    #[test]
    fn test_lower_let_add() {
        let source = "forge main() { let x = 1 + 2; }";
        let mut lexer = Lexer::new(source, SourceId(0));
        let mut tokens = Vec::new();
        loop {
            let t = lexer.next_token();
            tokens.push(t.clone());
            if t.kind == TokenKind::Eof { break; }
        }
        
        let mut parser = Parser::new(tokens);
        let cst = parser.parse_decl();
        
        let mut lowerer = MirLowerer::new(source);
        let mir = lowerer.lower_forge(&cst);

        // x is local 0
        assert_eq!(mir.locals.len(), 1);
        assert_eq!(mir.locals[0].name, "x");
        
        let block = mir.blocks.node_weight(mir.entry).unwrap();
        // [Assign(x, BinaryOp(Add, 1, 2))]
        assert_eq!(block.instructions.len(), 1);
    }
}
