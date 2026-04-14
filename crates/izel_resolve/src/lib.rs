#![allow(clippy::arc_with_non_send_sync)]
//! Name and scope resolution for Izel.

use izel_lexer::TokenKind;
use izel_parser::cst::{NodeKind, SyntaxElement, SyntaxNode};
use izel_span::Span;
use rustc_hash::FxHashMap;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, RwLock,
};

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
    pub symbols: RwLock<FxHashMap<String, Symbol>>,
    pub parent: Option<Arc<Scope>>,
}

impl Scope {
    pub fn new(parent: Option<Arc<Scope>>) -> Self {
        Self {
            symbols: RwLock::new(FxHashMap::default()),
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
        self.symbols.write().unwrap().insert(name, symbol);
    }

    pub fn define_module(&self, name: String, span: Span, def_id: DefId, scope: Arc<Scope>) {
        let symbol = Symbol {
            name: name.clone(),
            span,
            def_id,
            is_module: true,
            module_scope: Some(scope),
        };
        self.symbols.write().unwrap().insert(name, symbol);
    }

    pub fn resolve(&self, name: &str) -> Option<Symbol> {
        if let Some(symbol) = self.symbols.read().unwrap().get(name) {
            return Some(symbol.clone());
        }
        if let Some(parent) = &self.parent {
            return parent.resolve(name);
        }
        None
    }

    pub fn resolve_local(&self, name: &str) -> Option<Symbol> {
        self.symbols.read().unwrap().get(name).cloned()
    }

    pub fn merge_scope(&self, other: &Scope) {
        let mut symbols = self.symbols.write().unwrap();
        for (name, sym) in other.symbols.read().unwrap().iter() {
            if !symbols.contains_key(name) {
                symbols.insert(name.clone(), sym.clone());
            }
        }
    }
}

use std::path::Path;
use std::path::PathBuf;

pub struct Resolver {
    pub root_scope: Arc<Scope>,
    pub current_scope: Arc<Scope>,
    pub base_path: Option<PathBuf>,
    pub loaded_csts: Arc<RwLock<FxHashMap<String, (SyntaxNode, String)>>>,
    pub def_ids: Arc<RwLock<FxHashMap<Span, DefId>>>,
    pub next_def_id: Arc<AtomicU32>,
    pub next_source_id: Arc<AtomicU32>,
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Resolver {
    pub fn new(base_path: Option<PathBuf>) -> Self {
        let root = Arc::new(Scope::new(None));
        Self {
            root_scope: root.clone(),
            current_scope: root,
            base_path,
            loaded_csts: Arc::new(RwLock::new(FxHashMap::default())),
            def_ids: Arc::new(RwLock::new(FxHashMap::default())),
            next_def_id: Arc::new(AtomicU32::new(0)),
            next_source_id: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn create_module_resolver(&self, path: &Path) -> Option<Resolver> {
        // Load and parse other file
        // FOR NOW: just return a new resolver for the path context
        let root = Arc::new(Scope::new(None));
        let resolver = Self {
            root_scope: root.clone(),
            current_scope: root,
            base_path: Some(match path.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
                _ => PathBuf::from("."),
            }),
            loaded_csts: self.loaded_csts.clone(),
            def_ids: Arc::clone(&self.def_ids),
            next_def_id: Arc::clone(&self.next_def_id),
            next_source_id: Arc::clone(&self.next_source_id),
        };
        Some(resolver)
    }

    pub fn load_module(&mut self, name: &str) -> Option<Arc<Scope>> {
        if let Some(mod_sym) = self.root_scope.resolve_local(name) {
            if mod_sym.is_module {
                return mod_sym.module_scope.clone();
            }
        }

        let base = self.base_path.as_ref()?;
        let file_path = if let Some(stripped) = name.strip_prefix("std/") {
            std::path::PathBuf::from("std").join(format!("{}.iz", stripped))
        } else {
            base.join(format!("{}.iz", name))
        };

        if !file_path.exists() {
            return None;
        }

        let source = std::fs::read_to_string(&file_path).ok()?;
        let source_id = self.next_source_id.fetch_add(1, Ordering::SeqCst) + 1;
        let mut lexer = izel_lexer::Lexer::new(&source, izel_span::SourceId(source_id));
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            if token.kind == TokenKind::Eof {
                break;
            }
            tokens.push(token);
        }

        let mut parser = izel_parser::Parser::new(tokens, source.clone());
        let cst = parser.parse_source_file();

        let new_mod_scope = Arc::new(Scope::new(None));
        let prev_scope = self.current_scope.clone();
        let prev_base = self.base_path.clone();

        self.current_scope = new_mod_scope.clone();
        self.base_path = Some(match file_path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
            _ => base.clone(),
        });

        self.resolve_source_file(&cst, &source);

        self.current_scope = prev_scope;
        self.base_path = prev_base;

        self.loaded_csts
            .write()
            .unwrap()
            .insert(name.to_string(), (cst, source));

        Some(new_mod_scope)
    }

