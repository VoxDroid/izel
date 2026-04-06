use crate::*;
use izel_parser::ast;
use izel_typeck::type_system::Type;

pub struct HirLowerer<'a> {
    pub resolver: &'a izel_resolve::Resolver,
    pub def_types: &'a rustc_hash::FxHashMap<izel_resolve::DefId, Type>,
}

impl<'a> HirLowerer<'a> {
    pub fn new(
        resolver: &'a izel_resolve::Resolver,
        def_types: &'a rustc_hash::FxHashMap<izel_resolve::DefId, Type>,
    ) -> Self {
        Self {
            resolver,
            def_types,
        }
    }

    fn get_def_id(&self, span: Span) -> DefId {
        self.resolver
            .def_ids
            .read()
            .unwrap()
            .get(&span)
            .cloned()
            .unwrap_or(DefId(0))
    }

    fn get_type(&self, def_id: DefId) -> Type {
        self.def_types.get(&def_id).cloned().unwrap_or(Type::Error)
    }

    pub fn lower_module(&self, module: &ast::Module) -> HirModule {
        let mut items = Vec::new();
        for item in &module.items {
            self.lower_item_to_vec(item, &mut items);
        }
        HirModule { items }
    }

    fn lower_item_to_vec(&self, item: &ast::Item, items: &mut Vec<HirItem>) {
        match item {
            ast::Item::Forge(f) => items.push(HirItem::Forge(Box::new(self.lower_forge(f)))),
            ast::Item::Shape(s) => items.push(HirItem::Shape(self.lower_shape(s))),
            ast::Item::Scroll(s) => items.push(HirItem::Scroll(self.lower_scroll(s))),
            ast::Item::Echo(e) => items.push(HirItem::Echo(self.lower_echo(e))),
            ast::Item::Dual(d) => {
                for inner in &d.items {
                    self.lower_item_to_vec(inner, items);
                }
            }
            ast::Item::Ward(w) => items.push(HirItem::Ward(self.lower_ward(w))),
            ast::Item::Draw(d) => items.push(HirItem::Draw(self.lower_draw(d))),
            _ => {}
        }
    }

    fn lower_shape(&self, shape: &ast::Shape) -> HirShape {
        HirShape {
            name: shape.name.clone(),
            def_id: self.get_def_id(shape.span),
            span: shape.span,
        }
    }

    fn lower_scroll(&self, scroll: &ast::Scroll) -> HirScroll {
        HirScroll {
            name: scroll.name.clone(),
            def_id: self.get_def_id(scroll.span),
            span: scroll.span,
        }
    }

    fn lower_echo(&self, echo: &ast::Echo) -> HirEcho {
        HirEcho {
            body: self.lower_block(&echo.body),
            span: echo.span,
        }
    }

    fn lower_ward(&self, ward: &ast::Ward) -> HirWard {
        let mut items = Vec::new();
        for item in &ward.items {
            self.lower_item_to_vec(item, &mut items);
        }
        HirWard {
            name: ward.name.clone(),
            items,
            span: ward.span,
        }
    }

    fn lower_draw(&self, draw: &ast::Draw) -> HirDraw {
        HirDraw {
            path: draw.path.clone(),
            def_id: None,
            is_wildcard: draw.is_wildcard,
            span: draw.span,
        }
    }

    fn lower_forge(&self, forge: &ast::Forge) -> HirForge {
        let forge_def_id = self.get_def_id(forge.name_span);
        let full_ty = self.get_type(forge_def_id);
        let ret_type = match full_ty {
            Type::Function { ret, .. } => *ret,
            _ => Type::Error,
        };
        HirForge {
            name: forge.name.clone(),
            name_span: forge.name_span,
            def_id: forge_def_id,
            params: forge.params.iter().map(|p| self.lower_param(p)).collect(),
            ret_type,
            attributes: forge.attributes.clone(),
            body: forge.body.as_ref().map(|b| self.lower_block(b)),
            requires: forge.requires.iter().map(|e| self.lower_expr(e)).collect(),
            ensures: forge.ensures.iter().map(|e| self.lower_expr(e)).collect(),
            span: forge.span,
        }
    }

    fn lower_param(&self, param: &ast::Param) -> HirParam {
        let def_id = self.get_def_id(param.span);
        let ty = self.get_type(def_id);
        HirParam {
            name: param.name.clone(),
            def_id,
            ty,
            default_value: param.default_value.as_ref().map(|e| self.lower_expr(e)),
            is_variadic: param.is_variadic,
            span: param.span,
        }
    }

