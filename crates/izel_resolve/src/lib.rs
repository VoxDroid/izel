//! Name and scope resolution for Izel.

use rustc_hash::FxHashMap;
use izel_span::Span;
use std::sync::Arc;
use std::cell::RefCell;
use izel_lexer::TokenKind;
use izel_parser::cst::{SyntaxNode, SyntaxElement, NodeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(pub usize);

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub span: Span,
    pub def_id: DefId,
}

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
        let symbol = Symbol { name: name.clone(), span, def_id };
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
                    NodeKind::ForgeDecl => self.resolve_named_decl(child_node, TokenKind::Forge, source),
                    NodeKind::ShapeDecl => self.resolve_named_decl(child_node, TokenKind::Shape, source),
                    NodeKind::ScrollDecl => self.resolve_named_decl(child_node, TokenKind::Scroll, source),
                    NodeKind::WardDecl => self.resolve_ward_decl(child_node, source),
                    NodeKind::DualDecl => self.resolve_named_decl(child_node, TokenKind::Dual, source),
                    NodeKind::WeaveDecl => self.resolve_named_decl(child_node, TokenKind::Weave, source),
                    _ => {}
                }
            }
        }
    }

    fn resolve_named_decl(&mut self, node: &SyntaxNode, keyword: TokenKind, source: &str) {
        let mut found_kw = false;
        for child in &node.children {
            if let SyntaxElement::Token(token) = child {
                if token.kind == keyword {
                    found_kw = true;
                } else if found_kw && token.kind == TokenKind::Ident {
                    let span = token.span;
                    let name = source[span.lo.0 as usize..span.hi.0 as usize].to_string();
                    let id = self.next_id();
                    
                    self.current_scope.define(name, span, id);
                    break;
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
            self.current_scope.define(name, span, id);
            
            // Push scope
            let parent = self.current_scope.clone();
            self.current_scope = Arc::new(Scope::new(Some(parent)));
            
            // Resolve elements inside the ward
            self.resolve_children(node, source);
            
            // Pop scope
            let parent = self.current_scope.parent.clone().expect("Ward scope must have parent");
            self.current_scope = parent;
        }
    }
}
