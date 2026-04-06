use crate::*;
use izel_hir::{HirBlock, HirExpr, HirForge, HirStmt};
use izel_parser::ast::{BinaryOp, UnaryOp};
use izel_resolve::DefId;
use izel_typeck::type_system::{PrimType, Type};
use std::collections::HashMap;

pub struct MirLowerer {
    body: MirBody,
    current_block: BlockId,
    header: BlockId,
    forge_name: String,
    param_defs: Vec<DefId>,
    /// Map from DefId to Local slot
    vars: HashMap<DefId, Local>,
    pub check_contracts: bool,
    current_ensures: Vec<HirExpr>,
}

impl Default for MirLowerer {
    fn default() -> Self {
        Self::new()
    }
}

impl MirLowerer {
    pub fn new() -> Self {
        let body = MirBody::new();
        let entry = body.entry;
        Self {
            body,
            current_block: entry,
            header: entry,
            forge_name: String::new(),
            param_defs: Vec::new(),
            vars: HashMap::new(),
            check_contracts: false,
            current_ensures: Vec::new(),
        }
    }

    pub fn lower_forge(&mut self, forge: &HirForge) -> MirBody {
        self.vars.clear();
        self.current_ensures = forge.ensures.clone();
        self.body.arg_count = forge.params.len();
        self.forge_name = forge.name.clone();
        self.param_defs = forge.params.iter().map(|p| p.def_id).collect();

        let entry = self.body.entry;
        let header = self.body.blocks.add_node(BasicBlock {
            instructions: Vec::new(),
            terminator: None,
        });
        self.header = header;

        // Lower parameters as initial definitions in entry
        for param in &forge.params {
            let local = self.new_local(param.name.clone(), param.ty.clone());
            self.write_variable(param.def_id, local);
        }

        self.body
            .blocks
            .add_edge(entry, header, ControlFlow::Unconditional);
        self.body.blocks[entry].terminator = Some(Terminator::Goto(header));
        self.current_block = header;

        if let Some(body) = &forge.body {
            self.lower_block(body);
        }

        let block = &mut self.body.blocks[self.current_block];
        if block.terminator.is_none() {
            block.terminator = Some(Terminator::Return(None));
        }

        std::mem::take(&mut self.body)
    }

    fn new_local(&mut self, name: String, ty: Type) -> Local {
        let id = self.body.locals.len();
        self.body.locals.push(LocalData { name, ty });
        Local(id)
    }

    fn write_variable(&mut self, var: DefId, local: Local) {
        self.vars.insert(var, local);
    }

    fn read_variable(&mut self, var: DefId) -> Local {
        self.get_var_local(var)
    }

    fn get_var_local(&mut self, var: DefId) -> Local {
        if let Some(&local) = self.vars.get(&var) {
            local
        } else {
            let local = self.new_local(format!("v{:?}", var), Type::Error);
            self.vars.insert(var, local);
            local
        }
    }

    fn lower_block(&mut self, block: &HirBlock) -> Rvalue {
        for stmt in &block.stmts {
            self.lower_stmt(stmt);
        }
        if let Some(expr) = &block.expr {
            self.lower_expr(expr)
        } else {
            Rvalue::Use(Operand::Constant(Constant::Bool(false)))
        }
    }

