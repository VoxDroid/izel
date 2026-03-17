use crate::*;
use izel_parser::ast;
use izel_typeck::type_system::Type;

pub struct MirLowerer {
    body: MirBody,
    current_block: BlockId,
    scopes: Vec<std::collections::HashMap<String, Local>>,
}

impl MirLowerer {
    pub fn new() -> Self {
        let body = MirBody::new();
        let entry = body.entry;
        Self { 
            body, 
            current_block: entry,
            scopes: vec![std::collections::HashMap::new()],
        }
    }

    pub fn lower_forge(&mut self, forge: &ast::Forge) -> MirBody {
        // In a real compiler, forge would already have types.
        // For now we'll assume they are available or we lower from typed AST.
        
        if let Some(body) = &forge.body {
            self.lower_block(body);
        }
        
        let block = self.body.blocks.node_weight_mut(self.current_block).unwrap();
        if block.terminator.is_none() {
            block.terminator = Some(Terminator::Return);
        }

        std::mem::replace(&mut self.body, MirBody::new())
    }

    fn lower_block(&mut self, block: &ast::Block) {
        self.scopes.push(std::collections::HashMap::new());
        for stmt in &block.stmts {
            self.lower_stmt(stmt);
        }
        if let Some(expr) = &block.expr {
            self.lower_expr(expr);
        }
        self.scopes.pop();
    }

    fn lower_stmt(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::Let { name, ty: _, init, .. } => {
                if let Some(val_expr) = init {
                    let rvalue = self.lower_expr(val_expr);
                    let local = Local(self.body.locals.len());
                    // Dummy type for now, should come from TAST
                    self.body.locals.push(LocalData { name: name.clone(), ty: Type::Error });
                    self.scopes.last_mut().unwrap().insert(name.clone(), local);
                    
                    let instr = Instruction::Assign(Place { local }, rvalue);
                    self.body.blocks.node_weight_mut(self.current_block).unwrap().instructions.push(instr);
                }
            }
            ast::Stmt::Expr(expr) => {
                self.lower_expr(expr);
            }
            _ => {}
        }
    }

    fn lower_expr(&mut self, expr: &ast::Expr) -> Rvalue {
        match expr {
            ast::Expr::Literal(lit) => {
                let constant = match lit {
                    ast::Literal::Int(v) => Constant::Int(*v),
                    ast::Literal::Float(v) => Constant::Float(*v),
                    ast::Literal::Bool(v) => Constant::Bool(*v),
                    ast::Literal::Str(s) => Constant::Str(s.clone()),
                    ast::Literal::Nil => Constant::Bool(false), // TODO
                };
                Rvalue::Use(Operand::Constant(constant))
            }
            ast::Expr::Ident(name, _) => {
                for scope in self.scopes.iter().rev() {
                    if let Some(&local) = scope.get(name) {
                        // In ownership logic, we'd decide Copy vs Move here
                        return Rvalue::Use(Operand::Copy(Place { local }));
                    }
                }
                Rvalue::Use(Operand::Constant(Constant::Int(0)))
            }
            ast::Expr::Unary(op, expr) => {
                let rvalue = self.lower_expr(expr);
                match op {
                    ast::UnaryOp::Ref(is_mut) => {
                        let operand = self.rvalue_to_operand(rvalue);
                        if let Operand::Copy(place) | Operand::Move(place) = operand {
                            Rvalue::Ref(place, *is_mut)
                        } else {
                            Rvalue::Use(operand)
                        }
                    }
                    ast::UnaryOp::Not => Rvalue::UnaryOp(UnOp::Not, self.rvalue_to_operand(rvalue)),
                    ast::UnaryOp::Neg => Rvalue::UnaryOp(UnOp::Neg, self.rvalue_to_operand(rvalue)),
                    _ => Rvalue::Use(self.rvalue_to_operand(rvalue)),
                }
            }
            ast::Expr::Binary(op, left, right) => {
                let lr = self.lower_expr(left);
                let rr = self.lower_expr(right);
                let l_op = self.rvalue_to_operand(lr);
                let r_op = self.rvalue_to_operand(rr);
                
                let mir_op = match op {
                    ast::BinaryOp::Add => BinOp::Add,
                    ast::BinaryOp::Sub => BinOp::Sub,
                    ast::BinaryOp::Mul => BinOp::Mul,
                    ast::BinaryOp::Div => BinOp::Div,
                    ast::BinaryOp::Eq => BinOp::Eq,
                    ast::BinaryOp::Ne => BinOp::Ne,
                    ast::BinaryOp::Lt => BinOp::Lt,
                    ast::BinaryOp::Le => BinOp::Le,
                    ast::BinaryOp::Gt => BinOp::Gt,
                    ast::BinaryOp::Ge => BinOp::Ge,
                    _ => BinOp::Add,
                };
                Rvalue::BinaryOp(mir_op, l_op, r_op)
            }
            _ => Rvalue::Use(Operand::Constant(Constant::Int(0))),
        }
    }

    fn rvalue_to_operand(&mut self, rvalue: Rvalue) -> Operand {
        match rvalue {
            Rvalue::Use(op) => op,
            _ => {
                let local = Local(self.body.locals.len());
                self.body.locals.push(LocalData { 
                    name: format!("tmp{}", local.0),
                    ty: Type::Error 
                });
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
