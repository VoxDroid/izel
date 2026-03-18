use izel_parser::cst::{SyntaxNode, SyntaxElement, NodeKind};
use izel_parser::ast;
use izel_lexer::TokenKind;
use izel_span::Span;

pub mod elaboration;

pub struct Lowerer<'a> {
    source: &'a str,
}

impl<'a> Lowerer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source }
    }

    pub fn lower_module(&self, node: &SyntaxNode) -> ast::Module {
        let mut items = Vec::new();
        for child in &node.children {
            if let SyntaxElement::Node(child_node) = child {
                items.extend(self.lower_item(child_node));
            }
        }
        ast::Module { items }
    }

    pub fn lower_item(&self, node: &SyntaxNode) -> Vec<ast::Item> {
        let mut results = Vec::new();
        match node.kind {
            NodeKind::ForgeDecl => results.push(ast::Item::Forge(self.lower_forge(node))),
            NodeKind::ShapeDecl => results.push(ast::Item::Shape(self.lower_shape(node))),
            NodeKind::ScrollDecl => results.push(ast::Item::Scroll(self.lower_scroll(node))),
            NodeKind::WeaveDecl => results.push(ast::Item::Weave(self.lower_weave(node))),
            NodeKind::WardDecl => results.push(ast::Item::Ward(self.lower_ward(node))),
            NodeKind::DualDecl => {
                if let Some((dual, generated_test)) = self.lower_dual(node) {
                    results.push(ast::Item::Dual(dual));
                    if let Some(test) = generated_test {
                        results.push(test);
                    }
                }
            }
            NodeKind::DrawDecl => results.push(ast::Item::Draw(self.lower_draw(node))),
            NodeKind::ImplBlock => results.push(ast::Item::Impl(self.lower_impl(node))),
            NodeKind::TypeAlias => results.push(ast::Item::Alias(self.lower_alias(node))),
            _ => {}
        }
        results
    }

    fn lower_forge(&self, node: &SyntaxNode) -> ast::Forge {
        let mut name = String::new();
        let mut params = Vec::new();
        let mut ret_type = ast::Type::Prim("void".to_string());
        let mut body = None;
        let mut generic_params = Vec::new();

        let mut effects = Vec::new();
        let mut attributes = Vec::new();

        for child in &node.children {
            match child {
                SyntaxElement::Node(n) if n.kind == NodeKind::Attributes => {
                    attributes = self.lower_attributes(n);
                }
                SyntaxElement::Token(token) if self.is_naming_ident(token.kind) => {
                    name = self.source[token.span.lo.0 as usize..token.span.hi.0 as usize].to_string();
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::GenericParams => {
                    generic_params = self.lower_generic_params(n);
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::ParamPart => {
                    // Simple param parsing for now
                    params.push(self.lower_param(n));
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::Effect => {
                    effects.push(self.lower_effect(n));
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::Block => {
                    body = Some(self.lower_block(n));
                }
                SyntaxElement::Node(n) => {
                    // Could be the return type if it's not a block/params/effect/generic
                    if !matches!(n.kind, NodeKind::ParamPart | NodeKind::Block | NodeKind::GenericParams | NodeKind::Effect) {
                        ret_type = self.lower_type(n);
                    }
                }
                _ => {}
            }
        }

        let mut requires = Vec::new();
        let mut ensures = Vec::new();
        for attr in &attributes {
            if attr.name == "requires" {
                requires.extend(attr.args.clone());
            } else if attr.name == "ensures" {
                ensures.extend(attr.args.clone());
            }
        }

        ast::Forge {
            name,
            generic_params,
            params,
            ret_type,
            effects,
            attributes,
            requires,
            ensures,
            body,
            span: node.span(),
        }
    }

    fn lower_shape(&self, node: &SyntaxNode) -> ast::Shape {
        let mut name = String::new();
        let mut fields = Vec::new();
        let mut generic_params = Vec::new();
        let mut attributes = Vec::new();
        let mut invariants = Vec::new();

        for child in &node.children {
            match child {
                SyntaxElement::Node(n) if n.kind == NodeKind::Attributes => {
                    attributes = self.lower_attributes(n);
                }
                SyntaxElement::Token(token) if self.is_naming_ident(token.kind) => {
                    name = self.source[token.span.lo.0 as usize..token.span.hi.0 as usize].to_string();
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::GenericParams => {
                    generic_params = self.lower_generic_params(n);
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::Field => {
                    fields.push(self.lower_field(n));
                }
                _ => {}
            }
        }

        // Extract #[invariant] attributes into invariants
        let mut non_invariant_attrs = Vec::new();
        for attr in attributes {
            if attr.name == "invariant" {
                invariants.extend(attr.args);
            } else {
                non_invariant_attrs.push(attr);
            }
        }

        ast::Shape {
            name,
            generic_params,
            fields,
            attributes: non_invariant_attrs,
            invariants,
            span: node.span(),
        }
    }

    fn lower_field(&self, node: &SyntaxNode) -> ast::Field {
        let mut name = String::new();
        let mut ty = ast::Type::Error;

        for child in &node.children {
            match child {
                SyntaxElement::Token(token) if self.is_naming_ident(token.kind) => {
                    name = self.source[token.span.lo.0 as usize..token.span.hi.0 as usize].to_string();
                }
                SyntaxElement::Node(n) => {
                    ty = self.lower_type(n);
                }
                _ => {}
            }
        }

        ast::Field { name, ty, span: node.span() }
    }

    fn lower_dual(&self, node: &SyntaxNode) -> Option<(ast::Dual, Option<ast::Item>)> {
        let mut name = String::new();
        let mut generic_params = Vec::new();
        let mut items = Vec::new();
        let mut attributes = Vec::new();

        for child in &node.children {
            match child {
                SyntaxElement::Token(token) if self.is_naming_ident(token.kind) => {
                    name = self.source[token.span.lo.0 as usize..token.span.hi.0 as usize].to_string();
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::Attributes => {
                    attributes = self.lower_attributes(n);
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::GenericParams => {
                    generic_params = self.lower_generic_params(n);
                }
                SyntaxElement::Node(n) => {
                    // Try to lower any enclosed item like a forge declaration
                    items.extend(self.lower_item(n));
                }
                _ => {}
            }
        }

        let mut dual = ast::Dual {
            name,
            generic_params,
            items,
            attributes,
            span: node.span(),
        };

        let generated_test = elaboration::elaborate_dual(&mut dual);

        Some((dual, generated_test))
    }

    fn lower_generic_params(&self, node: &SyntaxNode) -> Vec<ast::GenericParam> {
        let mut params = Vec::new();
        for child in &node.children {
            if let SyntaxElement::Node(n) = child {
                if n.kind == NodeKind::GenericParam {
                    params.push(self.lower_generic_param(n));
                }
            }
        }
        params
    }

    fn lower_generic_param(&self, node: &SyntaxNode) -> ast::GenericParam {
        let mut name = String::new();
        let mut bounds = Vec::new();
        let mut in_bounds = false;
        
        for child in &node.children {
            if let SyntaxElement::Token(t) = child {
                match t.kind {
                    TokenKind::Ident => {
                        if in_bounds {
                            bounds.push(self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string());
                        } else {
                            name = self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                        }
                    }
                    TokenKind::Colon => {
                        in_bounds = true;
                    }
                    _ => {}
                }
            }
        }
        
        ast::GenericParam {
            name,
            bounds,
            span: node.span(),
        }
    }

    fn lower_param(&self, node: &SyntaxNode) -> ast::Param {
        let mut name = String::new();
        let mut ty = ast::Type::Error;

        for child in &node.children {
            match child {
                SyntaxElement::Token(token) if self.is_naming_ident(token.kind) => {
                    name = self.source[token.span.lo.0 as usize..token.span.hi.0 as usize].to_string();
                }
                SyntaxElement::Node(n) => {
                    ty = self.lower_type(n);
                }
                _ => {}
            }
        }
        ast::Param { name, ty, span: node.span() }
    }

    fn lower_block(&self, node: &SyntaxNode) -> ast::Block {
        let mut stmts = Vec::new();
        let mut last_expr = None;

        for (i, child) in node.children.iter().enumerate() {
            if let SyntaxElement::Node(n) = child {
                if i == node.children.len() - 1 && n.kind != NodeKind::LetStmt {
                     // Last node might be a trailing expression if it's not a let
                     last_expr = Some(Box::new(self.lower_expr(n)));
                } else {
                     stmts.push(self.lower_stmt(n));
                }
            }
        }
        ast::Block { stmts, expr: last_expr, span: node.span() }
    }

    fn lower_stmt(&self, node: &SyntaxNode) -> ast::Stmt {
        match node.kind {
            NodeKind::LetStmt => {
                let mut name = String::new();
                let mut ty = None;
                let mut init = None;
                let mut found_eq = false;
                for child in &node.children {
                    match child {
                        SyntaxElement::Token(t) => {
                             if t.kind == TokenKind::Ident {
                                  name = self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                             } else if t.kind == TokenKind::Equal {
                                  found_eq = true;
                             }
                        }
                        SyntaxElement::Node(n) => {
                             if found_eq {
                                  init = Some(self.lower_expr(n));
                             } else {
                                  ty = Some(self.lower_type(n));
                             }
                        }
                    }
                }
                ast::Stmt::Let { name, ty, init, span: node.span() }
            }
            NodeKind::ExprStmt => {
                for child in &node.children {
                    if let SyntaxElement::Node(n) = child {
                        return ast::Stmt::Expr(self.lower_expr(n));
                    }
                }
                ast::Stmt::Expr(ast::Expr::Literal(ast::Literal::Nil))
            }
            _ => ast::Stmt::Expr(self.lower_expr(node)),
        }
    }

    pub fn lower_type(&self, node: &SyntaxNode) -> ast::Type {
        match node.kind {
            NodeKind::OptionalType => {
                let inner = self.lower_element_type(&node.children[1]);
                ast::Type::Optional(Box::new(inner))
            }
            NodeKind::PointerType => {
                 let mut is_mut = false;
                 let mut ty = ast::Type::Error;
                 for child in &node.children {
                      match child {
                           SyntaxElement::Token(t) if t.kind == TokenKind::Tilde => is_mut = true,
                           SyntaxElement::Node(n) => ty = self.lower_type(n),
                           _ => {}
                      }
                 }
                 ast::Type::Pointer(Box::new(ty), is_mut)
            }
            NodeKind::CallExpr | NodeKind::Ident | NodeKind::PathExpr | NodeKind::Type => {
                 // Check if it's Witness<P> desugared or similar
                 let mut name = String::new();
                 let mut args = Vec::new();
                 
                 for child in &node.children {
                     match child {
                         SyntaxElement::Token(t) if t.kind == TokenKind::Ident => {
                             name = self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                         }
                         SyntaxElement::Node(n) if n.kind == NodeKind::GenericArgs => {
                             args = self.lower_generic_args(n);
                         }
                         SyntaxElement::Node(n) if n.kind == NodeKind::Ident || n.kind == NodeKind::PathExpr => {
                             // Recurse for nested structures
                             let ty = self.lower_type(n);
                             if let ast::Type::Prim(s) = ty { name = s; }
                         }
                         _ => {}
                     }
                 }
                 
                 if name == "Witness" && !args.is_empty() {
                     return ast::Type::Witness(Box::new(args[0].clone()));
                 }
                 
                 if node.kind == NodeKind::PathExpr {
                      return self.lower_type_path(node);
                 }

                 // If we have generic args, preserve them as a Path type
                 // so the typeck layer can resolve parameterized types (e.g., NonZero<i32>)
                 if !args.is_empty() && !name.is_empty() {
                      return ast::Type::Path(vec![name], args);
                 }
                 
                 ast::Type::Prim(if name.is_empty() { "Error".to_string() } else { name })
            }
            NodeKind::UnaryExpr => {
                 let mut is_cascade = false;
                 let mut inner = ast::Type::Error;
                 for child in &node.children {
                      match child {
                           SyntaxElement::Token(t) if t.kind == TokenKind::Bang => is_cascade = true,
                           SyntaxElement::Node(n) => inner = self.lower_type(n),
                           _ => {}
                      }
                 }
                 if is_cascade {
                      ast::Type::Cascade(Box::new(inner))
                 } else {
                      ast::Type::Error
                 }
            }
            _ => ast::Type::Error,
        }
    }

    fn lower_type_path(&self, node: &SyntaxNode) -> ast::Type {
        let mut path = Vec::new();
        let mut generic_args = Vec::new();
        for child in &node.children {
            match child {
                SyntaxElement::Token(t) if t.kind == TokenKind::Ident => {
                    path.push(self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string());
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::GenericArgs => {
                    generic_args = self.lower_generic_args(n);
                }
                _ => {}
            }
        }
        ast::Type::Path(path, generic_args)
    }

    fn lower_element_type(&self, element: &SyntaxElement) -> ast::Type {
        match element {
            SyntaxElement::Node(n) => self.lower_type(n),
            _ => ast::Type::Error,
        }
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
                        TokenKind::Str { .. } | TokenKind::InterpolatedStr { .. } => {
                             let text = &self.source[token.span.lo.0 as usize..token.span.hi.0 as usize];
                             return ast::Expr::Literal(ast::Literal::Str(text.to_string()));
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
                 for child in &node.children {
                      if let SyntaxElement::Token(token) = child {
                           if self.is_naming_ident(token.kind) {
                                let text = &self.source[token.span.lo.0 as usize..token.span.hi.0 as usize].to_string();
                                return ast::Expr::Ident(text.clone(), token.span);
                           }
                      }
                 }
                 ast::Expr::Literal(ast::Literal::Nil)
            }
            NodeKind::BinaryExpr => {
                 let lhs = self.lower_element(&node.children[0]);
                 let op_tok = &node.children[1];
                 let rhs = self.lower_element(&node.children[2]);
                 
                 let op = match op_tok {
                      SyntaxElement::Token(t) => match t.kind {
                           TokenKind::Plus => ast::BinaryOp::Add,
                           TokenKind::Minus => ast::BinaryOp::Sub,
                           TokenKind::Star => ast::BinaryOp::Mul,
                           TokenKind::Slash => ast::BinaryOp::Div,
                           TokenKind::EqEq => ast::BinaryOp::Eq,
                           TokenKind::NotEq => ast::BinaryOp::Ne,
                             TokenKind::Pipe => return self.desugar_pipeline(lhs, rhs),
                            TokenKind::QuestionQuestion => return self.desugar_coalesce(lhs, rhs),
                            TokenKind::And => ast::BinaryOp::And,
                            TokenKind::Or => ast::BinaryOp::Or,
                           _ => ast::BinaryOp::Add,
                      }
                      _ => ast::BinaryOp::Add,
                 };
                 ast::Expr::Binary(op, Box::new(lhs), Box::new(rhs))
            }
            NodeKind::CascadeExpr => {
                let mut expr = None;
                for child in &node.children {
                    if let SyntaxElement::Node(n) = child {
                        expr = Some(self.lower_expr(n));
                        break;
                    }
                }
                let mut context = None;
                if node.children.len() > 3 {
                    let mut found_or = false;
                    for child in node.children.iter().skip(1) {
                         if let SyntaxElement::Token(t) = child {
                             if t.kind == TokenKind::Or { found_or = true; }
                         } else if let SyntaxElement::Node(n) = child {
                             if found_or {
                                 context = Some(Box::new(self.lower_expr(n)));
                                 break;
                             }
                         }
                    }
                }
                ast::Expr::Cascade {
                    expr: Box::new(expr.unwrap_or(ast::Expr::Literal(ast::Literal::Nil))),
                    context,
                }
            }
            NodeKind::UnaryExpr => {
                let mut op = ast::UnaryOp::Neg;
                let mut expr = None;
                for child in &node.children {
                     match child {
                          SyntaxElement::Token(t) => {
                               op = match t.kind {
                                    TokenKind::Minus => ast::UnaryOp::Neg,
                                    TokenKind::Not => ast::UnaryOp::Not,
                                    TokenKind::Tilde => ast::UnaryOp::BitNot,
                                    TokenKind::Star => ast::UnaryOp::Deref,
                                    TokenKind::Ampersand => ast::UnaryOp::Ref(false),
                                    TokenKind::AmpersandTilde => ast::UnaryOp::Ref(true),
                                    _ => ast::UnaryOp::Neg,
                               };
                          }
                          SyntaxElement::Node(n) => expr = Some(self.lower_expr(n)),
                     }
                }
                ast::Expr::Unary(op, Box::new(expr.unwrap_or(ast::Expr::Literal(ast::Literal::Nil))))
            }
            NodeKind::CallExpr => {
                 let target = self.lower_element(&node.children[0]);
                 let mut args = Vec::new();
                 for i in 1..node.children.len() {
                      if let SyntaxElement::Node(n) = &node.children[i] {
                           args.push(self.lower_expr(n));
                      }
                 }
                 ast::Expr::Call(Box::new(target), args)
            }
            NodeKind::MemberExpr => {
                 let target = self.lower_element(&node.children[0]);
                 let mut name = String::new();
                 let mut is_optional = false;
                 for child in &node.children {
                      match child {
                           SyntaxElement::Token(t) => {
                                if t.kind == TokenKind::Ident {
                                     name = self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                                } else if t.kind == TokenKind::Question {
                                     is_optional = true;
                                }
                           }
                           _ => {}
                      }
                 }
                 if is_optional {
                      return self.desugar_optional_chain(target, name, node.span());
                 }
                 ast::Expr::Member(Box::new(target), name, node.span())
            }
            NodeKind::PathExpr => {
                 let mut path = Vec::new();
                 let mut generic_args = Vec::new();
                 for child in &node.children {
                      match child {
                           SyntaxElement::Token(t) if t.kind == TokenKind::Ident => {
                                path.push(self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string());
                           }
                           SyntaxElement::Node(n) if n.kind == NodeKind::GenericArgs => {
                                generic_args = self.lower_generic_args(n);
                           }
                           _ => {}
                      }
                 }
                 ast::Expr::Path(path, generic_args)
            }
            NodeKind::GivenExpr => {
                let mut cond = None;
                let mut then_block = None;
                let mut else_expr = None;
                for child in &node.children {
                    match child {
                         SyntaxElement::Node(n) if n.kind == NodeKind::Block => then_block = Some(self.lower_block(n)),
                         SyntaxElement::Node(n) if cond.is_none() => cond = Some(self.lower_expr(n)),
                         SyntaxElement::Node(n) => else_expr = Some(Box::new(self.lower_expr(n))),
                         _ => {}
                    }
                }
                ast::Expr::Given {
                    cond: Box::new(cond.unwrap_or(ast::Expr::Literal(ast::Literal::Nil))),
                    then_block: then_block.unwrap_or(ast::Block { stmts: vec![], expr: None, span: node.span() }),
                    else_expr,
                }
            }
            NodeKind::Block => ast::Expr::Block(self.lower_block(node)),
            NodeKind::RawExpr => {
                let mut inner_node = None;
                for child in &node.children {
                    if let SyntaxElement::Node(n) = child {
                         inner_node = Some(n);
                         break;
                    }
                }
                if let Some(n) = inner_node {
                    ast::Expr::Raw(Box::new(self.lower_expr(n)))
                } else {
                    ast::Expr::Literal(ast::Literal::Nil)
                }
            }
            NodeKind::MacroCall => {
                let mut macro_name = String::new();
                for child in &node.children {
                    if let SyntaxElement::Token(t) = child {
                        if t.kind == TokenKind::Ident {
                            macro_name = self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                            break;
                        }
                    }
                }
                
                if macro_name == "here" {
                    // Calculate line number dynamically based on the node's span.
                    let span = node.span();
                    let file_content = &self.source[..span.lo.0 as usize];
                    let line_number = file_content.chars().filter(|&c| c == '\n').count() + 1;
                    
                    let location_string = format!("{}:{}", "main.iz", line_number);
                    ast::Expr::Literal(ast::Literal::Str(location_string))
                } else {
                    ast::Expr::Literal(ast::Literal::Nil)
                }
            }
            NodeKind::StructLiteral => {
                let path = self.lower_element_type(&node.children[0]);
                let mut fields = Vec::new();
                let mut current_field = None;
                let mut state = 0; // 0: looking for field name, 1: looking for colon, 2: looking for expr

                for child in node.children.iter().skip(1) {
                    match child {
                        SyntaxElement::Node(n) => {
                            if state == 0 {
                                // Extract field name from node (skipping trivia)
                                for gc in &n.children {
                                    if let SyntaxElement::Token(t) = gc {
                                        if t.kind == TokenKind::Ident {
                                            current_field = Some(self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string());
                                            break;
                                        }
                                    }
                                }
                                state = 1;
                            } else if state == 2 {
                                if let Some(name) = current_field.take() {
                                    fields.push((name, self.lower_expr(n)));
                                }
                                state = 0;
                            }
                        }
                        SyntaxElement::Token(t) => {
                            if t.kind == TokenKind::Colon {
                                state = 2;
                            } else if t.kind == TokenKind::CloseBrace {
                                break;
                            }
                        }
                    }
                }
                ast::Expr::StructLiteral { path, fields }
            }
            _ => ast::Expr::Literal(ast::Literal::Nil),
        }
    }


    fn desugar_coalesce(&self, lhs: ast::Expr, rhs: ast::Expr) -> ast::Expr {
        // x ?? y -> branch x { Some(v) => v, None => y, Ok(v) => v, Err(_) => y }
        ast::Expr::Branch {
            target: Box::new(lhs),
            arms: vec![
                ast::Arm {
                    pattern: ast::Pattern::Variant("Some".to_string(), vec![ast::Pattern::Ident("v".to_string())]),
                    body: ast::Expr::Ident("v".to_string(), Span::dummy()),
                    span: Span::dummy(),
                },
                ast::Arm {
                    pattern: ast::Pattern::Ident("None".to_string()),
                    body: rhs.clone(),
                    span: Span::dummy(),
                },
                ast::Arm {
                    pattern: ast::Pattern::Variant("Ok".to_string(), vec![ast::Pattern::Ident("v".to_string())]),
                    body: ast::Expr::Ident("v".to_string(), Span::dummy()),
                    span: Span::dummy(),
                },
                ast::Arm {
                    pattern: ast::Pattern::Ident("_".to_string()),
                    body: rhs,
                    span: Span::dummy(),
                },
            ]
        }
    }

    fn desugar_optional_chain(&self, target: ast::Expr, name: String, span: Span) -> ast::Expr {
        // x?.y -> given let Some(t) = x { Some(t.y) } else { None }
        ast::Expr::Given {
            cond: Box::new(target),
            then_block: ast::Block {
                stmts: vec![],
                expr: Some(Box::new(ast::Expr::Member(Box::new(ast::Expr::Ident("t".to_string(), Span::dummy())), name, span))),
                span,
            },
            else_expr: Some(Box::new(ast::Expr::Literal(ast::Literal::Nil))),
        }
    }

    fn lower_generic_args(&self, node: &SyntaxNode) -> Vec<ast::Type> {
        let mut args = Vec::new();
        for child in &node.children {
            if let SyntaxElement::Node(n) = child {
                if n.kind == NodeKind::GenericArg {
                    for gc in &n.children {
                        if let SyntaxElement::Node(ty_node) = gc {
                             args.push(self.lower_type(ty_node));
                        }
                    }
                }
            }
        }
        args
    }

    fn lower_element(&self, element: &SyntaxElement) -> ast::Expr {
        match element {
            SyntaxElement::Node(node) => self.lower_expr(node),
            _ => ast::Expr::Literal(ast::Literal::Nil),
        }
    }

    // Item-specific lowerers
    fn lower_scroll(&self, node: &SyntaxNode) -> ast::Scroll {
        let mut name = String::new();
        let mut variants = Vec::new();
        let mut attributes = Vec::new();
        for child in &node.children {
            match child {
                SyntaxElement::Node(n) if n.kind == NodeKind::Attributes => {
                    attributes = self.lower_attributes(n);
                }
                SyntaxElement::Token(t) if t.kind == TokenKind::Ident => {
                    name = self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::Variant => {
                    variants.push(self.lower_variant(n));
                }
                _ => {}
            }
        }
        ast::Scroll { name, variants, attributes, span: node.span() }
    }

    fn lower_variant(&self, node: &SyntaxNode) -> ast::Variant {
        let mut name = String::new();
        let mut fields = None;
        for child in &node.children {
             match child {
                 SyntaxElement::Token(t) if t.kind == TokenKind::Ident => {
                     name = self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                 }
                 SyntaxElement::Node(n) if n.kind == NodeKind::Field => {
                      let f = self.lower_field(n);
                      if fields.is_none() { fields = Some(vec![]); }
                      fields.as_mut().unwrap().push(f);
                 }
                 _ => {}
             }
        }
        ast::Variant { name, fields, span: node.span() }
    }

    fn lower_alias(&self, node: &SyntaxNode) -> ast::Alias {
        let mut name = String::new();
        let mut ty = ast::Type::Error;
        let mut attributes = Vec::new();
        
        for child in &node.children {
            match child {
                SyntaxElement::Node(n) if n.kind == NodeKind::Attributes => {
                    attributes = self.lower_attributes(n);
                }
                SyntaxElement::Token(t) if t.kind == TokenKind::Ident => {
                    name = self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::Type => {
                    ty = self.lower_type(n);
                }
                _ => {}
            }
        }
        
        ast::Alias { name, ty, attributes, span: node.span() }
    }
    fn lower_weave(&self, node: &SyntaxNode) -> ast::Weave {
        let mut name = String::new();
        let mut associated_types = Vec::new();
        let mut methods = Vec::new();
        let mut attributes = Vec::new();
        
        for child in &node.children {
            match child {
                SyntaxElement::Node(n) if n.kind == NodeKind::Attributes => {
                    attributes = self.lower_attributes(n);
                }
                SyntaxElement::Token(t) if t.kind == TokenKind::Ident => {
                    name = self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::TypeAlias => {
                    let alias = self.lower_alias(n);
                    associated_types.push(alias.name);
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::ForgeDecl => {
                    methods.push(self.lower_forge(n));
                }
                _ => {}
            }
        }
        
        ast::Weave {
            name,
            associated_types,
            methods,
            attributes,
            span: node.span(),
        }
    }

    fn lower_ward(&self, node: &SyntaxNode) -> ast::Ward {
        let mut name = String::new();
        let mut items = Vec::new();
        let mut attributes = Vec::new();
        for child in &node.children {
            match child {
                SyntaxElement::Node(n) if n.kind == NodeKind::Attributes => {
                    attributes = self.lower_attributes(n);
                }
                SyntaxElement::Token(t) if t.kind == TokenKind::Ident => {
                    name = self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                }
                SyntaxElement::Node(n) => {
                     items.extend(self.lower_item(n));
                }
                _ => {}
            }
        }
        ast::Ward { name, items, attributes, span: node.span() }
    }

    fn lower_draw(&self, node: &SyntaxNode) -> ast::Draw {
        let mut path = Vec::new();
        let mut is_wildcard = false;
        for child in &node.children {
             match child {
                 SyntaxElement::Token(t) => {
                      if t.kind == TokenKind::Ident {
                           path.push(self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string());
                      } else if t.kind == TokenKind::Star {
                           is_wildcard = true;
                      }
                 }
                 _ => {}
             }
        }
        ast::Draw { path, is_wildcard, span: node.span() }
    }

    fn lower_impl(&self, node: &SyntaxNode) -> ast::Impl {
        let mut target = ast::Type::Error;
        let mut weave = None;
        let mut items = Vec::new();
        let mut attributes = Vec::new();
        
        let mut found_for = false;
        for child in &node.children {
            match child {
                SyntaxElement::Node(n) if n.kind == NodeKind::Attributes => {
                    attributes = self.lower_attributes(n);
                }
                SyntaxElement::Token(t) if t.kind == TokenKind::For => {
                     found_for = true;
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::ForgeDecl || n.kind == NodeKind::TypeAlias => {
                     items.extend(self.lower_item(n));
                }
                SyntaxElement::Token(t) if t.kind == TokenKind::Ident => {
                     let ty = ast::Type::Prim(self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string());
                     if !found_for {
                          weave = Some(ty);
                     } else {
                          target = ty;
                     }
                }
                SyntaxElement::Node(n) if n.kind == NodeKind::Type => {
                     let ty = self.lower_type(n);
                     if !found_for {
                          weave = Some(ty);
                     } else {
                          target = ty;
                     }
                }
                _ => {}
            }
        }
        
        if !found_for && weave.is_some() {
             target = weave.take().unwrap();
        }

        ast::Impl { target, weave, items, attributes, span: node.span() }
    }

    fn lower_effect(&self, node: &SyntaxNode) -> String {
        for child in &node.children {
            if let SyntaxElement::Token(t) = child {
                if matches!(t.kind, TokenKind::Ident | TokenKind::Pure) {
                    return self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                }
            }
        }
        String::new()
    }

    fn is_naming_ident(&self, kind: TokenKind) -> bool {
        match kind {
            TokenKind::Ident | TokenKind::SelfKw | TokenKind::Next | TokenKind::Loop | TokenKind::Each | 
            TokenKind::While | TokenKind::Break | TokenKind::Give | TokenKind::Type | 
            TokenKind::Forge | TokenKind::Sole | TokenKind::Pure | TokenKind::Open | 
            TokenKind::Hidden | TokenKind::Draw | TokenKind::Seek | TokenKind::Catch |
            TokenKind::Flow | TokenKind::Tide | TokenKind::Zone | TokenKind::Bridge |
            TokenKind::Raw | TokenKind::Echo | TokenKind::Ward | TokenKind::Scroll |
            TokenKind::Dual | TokenKind::Alias | TokenKind::Pkg | TokenKind::Comptime |
            TokenKind::Static | TokenKind::Extern | TokenKind::Bind => true,
            _ => false,
        }
    }

    fn lower_attributes(&self, node: &SyntaxNode) -> Vec<ast::Attribute> {
        let mut attrs = Vec::new();
        for child in &node.children {
            if let SyntaxElement::Node(n) = child {
                if n.kind == NodeKind::Attribute {
                    attrs.push(self.lower_attribute(n));
                }
            }
        }
        attrs
    }

    fn lower_attribute(&self, node: &SyntaxNode) -> ast::Attribute {
        let mut name = String::new();
        let mut args = Vec::new();
        for child in &node.children {
            match child {
                SyntaxElement::Token(t) if t.kind == TokenKind::Ident => {
                    name = self.source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                }
                SyntaxElement::Node(n) if n.kind != NodeKind::Attributes && n.kind != NodeKind::Attribute => {
                    args.push(self.lower_expr(n));
                }
                _ => {}
            }
        }
        ast::Attribute { name, args, span: node.span() }
    }

    fn desugar_pipeline(&self, lhs: ast::Expr, rhs: ast::Expr) -> ast::Expr {
        // x |> f     => f(x)
        // x |> f(y)  => f(x, y)
        match rhs {
            ast::Expr::Call(callee, mut args) => {
                // Prepend lhs to args
                args.insert(0, lhs);
                ast::Expr::Call(callee, args)
            }
            // If it's just an identifier or member access, treat it as a call with no args except lhs
            ast::Expr::Ident(..) | ast::Expr::Path(..) | ast::Expr::Member(..) => {
                ast::Expr::Call(Box::new(rhs), vec![lhs])
            }
            _ => {
                // Fallback: This shouldn't happen in valid Izel, but we wrap it in a call just in case
                ast::Expr::Call(Box::new(rhs), vec![lhs])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use izel_lexer::{Lexer, TokenKind, Token};

    fn tokenize(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source, izel_span::SourceId(0));
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            if token.kind == TokenKind::Eof {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }
        tokens
    }

    #[test]
    fn test_lower_attributes() {
        let source = "@proof forge f() {}";
        let tokens = tokenize(source);
        let mut parser = izel_parser::Parser::new(tokens, source.to_string());
        parser.source = source.to_string();
        let cst = parser.parse_decl();
        
        let lowerer = Lowerer::new(source);
        let mut items = lowerer.lower_item(&cst);
        let item = items.pop().unwrap();
        
        if let ast::Item::Forge(f) = item {
            assert_eq!(f.name, "f");
            assert_eq!(f.attributes.len(), 1);
            assert_eq!(f.attributes[0].name, "proof");
        } else {
            panic!("Expected Forge item");
        }
    }

    #[test]
    fn test_lower_attributes_with_args() {
        let source = "@requires(n > 0) forge f(n: i32) {}";
        let tokens = tokenize(source);
        let mut parser = izel_parser::Parser::new(tokens, source.to_string());
        parser.source = source.to_string();
        let cst = parser.parse_decl();
        
        let lowerer = Lowerer::new(source);
        let mut items = lowerer.lower_item(&cst);
        let item = items.pop().unwrap();
        
        if let ast::Item::Forge(f) = item {
            assert_eq!(f.name, "f");
            assert_eq!(f.attributes.len(), 1);
            assert_eq!(f.attributes[0].name, "requires");
            assert_eq!(f.attributes[0].args.len(), 1);
        } else {
            panic!("Expected Forge item");
        }
    }

    #[test]
    fn test_lower_cascade_expr() {
        let source = "foo!";
        let tokens = tokenize(source);
        let mut parser = izel_parser::Parser::new(tokens, source.to_string());
        parser.source = source.to_string();
        let cst = parser.parse_expr(izel_parser::expr::Precedence::None);
        
        let lowerer = Lowerer::new(source);
        let expr = lowerer.lower_expr(&cst);
        
        match expr {
            ast::Expr::Cascade { expr, context } => {
                assert!(matches!(*expr, ast::Expr::Ident(..)));
                assert!(context.is_none());
            }
            _ => panic!("Expected Expr::Cascade"),
        }
    }

    #[test]
    fn test_lower_macro_here() {
        let source = "here!()";
        let tokens = tokenize(source);
        let mut parser = izel_parser::Parser::new(tokens, source.to_string());
        parser.source = source.to_string();
        let cst = parser.parse_expr(izel_parser::expr::Precedence::None);
        
        let lowerer = Lowerer::new(source);
        let expr = lowerer.lower_expr(&cst);
        
        match expr {
            ast::Expr::Literal(ast::Literal::Str(s)) => {
                // Line 1 because the string only has one line, file 'main.iz' is default
                assert_eq!(s, "main.iz:1");
            }
            _ => panic!("Expected Expr::Literal(Str)"),
        }
    }

    #[test]
    fn test_lower_dual_decl() {
        let source = "dual shape JsonFormat<T> { forge encode(&self, val: &T) }";
        let tokens = tokenize(source);
        let mut parser = izel_parser::Parser::new(tokens, source.to_string());
        parser.source = source.to_string();
        let cst = parser.parse_decl();
        
        let lowerer = Lowerer::new(source);
        let mut items = lowerer.lower_item(&cst);
        let item = items.remove(0); // Take the first item which should be Dual
        
        if let ast::Item::Dual(d) = item {
            assert_eq!(d.name, "JsonFormat");
            assert_eq!(d.generic_params.len(), 1);
            // Elaboration should have generated the inverse decode method, resulting in 2 items!
            assert_eq!(d.items.len(), 2);
            
            let mut found_encode = false;
            let mut found_decode = false;
            for i in d.items {
                if let ast::Item::Forge(f) = i {
                    if f.name == "encode" { found_encode = true; }
                    if f.name == "decode" { found_decode = true; }
                }
            }
            assert!(found_encode && found_decode, "Both encode and decode should be present");
        } else {
            panic!("Expected Dual item");
        }
    }
}
