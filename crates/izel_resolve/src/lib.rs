//! Name and scope resolution for Izel.

use izel_lexer::TokenKind;
use izel_parser::cst::{NodeKind, SyntaxElement, SyntaxNode};
use izel_span::Span;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(pub usize);

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub span: Span,
    pub def_id: DefId,
    pub is_module: bool,
    pub module_scope: Option<Arc<Scope>>,
}

#[derive(Debug)]
pub struct Scope {
    pub symbols: RefCell<FxHashMap<String, Symbol>>,
    pub parent: Option<Arc<Scope>>,
}

impl Scope {
    pub fn new(parent: Option<Arc<Scope>>) -> Self {
        Self {
            symbols: RefCell::new(FxHashMap::default()),
            parent,
        }
    }

    pub fn define(&self, name: String, span: Span, def_id: DefId) {
        let symbol = Symbol {
            name: name.clone(),
            span,
            def_id,
            is_module: false,
            module_scope: None,
        };
        self.symbols.borrow_mut().insert(name, symbol);
    }

    pub fn define_module(&self, name: String, span: Span, def_id: DefId, scope: Arc<Scope>) {
        let symbol = Symbol {
            name: name.clone(),
            span,
            def_id,
            is_module: true,
            module_scope: Some(scope),
        };
        self.symbols.borrow_mut().insert(name, symbol);
    }

    pub fn resolve(&self, name: &str) -> Option<Symbol> {
        if let Some(symbol) = self.symbols.borrow().get(name) {
            return Some(symbol.clone());
        }
        if let Some(parent) = &self.parent {
            return parent.resolve(name);
        }
        None
    }

    pub fn resolve_local(&self, name: &str) -> Option<Symbol> {
        self.symbols.borrow().get(name).cloned()
    }

    pub fn merge_scope(&self, other: &Scope) {
        let mut symbols = self.symbols.borrow_mut();
        for (name, sym) in other.symbols.borrow().iter() {
            if !symbols.contains_key(name) {
                symbols.insert(name.clone(), sym.clone());
            }
        }
    }
}

pub struct Resolver {
    next_def_id: usize,
    pub root_scope: Arc<Scope>,
    pub current_scope: Arc<Scope>,
}

impl Resolver {
    pub fn new() -> Self {
        let root = Arc::new(Scope::new(None));
        Self {
            next_def_id: 0,
            root_scope: root.clone(),
            current_scope: root,
        }
    }

    pub fn next_id(&mut self) -> DefId {
        let id = DefId(self.next_def_id);
        self.next_def_id += 1;
        id
    }

    pub fn resolve_source_file(&mut self, node: &SyntaxNode, source: &str) {
        self.resolve_children(node, source);
    }

    fn resolve_children(&mut self, node: &SyntaxNode, source: &str) {
        for child in &node.children {
            if let SyntaxElement::Node(child_node) = child {
                match child_node.kind {
                    NodeKind::ForgeDecl => {
                        self.resolve_named_decl(child_node, TokenKind::Forge, source);
                        // Also resolve inside forge
                        self.resolve_block_in_node(child_node, source);
                    }
                    NodeKind::ShapeDecl => {
                        self.resolve_named_decl(child_node, TokenKind::Shape, source)
                    }
                    NodeKind::ScrollDecl => {
                        self.resolve_named_decl(child_node, TokenKind::Scroll, source)
                    }
                    NodeKind::WardDecl => self.resolve_ward_decl(child_node, source),
                    NodeKind::DualDecl => {
                        self.resolve_named_decl(child_node, TokenKind::Dual, source)
                    }
                    NodeKind::ImplBlock => self.resolve_impl_block(child_node, source),
                    NodeKind::TypeAlias => {
                        self.resolve_named_decl(child_node, TokenKind::Type, source)
                    }
                    NodeKind::Block => self.resolve_block(child_node, source),
                    NodeKind::LetStmt => self.resolve_let_stmt(child_node, source),
                    NodeKind::DrawDecl => self.resolve_draw_decl(child_node, source),
                    NodeKind::Ident => {
                        // Resolve use of identifier
                        let span = child_node.span();
                        let name = source[span.lo.0 as usize..span.hi.0 as usize].to_string();
                        if let Some(sym) = self.current_scope.resolve(&name) {
                            // Link this use to sym.def_id
                            // For now we just print/log
                            println!("Resolved use: {} to DefId({:?})", name, sym.def_id);
                        }
                    }
                    _ => self.resolve_children(child_node, source),
                }
            }
        }
    }

    fn resolve_block_in_node(&mut self, node: &SyntaxNode, source: &str) {
        for child in &node.children {
            if let SyntaxElement::Node(child_node) = child {
                if child_node.kind == NodeKind::Block {
                    self.resolve_block(child_node, source);
                }
            }
        }
    }

