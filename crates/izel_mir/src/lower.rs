use crate::*;
use izel_parser::ast;
use izel_typeck::type_system::Type;

pub struct MirLowerer {
    body: MirBody,
    current_block: BlockId,
    scopes: Vec<std::collections::HashMap<String, Local>>,
    pub check_contracts: bool,
}

impl MirLowerer {
    pub fn new() -> Self {
        let body = MirBody::new();
        let entry = body.entry;
        Self { 
            body, 
            current_block: entry,
            scopes: vec![std::collections::HashMap::new()],
            check_contracts: false,
        }
    }

    fn lower_type(&self, ty: &ast::Type) -> Type {
        match ty {
            ast::Type::Prim(name) => Type::Prim(self.map_prim(name)),
            ast::Type::Pointer(inner, is_mut) => Type::Pointer(Box::new(self.lower_type(inner)), *is_mut, izel_typeck::type_system::Lifetime::Anonymous(0)),
            ast::Type::Optional(inner) => Type::Optional(Box::new(self.lower_type(inner))),
            _ => Type::Error,
        }
    }

    fn map_prim(&self, name: &str) -> izel_typeck::type_system::PrimType {
        match name {
            "i32" => izel_typeck::type_system::PrimType::I32,
            "f64" => izel_typeck::type_system::PrimType::F64,
            "bool" => izel_typeck::type_system::PrimType::Bool,
            "str" => izel_typeck::type_system::PrimType::Str,
            _ => izel_typeck::type_system::PrimType::I32,
        }
    }