    pub fn next_id(&self) -> DefId {
        DefId(self.next_def_id.fetch_add(1, Ordering::SeqCst) as usize)
    }

    pub fn resolve_source_file(&mut self, node: &SyntaxNode, source: &str) {
        self.resolve_children(node, source);
    }

    fn span_text<'a>(&self, source: &'a str, span: Span) -> Option<&'a str> {
        source.get(span.lo.0 as usize..span.hi.0 as usize)
    }

    fn resolve_children(&mut self, node: &SyntaxNode, source: &str) {
        for child in &node.children {
            match child {
                SyntaxElement::Node(child_node) => match child_node.kind {
                    NodeKind::ForgeDecl => self.resolve_forge_decl(child_node, source),
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
                    NodeKind::Param | NodeKind::ParamPart => self.resolve_param(child_node, source),
                    NodeKind::Block => self.resolve_block(child_node, source),
                    NodeKind::LetStmt => self.resolve_let_stmt(child_node, source),
                    NodeKind::DrawDecl => self.resolve_draw_decl(child_node, source),
                    _ => self.resolve_children(child_node, source),
                },
                SyntaxElement::Token(t) => {
                    if t.kind == TokenKind::Ident {
                        let span = t.span;
                        if let Some(name) = self.span_text(source, span) {
                            if let Some(sym) = self.current_scope.resolve(name) {
                                self.def_ids.write().unwrap().insert(span, sym.def_id);
                            }
                        }
                    }
                }
            }
        }
    }

    fn resolve_impl_block(&mut self, node: &SyntaxNode, source: &str) {
        for child in &node.children {
            match child {
                SyntaxElement::Token(t) if t.kind == TokenKind::Ident => {
                    if let Some(name) = self.span_text(source, t.span) {
                        let _ = self.current_scope.resolve(name);
                    }
                }
                SyntaxElement::Node(n) => self.resolve_children(n, source),
                SyntaxElement::Token(_) => {}
            }
        }
    }

    fn resolve_block(&mut self, node: &SyntaxNode, source: &str) {
        let parent = self.current_scope.clone();
        self.current_scope = Arc::new(Scope::new(Some(parent)));

        self.resolve_children(node, source);

        if let Some(parent) = self.current_scope.parent.clone() {
            self.current_scope = parent;
        } else {
            self.current_scope = self.root_scope.clone();
        }
    }

    fn resolve_let_stmt(&mut self, node: &SyntaxNode, source: &str) {
        let mut defined_name = false;
        let mut found_let_or_tilde = false;
        for child in &node.children {
            match child {
                SyntaxElement::Token(t)
                    if t.kind == TokenKind::Let || t.kind == TokenKind::Tilde =>
                {
                    found_let_or_tilde = true;
                }
                SyntaxElement::Token(t)
                    if found_let_or_tilde && !defined_name && t.kind == TokenKind::Ident =>
                {
                    if let Some(name) = self.span_text(source, t.span) {
                        let id = self.next_id();
                        self.current_scope.define(name.to_string(), t.span, id);
                        self.def_ids.write().unwrap().insert(t.span, id);
                        defined_name = true;
                        // Continue to resolve the rest of the statement (e.g., the RHS)
                    }
                }
                SyntaxElement::Node(n)
                    if found_let_or_tilde
                        && !defined_name
                        && (n.kind == NodeKind::Ident
                            || n.kind == NodeKind::Identifier
                            || n.kind == NodeKind::Pattern) =>
                {
                    // This handles cases like `let (a, b) = ...` or `let Some(x) = ...`
                    // We need to find all idents within the pattern and define them.
                    // For simplicity, we'll just define the first ident found for now,
                    // but a full pattern matching resolver would iterate and define all.
                    for pattern_child in &n.children {
                        if let SyntaxElement::Token(t) = pattern_child {
                            if t.kind == TokenKind::Ident {
                                if let Some(name) = self.span_text(source, t.span) {
                                    let id = self.next_id();
                                    self.current_scope.define(name.to_string(), t.span, id);
                                    self.def_ids.write().unwrap().insert(t.span, id);
                                    defined_name = true;
                                    // Continue to resolve other parts of the pattern or the RHS
                                }
                            }
                        }
                        // Recursively resolve children of the pattern node if they are nodes
                        if let SyntaxElement::Node(sub_node) = pattern_child {
                            self.resolve_children(sub_node, source);
                        }
                    }
                }
                SyntaxElement::Node(n) => {
                    // Resolve children for the RHS of the let statement or other parts
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
                        if let Some(name) = self.span_text(source, span) {
                            let id = self.next_id();
                            self.current_scope.define(name.to_string(), span, id);
                            self.def_ids.write().unwrap().insert(span, id);
                            found_kw = false; // reset to avoid matching subsequent idents
                        }
                    }
                }
                SyntaxElement::Node(n) => {
                    self.resolve_children(n, source);
                }
            }
        }
    }

    fn resolve_named_decl_only(&mut self, node: &SyntaxNode, keyword: TokenKind, source: &str) {
        let mut found_kw = false;
        for child in &node.children {
            if let SyntaxElement::Token(token) = child {
                if token.kind == keyword {
                    found_kw = true;
                } else if found_kw && token.kind == TokenKind::Ident {
                    let span = token.span;
                    if let Some(name) = self.span_text(source, span) {
                        let id = self.next_id();

                        self.current_scope.define(name.to_string(), span, id);
                        self.def_ids.write().unwrap().insert(span, id);
                        return;
                    }
                }
            }
        }
    }

    fn resolve_forge_decl(&mut self, node: &SyntaxNode, source: &str) {
        self.resolve_named_decl_only(node, TokenKind::Forge, source);

        let parent = self.current_scope.clone();
        self.current_scope = Arc::new(Scope::new(Some(parent)));

        self.resolve_children(node, source);

        if let Some(parent) = self.current_scope.parent.clone() {
            self.current_scope = parent;
        } else {
            self.current_scope = self.root_scope.clone();
        }
    }

    fn resolve_param(&mut self, node: &SyntaxNode, source: &str) {
        for child in &node.children {
            match child {
                SyntaxElement::Token(t) if t.kind == TokenKind::Ident => {
                    if let Some(name) = self.span_text(source, t.span) {
                        let id = self.next_id();
                        self.current_scope.define(name.to_string(), t.span, id);
                        self.def_ids.write().unwrap().insert(t.span, id);
                        return;
                    }
                }
                _ => {}
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
                    if let Some(name) = self.span_text(source, span) {
                        name_info = Some((name.to_string(), span));
                        break;
                    }
                }
            }
        }

        if let Some((name, span)) = name_info {
            let id = self.next_id();
            let new_scope = Arc::new(Scope::new(Some(self.current_scope.clone())));
            self.current_scope
                .define_module(name, span, id, new_scope.clone());
            self.def_ids.write().unwrap().insert(span, id);

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
                        if let Some(name) = self.span_text(source, t.span) {
                            path.push((name.to_string(), t.span));
                        }
                    }
                    TokenKind::Star => {
                        is_wildcard = true;
                    }
                    _ => {}
                }
            }
        }

        let _ = is_wildcard; // Silencing warning while behavior is wildcard-by-default

        if path.is_empty() {
            return;
        }

        let mut current_scope = self.current_scope.clone();

        // Traverse/Build path
        let mut full_path = String::new();
        for (i, (name, span)) in path.iter().enumerate() {
            if !full_path.is_empty() {
                full_path.push('/');
            }
            full_path.push_str(name);
            let is_last = i == path.len() - 1;

            let symbol = current_scope.resolve_local(name);
            match symbol {
                Some(sym) if sym.is_module => {
                    if let Some(module_scope) = sym.module_scope.clone() {
                        current_scope = module_scope;
                    }
                }
                None => {
                    let new_mod_scope = self
                        .load_module(&full_path)
                        .unwrap_or_else(|| Arc::new(Scope::new(None)));
                    let id = self.next_id();
                    current_scope.define_module(name.clone(), *span, id, new_mod_scope.clone());
                    current_scope = new_mod_scope;
                }
                _ => {
                    // Symbol exists but is not a module, error or handle as re-export
                }
            }

            if is_last {
                self.current_scope.merge_scope(&current_scope);
            }
        }
    }
}