    fn resolve_impl_block(&mut self, node: &SyntaxNode, source: &str) {
        for child in &node.children {
            match child {
                SyntaxElement::Token(t) if t.kind == TokenKind::Ident => {
                    let name = source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                    if let Some(sym) = self.current_scope.resolve(&name) {
                        println!("Resolved impl ref: {} to DefId({:?})", name, sym.def_id);
                    }
                }
                SyntaxElement::Node(n) => self.resolve_children(n, source),
                _ => {}
            }
        }
    }

    fn resolve_block(&mut self, node: &SyntaxNode, source: &str) {
        let parent = self.current_scope.clone();
        self.current_scope = Arc::new(Scope::new(Some(parent)));

        self.resolve_children(node, source);

        let p = self
            .current_scope
            .parent
            .clone()
            .expect("Block scope must have parent");
        self.current_scope = p;
    }

    fn resolve_let_stmt(&mut self, node: &SyntaxNode, source: &str) {
        let mut found_let = false;
        for child in &node.children {
            match child {
                SyntaxElement::Token(t)
                    if t.kind == TokenKind::Let || t.kind == TokenKind::Tilde =>
                {
                    found_let = true;
                }
                SyntaxElement::Token(t) if found_let && t.kind == TokenKind::Ident => {
                    let name = source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                    let id = self.next_id();
                    self.current_scope.define(name, t.span, id);
                    break;
                }
                SyntaxElement::Node(n) => {
                    // Resolve RHS before defining the name (if we want to prevent recursive use)
                    self.resolve_children(n, source);
                }
                _ => {}
            }
        }
    }

    fn resolve_named_decl(&mut self, node: &SyntaxNode, keyword: TokenKind, source: &str) {
        let mut found_kw = false;
        for child in &node.children {
            match child {
                SyntaxElement::Token(token) => {
                    if token.kind == keyword {
                        found_kw = true;
                    } else if found_kw && token.kind == TokenKind::Ident {
                        let span = token.span;
                        let name = source[span.lo.0 as usize..span.hi.0 as usize].to_string();
                        let id = self.next_id();
                        self.current_scope.define(name, span, id);
                        found_kw = false; // reset to avoid matching subsequent idents
                    }
                }
                SyntaxElement::Node(n) => {
                    self.resolve_children(n, source);
                }
            }
        }
    }

    fn resolve_ward_decl(&mut self, node: &SyntaxNode, source: &str) {
        let mut name_info = None;
        let mut found_ward = false;
        for child in &node.children {
            if let SyntaxElement::Token(token) = child {
                if token.kind == TokenKind::Ward {
                    found_ward = true;
                } else if found_ward && token.kind == TokenKind::Ident {
                    let span = token.span;
                    let name = source[span.lo.0 as usize..span.hi.0 as usize].to_string();
                    name_info = Some((name, span));
                    break;
                }
            }
        }

        if let Some((name, span)) = name_info {
            let id = self.next_id();
            let new_scope = Arc::new(Scope::new(Some(self.current_scope.clone())));
            self.current_scope
                .define_module(name, span, id, new_scope.clone());

            // Push scope
            let parent = self.current_scope.clone();
            self.current_scope = new_scope;

            // Resolve elements inside the ward
            self.resolve_children(node, source);

            // Pop scope
            self.current_scope = parent;
        }
    }

    fn resolve_draw_decl(&mut self, node: &SyntaxNode, source: &str) {
        let mut path = vec![];
        let mut is_wildcard = false;

        for child in &node.children {
            if let SyntaxElement::Token(t) = child {
                match t.kind {
                    TokenKind::Ident => {
                        let name = source[t.span.lo.0 as usize..t.span.hi.0 as usize].to_string();
                        path.push((name, t.span));
                    }
                    TokenKind::Star => {
                        is_wildcard = true;
                    }
                    _ => {}
                }
            }
        }

        if path.is_empty() {
            return;
        }

        let mut current_scope = self.current_scope.clone();

        // Traverse/Build path
        for (i, (name, span)) in path.iter().enumerate() {
            let is_last = i == path.len() - 1;

            let symbol = current_scope.resolve_local(name);
            match symbol {
                Some(sym) if sym.is_module => {
                    current_scope = sym.module_scope.clone().unwrap();
                }
                None => {
                    let id = self.next_id();
                    let new_mod_scope = Arc::new(Scope::new(None)); // Dummy or loaded module
                    current_scope.define_module(name.clone(), *span, id, new_mod_scope.clone());
                    current_scope = new_mod_scope;
                }
                _ => {
                    // Symbol exists but is not a module, error or handle as re-export
                }
            }

            if is_last && is_wildcard {
                self.current_scope.merge_scope(&current_scope);
            }
        }
    }
}
