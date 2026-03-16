//! Name and scope resolution for Izel.

use rustc_hash::FxHashMap;
use izel_span::Span;
use std::sync::Arc;
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

#[derive(Clone)]
pub struct Scope {
    pub symbols: FxHashMap<String, Symbol>,
    pub parent: Option<Arc<Scope>>,
}

impl Scope {
    pub fn new(parent: Option<Arc<Scope>>) -> Self {
        Self {
            symbols: FxHashMap::default(),
            parent,
        }
    }

    pub fn define(&mut self, name: String, span: Span, def_id: DefId) {
        let symbol = Symbol { name: name.clone(), span, def_id };
        self.symbols.insert(name, symbol);
    }

    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        if let Some(symbol) = self.symbols.get(name) {
            return Some(symbol);
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
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            next_def_id: 0,
            root_scope: Arc::new(Scope::new(None)),
        }
    }

    pub fn next_id(&mut self) -> DefId {
        let id = DefId(self.next_def_id);
        self.next_def_id += 1;
        id
    }

    pub fn resolve_source_file(&mut self, node: &SyntaxNode, source: &str) {
        for child in &node.children {
            if let SyntaxElement::Node(child_node) = child {
                if child_node.kind == NodeKind::ForgeDecl {
                    self.resolve_forge_decl(child_node, source);
                }
            }
        }
    }

    fn resolve_forge_decl(&mut self, node: &SyntaxNode, source: &str) {
        for child in &node.children {
            if let SyntaxElement::Token(token) = child {
                if token.kind == TokenKind::Ident {
                    let span = token.span;
                    let name = source[span.lo.0 as usize..span.hi.0 as usize].to_string();
                    let id = self.next_id();
                    
                    // Root scope definition
                    let scope = Arc::make_mut(&mut self.root_scope);
                    scope.define(name, span, id);
                }
            }
        }
    }
}