    fn lower_stmt(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Let {
                name,
                def_id,
                ty,
                init,
                ..
            } => {
                if let Some(val_expr) = init {
                    let rvalue = self.lower_expr(val_expr);
                    let local = self.new_local(name.clone(), ty.clone());
                    self.body.blocks[self.current_block]
                        .instructions
                        .push(Instruction::Assign(local, rvalue));
                    self.write_variable(*def_id, local);
                }
            }
            HirStmt::Assign { def_id, expr, .. } => {
                let rvalue = self.lower_expr(expr);
                let local = self.read_variable(*def_id);
                self.body.blocks[self.current_block]
                    .instructions
                    .push(Instruction::Assign(local, rvalue));
            }
            HirStmt::Expr(expr) => {
                self.lower_expr(expr);
            }
        }
    }

    fn lower_expr(&mut self, expr: &HirExpr) -> Rvalue {
        match expr {
            HirExpr::Literal(lit) => {
                let constant = match lit {
                    izel_parser::ast::Literal::Int(v) => Constant::Int(*v),
                    izel_parser::ast::Literal::Float(v) => Constant::Float(*v),
                    izel_parser::ast::Literal::Bool(v) => Constant::Bool(*v),
                    izel_parser::ast::Literal::Str(s) => Constant::Str(s.clone()),
                    izel_parser::ast::Literal::Nil => Constant::Bool(false),
                };
                Rvalue::Use(Operand::Constant(constant))
            }
            HirExpr::Ident(_, var, _ty, _) => {
                let local = self.read_variable(*var);
                Rvalue::Use(Operand::Copy(local))
            }
            HirExpr::Zone { name, body, .. } => {
                println!("  Adding ZoneEnter to {:?}: {}", self.current_block, name);
                self.body.blocks[self.current_block]
                    .instructions
                    .push(Instruction::ZoneEnter(name.clone()));
                let rv = self.lower_block(body);
                println!("  Adding ZoneExit to {:?}: {}", self.current_block, name);
                self.body.blocks[self.current_block]
                    .instructions
                    .push(Instruction::ZoneExit(name.clone()));
                rv
            }
            HirExpr::Binary(op, left, right, _) => {
                let lr = self.lower_expr(left);
                let l_op = self.rvalue_to_operand(lr);
                let rr = self.lower_expr(right);
                let r_op = self.rvalue_to_operand(rr);

                let mir_op = match op {
                    BinaryOp::Add => BinOp::Add,
                    BinaryOp::Sub => BinOp::Sub,
                    BinaryOp::Mul => BinOp::Mul,
                    BinaryOp::Div => BinOp::Div,
                    BinaryOp::Eq => BinOp::Eq,
                    BinaryOp::Ne => BinOp::Ne,
                    BinaryOp::Lt => BinOp::Lt,
                    BinaryOp::Le => BinOp::Le,
                    BinaryOp::Gt => BinOp::Gt,
                    BinaryOp::Ge => BinOp::Ge,
                    _ => BinOp::Add,
                };
                Rvalue::Binary(mir_op, l_op, r_op)
            }
            HirExpr::Unary(op, inner, _) => {
                let rv = self.lower_expr(inner);
                let op_val = self.rvalue_to_operand(rv);

                let mir_op = match op {
                    UnaryOp::Neg => UnOp::Neg,
                    UnaryOp::Not => UnOp::Not,
                    _ => UnOp::Neg,
                };
                Rvalue::Unary(mir_op, op_val)
            }
            HirExpr::Call(callee, args, requires, ret_ty) => {
                let mut operands = Vec::new();
                for arg in args {
                    let rv = self.lower_expr(arg);
                    operands.push(self.rvalue_to_operand(rv));
                }

                // Emit runtime assertions for @requires when runtime checking is enabled.
                if self.check_contracts {
                    for req in requires {
                        let req_rv = self.lower_expr(req);
                        let req_op = self.rvalue_to_operand(req_rv);
                        self.body.blocks[self.current_block].instructions.push(
                            Instruction::Assert(req_op, "precondition violation".to_string()),
                        );
                    }
                }

                let callee_name = if let HirExpr::Ident(name, _, _, _) = &**callee {
                    name.clone()
                } else {
                    "unknown".to_string()
                };

                if let Type::Prim(PrimType::Void) = ret_ty {
                    self.body.blocks[self.current_block]
                        .instructions
                        .push(Instruction::Call(None, callee_name, operands));
                    Rvalue::Use(Operand::Constant(Constant::Bool(false))) // Return dummy
                } else {
                    let local = self.new_local("call_tmp".to_string(), ret_ty.clone());
                    self.body.blocks[self.current_block]
                        .instructions
                        .push(Instruction::Call(Some(local), callee_name, operands));
                    Rvalue::Use(Operand::Move(local))
                }
            }
            HirExpr::Return(expr) => {
                if let Some(e) = expr {
                    if let HirExpr::Call(callee, args, _, _) = &**e {
                        // Check for TCO
                        if let HirExpr::Ident(name, _, _, _) = &**callee {
                            if name == &self.forge_name {
                                // TCO transformation:
                                let mut arg_ops: Vec<Operand> = Vec::new();
                                for arg in args {
                                    let rv = self.lower_expr(arg);
                                    arg_ops.push(self.rvalue_to_operand(rv));
                                }
                                // Re-assign params
                                let param_defs = self.param_defs.clone();
                                for (i, def_id) in param_defs.iter().enumerate() {
                                    if i < arg_ops.len() {
                                        let local =
                                            self.new_local(format!("tco_p{}", i), Type::Error);
                                        self.body.blocks[self.current_block].instructions.push(
                                            Instruction::Assign(
                                                local,
                                                Rvalue::Use(arg_ops[i].clone()),
                                            ),
                                        );
                                        self.write_variable(*def_id, local);
                                    }
                                }
                                self.body.blocks.add_edge(
                                    self.current_block,
                                    self.header,
                                    ControlFlow::Unconditional,
                                );
                                self.body.blocks[self.current_block].terminator =
                                    Some(Terminator::Goto(self.header));
                                return Rvalue::Use(Operand::Constant(Constant::Int(0)));
                            }
                        }
                    }
                    let rv = self.lower_expr(e);
                    let op = self.rvalue_to_operand(rv);

                    // Emit runtime assertions for @ensures on explicit returns.
                    if self.check_contracts && !self.current_ensures.is_empty() {
                        for ens in self.current_ensures.clone() {
                            let substituted = self.substitute_result_expr(&ens, e);
                            let ens_rv = self.lower_expr(&substituted);
                            let ens_op = self.rvalue_to_operand(ens_rv);
                            self.body.blocks[self.current_block].instructions.push(
                                Instruction::Assert(ens_op, "postcondition violation".to_string()),
                            );
                        }
                    }

                    self.body.blocks[self.current_block].terminator =
                        Some(Terminator::Return(Some(op)));
                    Rvalue::Use(Operand::Constant(Constant::Int(0))) // DUMMY
                } else {
                    self.body.blocks[self.current_block].terminator =
                        Some(Terminator::Return(None));
                    Rvalue::Use(Operand::Constant(Constant::Int(0)))
                }
            }
            HirExpr::Given {
                cond,
                then_block,
                else_expr,
                ..
            } => {
                let cond_rv = self.lower_expr(cond);
                let cond_op = self.rvalue_to_operand(cond_rv);

                let then_id = self.body.blocks.add_node(BasicBlock {
                    instructions: Vec::new(),
                    terminator: None,
                });
                let else_id = self.body.blocks.add_node(BasicBlock {
                    instructions: Vec::new(),
                    terminator: None,
                });
                let join_id = self.body.blocks.add_node(BasicBlock {
                    instructions: Vec::new(),
                    terminator: None,
                });

                self.body.blocks.add_edge(
                    self.current_block,
                    then_id,
                    ControlFlow::Conditional(true),
                );
                self.body.blocks.add_edge(
                    self.current_block,
                    else_id,
                    ControlFlow::Conditional(false),
                );
                self.body.blocks[self.current_block].terminator =
                    Some(Terminator::SwitchInt(cond_op, vec![(1, then_id)], else_id));

                self.current_block = then_id;
                self.lower_block(then_block);
                if self.body.blocks[self.current_block].terminator.is_none() {
                    self.body.blocks.add_edge(
                        self.current_block,
                        join_id,
                        ControlFlow::Unconditional,
                    );
                    self.body.blocks[self.current_block].terminator =
                        Some(Terminator::Goto(join_id));
                }

                self.current_block = else_id;
                if let Some(el) = else_expr {
                    self.lower_expr(el);
                }
                if self.body.blocks[self.current_block].terminator.is_none() {
                    self.body.blocks.add_edge(
                        self.current_block,
                        join_id,
                        ControlFlow::Unconditional,
                    );
                    self.body.blocks[self.current_block].terminator =
                        Some(Terminator::Goto(join_id));
                }

                self.current_block = join_id;

                Rvalue::Use(Operand::Constant(Constant::Int(0)))
            }
            _ => Rvalue::Use(Operand::Constant(Constant::Int(0))),
        }
    }

    fn rvalue_to_operand(&mut self, rvalue: Rvalue) -> Operand {
        match rvalue {
            Rvalue::Use(op) => op,
            _ => {
                let local = self.new_local("tmp".to_string(), Type::Error);
                self.body.blocks[self.current_block]
                    .instructions
                    .push(Instruction::Assign(local, rvalue));
                Operand::Move(local)
            }
        }
    }

    fn substitute_result_expr(&self, expr: &HirExpr, result_expr: &HirExpr) -> HirExpr {
        match expr {
            HirExpr::Ident(name, _, _, _) if name == "result" => result_expr.clone(),
            HirExpr::Binary(op, lhs, rhs, ty) => HirExpr::Binary(
                op.clone(),
                Box::new(self.substitute_result_expr(lhs, result_expr)),
                Box::new(self.substitute_result_expr(rhs, result_expr)),
                ty.clone(),
            ),
            HirExpr::Unary(op, inner, ty) => HirExpr::Unary(
                op.clone(),
                Box::new(self.substitute_result_expr(inner, result_expr)),
                ty.clone(),
            ),
            HirExpr::Call(callee, args, requires, ret_ty) => HirExpr::Call(
                Box::new(self.substitute_result_expr(callee, result_expr)),
                args.iter()
                    .map(|a| self.substitute_result_expr(a, result_expr))
                    .collect(),
                requires
                    .iter()
                    .map(|r| self.substitute_result_expr(r, result_expr))
                    .collect(),
                ret_ty.clone(),
            ),
            _ => expr.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use izel_hir::*;
    use izel_span::Span;

    #[test]
    fn test_lower_ssa_let() {
        let forge = HirForge {
            name: "main".into(),
            name_span: Span::dummy(),
            def_id: DefId(0),
            params: vec![],
            ret_type: Type::Error,
            attributes: vec![],
            body: Some(HirBlock {
                stmts: vec![HirStmt::Let {
                    name: "x".into(),
                    def_id: DefId(10),
                    ty: Type::Error,
                    init: Some(HirExpr::Literal(izel_parser::ast::Literal::Int(1))),
                    span: Span::dummy(),
                }],
                expr: None,
                span: Span::dummy(),
            }),
            requires: vec![],
            ensures: vec![],
            span: Span::dummy(),
        };
        let mut lowerer = MirLowerer::new();
        let _mir = lowerer.lower_forge(&forge);
    }

    #[test]
    fn test_tco() {
        let x_def = DefId(10);
        let forge = HirForge {
            name: "fact".into(),
            name_span: Span::dummy(),
            def_id: DefId(0),
            params: vec![HirParam {
                name: "n".into(),
                def_id: x_def,
                ty: Type::Error,
                default_value: None,
                is_variadic: false,
                span: Span::dummy(),
            }],
            ret_type: Type::Error,
            attributes: vec![],
            body: Some(HirBlock {
                stmts: vec![],
                expr: Some(Box::new(HirExpr::Return(Some(Box::new(HirExpr::Call(
                    Box::new(HirExpr::Ident(
                        "fact".to_string(),
                        DefId(0),
                        Type::Error,
                        Span::dummy(),
                    )),
                    vec![HirExpr::Literal(izel_parser::ast::Literal::Int(0))],
                    vec![],
                    Type::Error,
                )))))),
                span: Span::dummy(),
            }),
            requires: vec![],
            ensures: vec![],
            span: Span::dummy(),
        };

        let mut lowerer = MirLowerer::new();
        let mir = lowerer.lower_forge(&forge);

        // Check for back-edge to header
        let mut has_back_edge = false;
        for edge in mir.blocks.edge_indices() {
            let (u, v) = mir.blocks.edge_endpoints(edge).unwrap();
            // Header is index 1.
            if v.index() == 1 && u.index() >= 1 {
                has_back_edge = true;
            }
        }
        assert!(
            has_back_edge,
            "TCO should have created a back-edge to the header block"
        );
    }

    #[test]
    fn test_contract_assertion_emission() {
        let mut lowerer = MirLowerer::new();
        lowerer.check_contracts = true;
        let i32_ty = Type::Prim(izel_typeck::type_system::PrimType::I32);

        // 1. Mock a call to 'f(n)' with @requires(n > 0)
        let n_id = DefId(10);
        let n_expr = HirExpr::Ident(
            "n".to_string(),
            n_id,
            i32_ty.clone(),
            izel_span::Span::dummy(),
        );

        let requires = vec![HirExpr::Binary(
            izel_parser::ast::BinaryOp::Gt,
            Box::new(n_expr.clone()),
            Box::new(HirExpr::Literal(izel_parser::ast::Literal::Int(0))),
            Type::Prim(PrimType::Bool),
        )];

        let callee = Box::new(HirExpr::Ident(
            "f".to_string(),
            DefId(20),
            Type::Error,
            izel_span::Span::dummy(),
        ));
        let call_expr = HirExpr::Call(callee, vec![n_expr], requires, i32_ty.clone());

        // 2. Lower the call
        lowerer.lower_expr(&call_expr);

        // 3. Verify that the MIR contains an Assert instruction
        let mir = &lowerer.body;
        let mut found_assert = false;
        for node in mir.blocks.node_indices() {
            for inst in &mir.blocks[node].instructions {
                if let Instruction::Assert(_, msg) = inst {
                    if msg == "precondition violation" {
                        found_assert = true;
                    }
                }
            }
        }
        assert!(
            found_assert,
            "MIR should contain an Assert instruction for the @requires contract"
        );
    }

    #[test]
    fn test_witness_typed_call_skips_runtime_assertions() {
        let mut lowerer = MirLowerer::new();
        lowerer.check_contracts = true;
        let i32_ty = Type::Prim(izel_typeck::type_system::PrimType::I32);
        let nz_ty = Type::BuiltinWitness(
            izel_typeck::type_system::BuiltinWitness::NonZero,
            Box::new(i32_ty.clone()),
        );

        // Simulate calling divide(a: i32, b: NonZero<i32>) where proof is
        // encoded in the type and no @requires runtime assertion is needed.
        let callee = Box::new(HirExpr::Ident(
            "divide".to_string(),
            DefId(30),
            Type::Error,
            izel_span::Span::dummy(),
        ));

        let a_expr = HirExpr::Literal(izel_parser::ast::Literal::Int(42));
        let b_expr = HirExpr::Ident("nz".to_string(), DefId(31), nz_ty, izel_span::Span::dummy());

        let call_expr = HirExpr::Call(callee, vec![a_expr, b_expr], vec![], i32_ty);
        lowerer.lower_expr(&call_expr);

        let mir = &lowerer.body;
        let found_assert = mir
            .blocks
            .node_indices()
            .flat_map(|node| mir.blocks[node].instructions.iter())
            .any(|inst| matches!(inst, Instruction::Assert(_, _)));

        assert!(
            !found_assert,
            "witness-typed call should not emit runtime contract asserts"
        );
    }

    #[test]
    fn test_contract_assertions_disabled_by_default() {
        let mut lowerer = MirLowerer::new();
        let i32_ty = Type::Prim(izel_typeck::type_system::PrimType::I32);

        let n_id = DefId(10);
        let n_expr = HirExpr::Ident(
            "n".to_string(),
            n_id,
            i32_ty.clone(),
            izel_span::Span::dummy(),
        );
        let requires = vec![HirExpr::Binary(
            izel_parser::ast::BinaryOp::Gt,
            Box::new(n_expr.clone()),
            Box::new(HirExpr::Literal(izel_parser::ast::Literal::Int(0))),
            Type::Prim(PrimType::Bool),
        )];
        let callee = Box::new(HirExpr::Ident(
            "f".to_string(),
            DefId(20),
            Type::Error,
            izel_span::Span::dummy(),
        ));
        let call_expr = HirExpr::Call(callee, vec![n_expr], requires, i32_ty);

        lowerer.lower_expr(&call_expr);

        let found_assert = lowerer
            .body
            .blocks
            .node_indices()
            .flat_map(|node| lowerer.body.blocks[node].instructions.iter())
            .any(|inst| matches!(inst, Instruction::Assert(_, _)));
        assert!(
            !found_assert,
            "runtime contract asserts must be disabled unless check_contracts is enabled"
        );
    }

    #[test]
    fn test_postcondition_assertion_emission() {
        let mut lowerer = MirLowerer::new();
        lowerer.check_contracts = true;

        let i32_ty = Type::Prim(izel_typeck::type_system::PrimType::I32);
        let ensure = HirExpr::Binary(
            izel_parser::ast::BinaryOp::Gt,
            Box::new(HirExpr::Ident(
                "result".to_string(),
                DefId(1000),
                i32_ty.clone(),
                izel_span::Span::dummy(),
            )),
            Box::new(HirExpr::Literal(izel_parser::ast::Literal::Int(0))),
            Type::Prim(PrimType::Bool),
        );

        let forge = HirForge {
            name: "ensured".into(),
            name_span: Span::dummy(),
            def_id: DefId(0),
            params: vec![],
            ret_type: i32_ty,
            attributes: vec![],
            body: Some(HirBlock {
                stmts: vec![],
                expr: Some(Box::new(HirExpr::Return(Some(Box::new(HirExpr::Literal(
                    izel_parser::ast::Literal::Int(1),
                )))))),
                span: Span::dummy(),
            }),
            requires: vec![],
            ensures: vec![ensure],
            span: Span::dummy(),
        };

        let mir = lowerer.lower_forge(&forge);
        let mut found_post_assert = false;
        for node in mir.blocks.node_indices() {
            for inst in &mir.blocks[node].instructions {
                if let Instruction::Assert(_, msg) = inst {
                    if msg == "postcondition violation" {
                        found_post_assert = true;
                    }
                }
            }
        }

        assert!(
            found_post_assert,
            "MIR should contain an Assert instruction for @ensures when runtime checks are enabled"
        );
    }

    #[test]
    fn test_zone_lowering() {
        let mut lowerer = MirLowerer::new();
        let i32_ty = Type::Prim(izel_typeck::type_system::PrimType::I32);

        let body = HirBlock {
            stmts: vec![],
            expr: Some(Box::new(HirExpr::Literal(izel_parser::ast::Literal::Int(
                42,
            )))),
            span: izel_span::Span::dummy(),
        };

        let zone_expr = HirExpr::Zone {
            name: "temp_arena".to_string(),
            body,
            ty: i32_ty.clone(),
        };

        lowerer.lower_expr(&zone_expr);

        let mir = &lowerer.body;
        let mut found_enter = false;
        let mut found_exit = false;

        for node in mir.blocks.node_indices() {
            for inst in &mir.blocks[node].instructions {
                if let Instruction::ZoneEnter(name) = inst {
                    if name == "temp_arena" {
                        found_enter = true;
                    }
                }
                if let Instruction::ZoneExit(name) = inst {
                    if name == "temp_arena" {
                        found_exit = true;
                    }
                }
            }
        }

        assert!(found_enter, "MIR should contain ZoneEnter instruction");
        assert!(found_exit, "MIR should contain ZoneExit instruction");
    }

    #[test]
    fn test_lower_stmt_and_expr_cover_additional_branches() {
        let mut lowerer = MirLowerer::default();
        let i32_ty = Type::Prim(PrimType::I32);

        lowerer.lower_stmt(&HirStmt::Assign {
            def_id: DefId(700),
            expr: HirExpr::Literal(izel_parser::ast::Literal::Int(5)),
            span: Span::dummy(),
        });

        lowerer.lower_stmt(&HirStmt::Expr(HirExpr::Unary(
            UnaryOp::BitNot,
            Box::new(HirExpr::Literal(izel_parser::ast::Literal::Int(1))),
            i32_ty.clone(),
        )));

        let _ = lowerer.lower_expr(&HirExpr::Literal(izel_parser::ast::Literal::Float(1.5)));
        let _ = lowerer.lower_expr(&HirExpr::Literal(izel_parser::ast::Literal::Bool(true)));
        let _ = lowerer.lower_expr(&HirExpr::Literal(izel_parser::ast::Literal::Str(
            "s".to_string(),
        )));
        let _ = lowerer.lower_expr(&HirExpr::Literal(izel_parser::ast::Literal::Nil));

        let binary_ops = vec![
            BinaryOp::Add,
            BinaryOp::Sub,
            BinaryOp::Mul,
            BinaryOp::Div,
            BinaryOp::Eq,
            BinaryOp::Ne,
            BinaryOp::Lt,
            BinaryOp::Le,
            BinaryOp::Ge,
            BinaryOp::And,
        ];
        for op in binary_ops {
            let _ = lowerer.lower_expr(&HirExpr::Binary(
                op,
                Box::new(HirExpr::Literal(izel_parser::ast::Literal::Int(1))),
                Box::new(HirExpr::Literal(izel_parser::ast::Literal::Int(2))),
                i32_ty.clone(),
            ));
        }

        let _ = lowerer.lower_expr(&HirExpr::Unary(
            UnaryOp::Neg,
            Box::new(HirExpr::Literal(izel_parser::ast::Literal::Int(2))),
            i32_ty.clone(),
        ));
        let _ = lowerer.lower_expr(&HirExpr::Unary(
            UnaryOp::Not,
            Box::new(HirExpr::Literal(izel_parser::ast::Literal::Bool(false))),
            Type::Prim(PrimType::Bool),
        ));

        lowerer.check_contracts = true;
        let void_call = HirExpr::Call(
            Box::new(HirExpr::Literal(izel_parser::ast::Literal::Int(0))),
            vec![HirExpr::Literal(izel_parser::ast::Literal::Int(3))],
            vec![HirExpr::Literal(izel_parser::ast::Literal::Bool(true))],
            Type::Prim(PrimType::Void),
        );
        let _ = lowerer.lower_expr(&void_call);

        let ret_none = HirExpr::Return(None);
        let _ = lowerer.lower_expr(&ret_none);

        lowerer.body.blocks[lowerer.current_block].terminator = None;
        let given_expr = HirExpr::Given {
            cond: Box::new(HirExpr::Literal(izel_parser::ast::Literal::Bool(true))),
            then_block: HirBlock {
                stmts: vec![HirStmt::Expr(HirExpr::Literal(
                    izel_parser::ast::Literal::Int(1),
                ))],
                expr: Some(Box::new(HirExpr::Literal(izel_parser::ast::Literal::Int(
                    2,
                )))),
                span: Span::dummy(),
            },
            else_expr: Some(Box::new(HirExpr::Literal(izel_parser::ast::Literal::Int(
                3,
            )))),
            ty: i32_ty.clone(),
        };
        let _ = lowerer.lower_expr(&given_expr);

        let while_expr = HirExpr::While {
            cond: Box::new(HirExpr::Literal(izel_parser::ast::Literal::Bool(true))),
            body: HirBlock {
                stmts: vec![],
                expr: None,
                span: Span::dummy(),
            },
        };
        let _ = lowerer.lower_expr(&while_expr);

        let mut found_void_call = false;
        for node in lowerer.body.blocks.node_indices() {
            for inst in &lowerer.body.blocks[node].instructions {
                if let Instruction::Call(None, callee, _) = inst {
                    if callee == "unknown" {
                        found_void_call = true;
                    }
                }
            }
        }
        assert!(
            found_void_call,
            "void calls with non-ident callee should be lowered"
        );
    }

    #[test]
    fn test_substitute_result_expr_covers_unary_and_call_paths() {
        let lowerer = MirLowerer::new();
        let i32_ty = Type::Prim(PrimType::I32);

        let source = HirExpr::Call(
            Box::new(HirExpr::Unary(
                UnaryOp::Neg,
                Box::new(HirExpr::Ident(
                    "result".to_string(),
                    DefId(1),
                    i32_ty.clone(),
                    Span::dummy(),
                )),
                i32_ty.clone(),
            )),
            vec![HirExpr::Ident(
                "result".to_string(),
                DefId(2),
                i32_ty.clone(),
                Span::dummy(),
            )],
            vec![HirExpr::Ident(
                "result".to_string(),
                DefId(3),
                i32_ty.clone(),
                Span::dummy(),
            )],
            i32_ty.clone(),
        );

        let substituted = lowerer.substitute_result_expr(
            &source,
            &HirExpr::Literal(izel_parser::ast::Literal::Int(9)),
        );

        assert!(matches!(
            substituted,
            HirExpr::Call(ref callee, ref args, ref requires, _)
                if matches!(callee.as_ref(), HirExpr::Unary(_, _, _))
                    && args.len() == 1
                    && requires.len() == 1
        ));
    }
}