    fn lower_block(&self, block: &ast::Block) -> HirBlock {
        HirBlock {
            stmts: block.stmts.iter().map(|s| self.lower_stmt(s)).collect(),
            expr: block.expr.as_ref().map(|e| Box::new(self.lower_expr(e))),
            span: block.span,
        }
    }

    fn lower_stmt(&self, stmt: &ast::Stmt) -> HirStmt {
        match stmt {
            ast::Stmt::Let {
                pat, init, span, ..
            } => {
                let (name, name_span) = match pat {
                    ast::Pattern::Ident(name, _, span) => (name.clone(), *span),
                    _ => ("_hir_pattern_unsupported".to_string(), *span),
                };
                let def_id = self.get_def_id(name_span);

                let ty = self.get_type(def_id);
                eprintln!("HIR Let: name={}, def_id={:?}, ty={:?}", name, def_id, ty);

                HirStmt::Let {
                    name,
                    def_id,
                    ty,
                    init: init.as_ref().map(|e| self.lower_expr(e)),
                    span: *span,
                }
            }
            ast::Stmt::Expr(e) => HirStmt::Expr(self.lower_expr(e)),
        }
    }

    fn lower_expr(&self, expr: &ast::Expr) -> HirExpr {
        match expr {
            ast::Expr::Literal(lit) => HirExpr::Literal(lit.clone()),
            ast::Expr::Ident(name, span) => {
                let def_id = self.get_def_id(*span);
                HirExpr::Ident(name.clone(), def_id, self.get_type(def_id), *span)
            }
            ast::Expr::Binary(op, left, right) => HirExpr::Binary(
                op.clone(),
                Box::new(self.lower_expr(left)),
                Box::new(self.lower_expr(right)),
                Type::Error,
            ),
            ast::Expr::Unary(op, inner) => {
                HirExpr::Unary(op.clone(), Box::new(self.lower_expr(inner)), Type::Error)
            }
            ast::Expr::Call(callee, args) => {
                let callee_hir = self.lower_expr(callee);
                let mut ret_type = Type::Error;
                if let HirExpr::Ident(_, def_id, _, _) = &callee_hir {
                    if let Type::Function { ret, .. } = self.get_type(*def_id) {
                        ret_type = (*ret).clone();
                    }
                }
                HirExpr::Call(
                    Box::new(callee_hir),
                    args.iter().map(|a| self.lower_expr(&a.value)).collect(),
                    vec![],
                    ret_type,
                )
            }
            ast::Expr::Member(inner, name, span) => {
                let def_id = self.get_def_id(*span);
                HirExpr::Call(
                    Box::new(HirExpr::Ident(
                        name.clone(),
                        def_id,
                        self.get_type(def_id),
                        *span,
                    )),
                    vec![self.lower_expr(inner)],
                    vec![],
                    Type::Error, // Member access return type handled by typeck later or can be looked up
                )
            }
            ast::Expr::Given {
                cond,
                then_block,
                else_expr,
            } => HirExpr::Given {
                cond: Box::new(self.lower_expr(cond)),
                then_block: self.lower_block(then_block),
                else_expr: else_expr.as_ref().map(|e| Box::new(self.lower_expr(e))),
                ty: Type::Error,
            },
            ast::Expr::While { cond, body } => HirExpr::While {
                cond: Box::new(self.lower_expr(cond)),
                body: self.lower_block(body),
            },
            ast::Expr::Return(e) => HirExpr::Return(Some(Box::new(self.lower_expr(e)))),
            ast::Expr::Zone { name, body } => HirExpr::Zone {
                name: name.clone(),
                body: self.lower_block(body),
                ty: Type::Error,
            },
            ast::Expr::StructLiteral { .. } => HirExpr::Literal(ast::Literal::Nil), // Stub
            _ => HirExpr::Literal(ast::Literal::Nil),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use izel_resolve::Resolver;
    use izel_span::{BytePos, SourceId};
    use izel_typeck::type_system::{EffectSet, PrimType};
    use rustc_hash::FxHashMap;

    fn sp(n: u32) -> Span {
        Span::new(BytePos(n), BytePos(n + 1), SourceId(0))
    }

    fn ast_prim(name: &str) -> ast::Type {
        ast::Type::Prim(name.to_string())
    }

    #[test]
    fn lower_module_flattens_dual_and_ignores_unsupported_items() {
        let resolver = Resolver::new(None);
        let def_types = FxHashMap::default();

        let shape_span = sp(10);
        let scroll_span = sp(20);
        let forge_name_span = sp(30);

        {
            let mut ids = resolver.def_ids.write().expect("def_ids lock");
            ids.insert(shape_span, DefId(1));
            ids.insert(scroll_span, DefId(2));
            ids.insert(forge_name_span, DefId(3));
        }

        let module = ast::Module {
            items: vec![
                ast::Item::Shape(ast::Shape {
                    name: "S".to_string(),
                    visibility: ast::Visibility::Open,
                    generic_params: vec![],
                    fields: vec![],
                    attributes: vec![],
                    invariants: vec![],
                    span: shape_span,
                }),
                ast::Item::Dual(ast::Dual {
                    name: "D".to_string(),
                    visibility: ast::Visibility::Open,
                    generic_params: vec![],
                    items: vec![
                        ast::Item::Scroll(ast::Scroll {
                            name: "E".to_string(),
                            visibility: ast::Visibility::Open,
                            variants: vec![],
                            attributes: vec![],
                            span: scroll_span,
                        }),
                        ast::Item::Alias(ast::Alias {
                            name: "Ignored".to_string(),
                            visibility: ast::Visibility::Open,
                            ty: ast_prim("i32"),
                            attributes: vec![],
                            span: sp(21),
                        }),
                    ],
                    attributes: vec![],
                    span: sp(22),
                }),
                ast::Item::Ward(ast::Ward {
                    name: "W".to_string(),
                    visibility: ast::Visibility::Open,
                    items: vec![ast::Item::Forge(ast::Forge {
                        name: "inside".to_string(),
                        name_span: forge_name_span,
                        visibility: ast::Visibility::Open,
                        is_flow: false,
                        generic_params: vec![],
                        params: vec![],
                        ret_type: ast_prim("i32"),
                        effects: vec![],
                        attributes: vec![],
                        requires: vec![],
                        ensures: vec![],
                        body: Some(ast::Block {
                            stmts: vec![],
                            expr: None,
                            span: sp(31),
                        }),
                        span: sp(32),
                    })],
                    attributes: vec![],
                    span: sp(23),
                }),
                ast::Item::Draw(ast::Draw {
                    path: vec!["std".to_string(), "io".to_string()],
                    is_wildcard: true,
                    span: sp(24),
                }),
                ast::Item::Echo(ast::Echo {
                    body: ast::Block {
                        stmts: vec![],
                        expr: Some(Box::new(ast::Expr::Literal(ast::Literal::Int(1)))),
                        span: sp(25),
                    },
                    attributes: vec![],
                    span: sp(26),
                }),
                ast::Item::Alias(ast::Alias {
                    name: "TopIgnored".to_string(),
                    visibility: ast::Visibility::Open,
                    ty: ast_prim("i32"),
                    attributes: vec![],
                    span: sp(27),
                }),
            ],
        };

        let lowerer = HirLowerer::new(&resolver, &def_types);
        let hir = lowerer.lower_module(&module);

        assert_eq!(hir.items.len(), 5);
        assert!(matches!(hir.items[0], HirItem::Shape(_)));
        assert!(matches!(hir.items[1], HirItem::Scroll(_)));
        assert!(matches!(hir.items[2], HirItem::Ward(_)));
        assert!(matches!(hir.items[3], HirItem::Draw(_)));
        assert!(matches!(hir.items[4], HirItem::Echo(_)));

        assert!(matches!(
            hir.items[2],
            HirItem::Ward(ref ward)
                if ward.items.len() == 1 && matches!(ward.items[0], HirItem::Forge(_))
        ));
    }

    #[test]
    fn lower_forge_reads_type_info_for_return_param_and_calls() {
        let resolver = Resolver::new(None);

        let forge_span = sp(100);
        let param_span = sp(101);
        let local_span = sp(102);
        let callee_span = sp(103);

        {
            let mut ids = resolver.def_ids.write().expect("def_ids lock");
            ids.insert(forge_span, DefId(10));
            ids.insert(param_span, DefId(11));
            ids.insert(local_span, DefId(12));
            ids.insert(callee_span, DefId(13));
        }

        let mut def_types = FxHashMap::default();
        def_types.insert(
            DefId(10),
            Type::Function {
                params: vec![Type::Prim(PrimType::I64)],
                ret: Box::new(Type::Prim(PrimType::Bool)),
                effects: EffectSet::Concrete(vec![]),
            },
        );
        def_types.insert(DefId(11), Type::Prim(PrimType::I64));
        def_types.insert(DefId(12), Type::Prim(PrimType::I32));
        def_types.insert(
            DefId(13),
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Prim(PrimType::I32)),
                effects: EffectSet::Concrete(vec![]),
            },
        );

        let forge = ast::Forge {
            name: "f".to_string(),
            name_span: forge_span,
            visibility: ast::Visibility::Open,
            is_flow: false,
            generic_params: vec![],
            params: vec![ast::Param {
                name: "p".to_string(),
                ty: ast_prim("i64"),
                default_value: Some(ast::Expr::Literal(ast::Literal::Int(7))),
                is_variadic: false,
                span: param_span,
            }],
            ret_type: ast_prim("bool"),
            effects: vec![],
            attributes: vec![],
            requires: vec![ast::Expr::Call(
                Box::new(ast::Expr::Ident("callee".to_string(), callee_span)),
                vec![],
            )],
            ensures: vec![ast::Expr::Binary(
                ast::BinaryOp::Eq,
                Box::new(ast::Expr::Ident("x".to_string(), local_span)),
                Box::new(ast::Expr::Literal(ast::Literal::Int(1))),
            )],
            body: Some(ast::Block {
                stmts: vec![ast::Stmt::Let {
                    pat: ast::Pattern::Ident("x".to_string(), false, local_span),
                    ty: None,
                    init: Some(ast::Expr::Literal(ast::Literal::Int(3))),
                    span: sp(104),
                }],
                expr: Some(Box::new(ast::Expr::Ident("x".to_string(), local_span))),
                span: sp(105),
            }),
            span: sp(106),
        };

        let module = ast::Module {
            items: vec![ast::Item::Forge(forge)],
        };
        let lowerer = HirLowerer::new(&resolver, &def_types);
        let hir = lowerer.lower_module(&module);

        assert!(matches!(hir.items[0], HirItem::Forge(_)));
        let mut forge_opt = None;
        if let HirItem::Forge(f) = &hir.items[0] {
            forge_opt = Some(f);
        }
        let forge = forge_opt.expect("expected forge item");

        assert_eq!(forge.def_id, DefId(10));
        assert_eq!(forge.ret_type, Type::Prim(PrimType::Bool));
        assert_eq!(forge.params[0].def_id, DefId(11));
        assert_eq!(forge.params[0].ty, Type::Prim(PrimType::I64));
        assert!(forge.params[0].default_value.is_some());

        assert!(matches!(
            &forge.requires[0],
            HirExpr::Call(_, _, _, ty) if *ty == Type::Prim(PrimType::I32)
        ));

        let HirBlock { stmts, expr, .. } = forge.body.as_ref().expect("forge body");
        assert_eq!(stmts.len(), 1);
        assert!(expr.is_some());
    }