    pub fn lower_forge(&mut self, forge: &ast::Forge) -> MirBody {
        self.scopes.last_mut().unwrap().clear();
        
        // Lower parameters
        for param in &forge.params {
            let local = Local(self.body.locals.len());
            let ty = self.lower_type(&param.ty);
            self.body.locals.push(LocalData { name: param.name.clone(), ty });
            self.scopes.last_mut().unwrap().insert(param.name.clone(), local);
        }

        // Inject @requires assertions at function entry (when --check-contracts is on)
        if self.check_contracts {
            for (i, req) in forge.requires.iter().enumerate() {
                let cond_rv = self.lower_expr(req);
                let cond_op = self.rvalue_to_operand(cond_rv);
                let msg = format!("precondition #{} of '{}' violated", i, forge.name);
                let instr = Instruction::Assert(cond_op, msg);
                self.body.blocks.node_weight_mut(self.current_block).unwrap().instructions.push(instr);
            }
        }

        if let Some(body) = &forge.body {
            self.lower_block(body);
        }
        
        // Inject @ensures assertions before return (when --check-contracts is on)
        if self.check_contracts && !forge.ensures.is_empty() {
            for (i, ens) in forge.ensures.iter().enumerate() {
                let cond_rv = self.lower_expr(ens);
                let cond_op = self.rvalue_to_operand(cond_rv);
                let msg = format!("postcondition #{} of '{}' violated", i, forge.name);
                let instr = Instruction::Assert(cond_op, msg);
                self.body.blocks.node_weight_mut(self.current_block).unwrap().instructions.push(instr);
            }
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
            ast::Stmt::Let { name, ty, init, .. } => {
                let local = Local(self.body.locals.len());
                let mir_ty = if let Some(t) = ty { self.lower_type(t) } else { Type::Error };
                self.body.locals.push(LocalData { name: name.clone(), ty: mir_ty });
                self.scopes.last_mut().unwrap().insert(name.clone(), local);

                if let Some(val_expr) = init {
                    let rvalue = self.lower_expr(val_expr);
                    let instr = Instruction::Assign(Place { local }, rvalue);
                    self.body.blocks.node_weight_mut(self.current_block).unwrap().instructions.push(instr);
                }
            }
            ast::Stmt::Expr(expr) => {
                self.lower_expr(expr);
            }
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
                        return Rvalue::Use(Operand::Move(Place { local }));
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
            ast::Expr::Call(callee, args) => {
                let callee_name = if let ast::Expr::Ident(name, _) = &**callee {
                    name.clone()
                } else {
                    "unknown".to_string()
                };
                let mut operands = Vec::new();
                for arg in args {
                    let rv = self.lower_expr(arg);
                    operands.push(self.rvalue_to_operand(rv));
                }
                let local = Local(self.body.locals.len());
                self.body.locals.push(LocalData { name: format!("call_tmp{}", local.0), ty: Type::Error });
                let instr = Instruction::Call(Place { local }, callee_name, operands);
                self.body.blocks.node_weight_mut(self.current_block).unwrap().instructions.push(instr);
                Rvalue::Use(Operand::Move(Place { local }))
            }
            ast::Expr::Given { cond, then_block, else_expr } => {
                let cond_rv = self.lower_expr(cond);
                let cond_op = self.rvalue_to_operand(cond_rv);
                
                let then_id = self.body.blocks.add_node(BasicBlock { instructions: Vec::new(), terminator: None });
                let else_id = self.body.blocks.add_node(BasicBlock { instructions: Vec::new(), terminator: None });
                let join_id = self.body.blocks.add_node(BasicBlock { instructions: Vec::new(), terminator: None });
                
                self.body.blocks.add_edge(self.current_block, then_id, ControlFlow::Conditional(true));
                self.body.blocks.add_edge(self.current_block, else_id, ControlFlow::Conditional(false));
                
                self.body.blocks.node_weight_mut(self.current_block).unwrap().terminator = Some(Terminator::SwitchInt(cond_op, vec![(1, then_id)], else_id));
                
                // Then branch
                self.current_block = then_id;
                self.lower_block(then_block);
                if self.body.blocks[self.current_block].terminator.is_none() {
                    self.body.blocks.add_edge(self.current_block, join_id, ControlFlow::Unconditional);
                    self.body.blocks[self.current_block].terminator = Some(Terminator::Goto(join_id));
                }

                // Else branch
                self.current_block = else_id;
                if let Some(el) = else_expr {
                    self.lower_expr(el);
                }
                if self.body.blocks[self.current_block].terminator.is_none() {
                    self.body.blocks.add_edge(self.current_block, join_id, ControlFlow::Unconditional);
                    self.body.blocks[self.current_block].terminator = Some(Terminator::Goto(join_id));
                }

                self.current_block = join_id;
                Rvalue::Use(Operand::Constant(Constant::Int(0))) // Dummy
            }
            ast::Expr::While { cond, body } => {
                let cond_head = self.body.blocks.add_node(BasicBlock { instructions: Vec::new(), terminator: None });
                let body_id = self.body.blocks.add_node(BasicBlock { instructions: Vec::new(), terminator: None });
                let exit_id = self.body.blocks.add_node(BasicBlock { instructions: Vec::new(), terminator: None });
                
                self.body.blocks.add_edge(self.current_block, cond_head, ControlFlow::Unconditional);
                self.body.blocks[self.current_block].terminator = Some(Terminator::Goto(cond_head));
                
                self.current_block = cond_head;
                let cond_rv = self.lower_expr(cond);
                let cond_op = self.rvalue_to_operand(cond_rv);
                self.body.blocks.add_edge(cond_head, body_id, ControlFlow::Conditional(true));
                self.body.blocks.add_edge(cond_head, exit_id, ControlFlow::Conditional(false));
                self.body.blocks[cond_head].terminator = Some(Terminator::SwitchInt(cond_op, vec![(1, body_id)], exit_id));
                
                self.current_block = body_id;
                self.lower_block(body);
                self.body.blocks.add_edge(self.current_block, cond_head, ControlFlow::Unconditional);
                self.body.blocks[self.current_block].terminator = Some(Terminator::Goto(cond_head));
                
                self.current_block = exit_id;
                Rvalue::Use(Operand::Constant(Constant::Int(0)))
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
    use izel_span::SourceId;
    use izel_parser::ast;

    #[test]
    fn test_lower_let_add() {
        let mut forge = ast::Forge {
            name: "main".into(),
            generic_params: vec![],
            params: vec![],
            ret_type: ast::Type::Prim("i32".into()),
            effects: vec![],
            attributes: vec![],
            requires: vec![],
            ensures: vec![],
            body: Some(ast::Block {
                stmts: vec![
                    ast::Stmt::Let {
                        name: "x".into(),
                        ty: Some(ast::Type::Prim("i32".into())),
                        init: Some(ast::Expr::Binary(
                            ast::BinaryOp::Add,
                            Box::new(ast::Expr::Literal(ast::Literal::Int(1))),
                            Box::new(ast::Expr::Literal(ast::Literal::Int(2))),
                        )),
                        span: izel_span::Span::dummy(),
                    }
                ],
                expr: None,
                span: izel_span::Span::dummy(),
            }),
            span: izel_span::Span::dummy(),
        };
        
        let mut lowerer = MirLowerer::new();
        let mir = lowerer.lower_forge(&mut forge);

        assert_eq!(mir.locals.len(), 1);
        assert_eq!(mir.locals[0].name, "x");
        
        let block = mir.blocks.node_weight(mir.entry).unwrap();
        assert_eq!(block.instructions.len(), 1);
    }

    #[test]
    fn test_lower_given() {
        let mut forge = ast::Forge {
            name: "test_if".into(),
            generic_params: vec![],
            params: vec![],
            ret_type: ast::Type::Prim("i32".into()),
            effects: vec![],
            attributes: vec![],
            requires: vec![],
            ensures: vec![],
            body: Some(ast::Block {
                stmts: vec![
                    ast::Stmt::Expr(ast::Expr::Given {
                        cond: Box::new(ast::Expr::Literal(ast::Literal::Bool(true))),
                        then_block: ast::Block {
                            stmts: vec![],
                            expr: Some(Box::new(ast::Expr::Literal(ast::Literal::Int(1)))),
                            span: izel_span::Span::dummy(),
                        },
                        else_expr: Some(Box::new(ast::Expr::Literal(ast::Literal::Int(2)))),
                    })
                ],
                expr: None,
                span: izel_span::Span::dummy(),
            }),
            span: izel_span::Span::dummy(),
        };

        let mut lowerer = MirLowerer::new();
        let mir = lowerer.lower_forge(&mut forge);
        
        // Entry block -> Switch -> (Then block | Else block) -> Join block
        // entry + then + else + join = 4 nodes.
        assert_eq!(mir.blocks.node_count(), 4);
    }
}