    #[test]
    fn lower_stmt_and_expr_cover_special_and_fallback_paths() {
        let resolver = Resolver::new(None);
        let mut def_types = FxHashMap::default();

        let ident_span = sp(200);
        let member_span = sp(201);
        let callee_span = sp(202);

        {
            let mut ids = resolver.def_ids.write().expect("def_ids lock");
            ids.insert(ident_span, DefId(20));
            ids.insert(member_span, DefId(21));
            ids.insert(callee_span, DefId(22));
        }

        def_types.insert(DefId(20), Type::Prim(PrimType::I32));
        def_types.insert(DefId(21), Type::Prim(PrimType::Bool));
        def_types.insert(
            DefId(22),
            Type::Function {
                params: vec![],
                ret: Box::new(Type::Prim(PrimType::I16)),
                effects: EffectSet::Concrete(vec![]),
            },
        );

        let lowerer = HirLowerer::new(&resolver, &def_types);

        let unsupported_let = ast::Stmt::Let {
            pat: ast::Pattern::Wildcard,
            ty: None,
            init: Some(ast::Expr::Literal(ast::Literal::Int(1))),
            span: sp(203),
        };
        let lowered_let = lowerer.lower_stmt(&unsupported_let);
        assert!(matches!(
            lowered_let,
            HirStmt::Let {
                ref name,
                def_id,
                ref ty,
                ..
            } if name == "_hir_pattern_unsupported" && def_id == DefId(0) && *ty == Type::Error
        ));

        let expr_stmt = ast::Stmt::Expr(ast::Expr::Unary(
            ast::UnaryOp::Neg,
            Box::new(ast::Expr::Literal(ast::Literal::Int(2))),
        ));
        assert!(matches!(lowerer.lower_stmt(&expr_stmt), HirStmt::Expr(_)));

        let ident = ast::Expr::Ident("x".to_string(), ident_span);
        let lowered_ident = lowerer.lower_expr(&ident);
        assert!(matches!(
            lowered_ident,
            HirExpr::Ident(ref name, def_id, ref ty, _)
                if name == "x" && def_id == DefId(20) && *ty == Type::Prim(PrimType::I32)
        ));

        let call_ident = ast::Expr::Call(
            Box::new(ast::Expr::Ident("callee".to_string(), callee_span)),
            vec![ast::Arg {
                label: None,
                value: ast::Expr::Literal(ast::Literal::Int(9)),
                span: sp(204),
            }],
        );
        let lowered_call_ident = lowerer.lower_expr(&call_ident);
        assert!(matches!(
            lowered_call_ident,
            HirExpr::Call(_, ref args, _, ref ty)
                if args.len() == 1 && *ty == Type::Prim(PrimType::I16)
        ));

        let call_non_ident =
            ast::Expr::Call(Box::new(ast::Expr::Literal(ast::Literal::Int(1))), vec![]);
        let lowered_call_non_ident = lowerer.lower_expr(&call_non_ident);
        assert!(matches!(
            lowered_call_non_ident,
            HirExpr::Call(_, _, _, ref ty) if *ty == Type::Error
        ));

        let member = ast::Expr::Member(
            Box::new(ast::Expr::Ident("obj".to_string(), ident_span)),
            "field".to_string(),
            member_span,
        );
        let lowered_member = lowerer.lower_expr(&member);
        assert!(matches!(
            lowered_member,
            HirExpr::Call(ref callee, ref args, _, ref ty)
                if args.len() == 1
                    && *ty == Type::Error
                    && matches!(
                        callee.as_ref(),
                        HirExpr::Ident(ref name, def_id, ref field_ty, _)
                            if name == "field"
                                && *def_id == DefId(21)
                                && *field_ty == Type::Prim(PrimType::Bool)
                    )
        ));

        let given = ast::Expr::Given {
            cond: Box::new(ast::Expr::Literal(ast::Literal::Bool(true))),
            then_block: ast::Block {
                stmts: vec![],
                expr: Some(Box::new(ast::Expr::Literal(ast::Literal::Int(1)))),
                span: sp(205),
            },
            else_expr: Some(Box::new(ast::Expr::Literal(ast::Literal::Int(2)))),
        };
        assert!(matches!(lowerer.lower_expr(&given), HirExpr::Given { .. }));

        let while_expr = ast::Expr::While {
            cond: Box::new(ast::Expr::Literal(ast::Literal::Bool(true))),
            body: ast::Block {
                stmts: vec![],
                expr: None,
                span: sp(206),
            },
        };
        assert!(matches!(
            lowerer.lower_expr(&while_expr),
            HirExpr::While { .. }
        ));

        let ret_expr = ast::Expr::Return(Box::new(ast::Expr::Literal(ast::Literal::Int(0))));
        assert!(matches!(
            lowerer.lower_expr(&ret_expr),
            HirExpr::Return(Some(_))
        ));

        let zone_expr = ast::Expr::Zone {
            name: "arena".to_string(),
            body: ast::Block {
                stmts: vec![],
                expr: None,
                span: sp(207),
            },
        };
        assert!(matches!(
            lowerer.lower_expr(&zone_expr),
            HirExpr::Zone { .. }
        ));

        let struct_lit = ast::Expr::StructLiteral {
            path: ast_prim("Point"),
            fields: vec![("x".to_string(), ast::Expr::Literal(ast::Literal::Int(1)))],
        };
        assert!(matches!(
            lowerer.lower_expr(&struct_lit),
            HirExpr::Literal(ast::Literal::Nil)
        ));

        let fallback = ast::Expr::Path(vec!["A".to_string()], vec![]);
        assert!(matches!(
            lowerer.lower_expr(&fallback),
            HirExpr::Literal(ast::Literal::Nil)
        ));

        let binary = ast::Expr::Binary(
            ast::BinaryOp::Add,
            Box::new(ast::Expr::Literal(ast::Literal::Int(1))),
            Box::new(ast::Expr::Literal(ast::Literal::Int(2))),
        );
        assert!(matches!(lowerer.lower_expr(&binary), HirExpr::Binary(..)));
    }
}
