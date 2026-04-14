use izel_lexer::{Lexer, Token, TokenKind};
use izel_parser::{
    cst::{NodeKind, SyntaxElement, SyntaxNode},
    Parser,
};
use izel_resolve::{DefId, Resolver, Scope};
use izel_span::{BytePos, SourceId, Span};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

fn parse_source(source: &str) -> SyntaxNode {
    let mut lexer = Lexer::new(source, SourceId(0));
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        if token.kind == TokenKind::Eof {
            break;
        }
        tokens.push(token);
    }

    let mut parser = Parser::new(tokens, source.to_string());
    parser.parse_source_file()
}

fn new_temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    path.push(format!("{}_{}_{}", prefix, std::process::id(), nanos));
    fs::create_dir_all(&path).expect("temp dir should be created");
    path
}

#[test]
fn scope_define_and_resolve_through_parent_chain() {
    let root = Arc::new(Scope::new(None));
    let span = Span::new(BytePos(0), BytePos(3), SourceId(0));
    root.define("abc".to_string(), span, DefId(10));

    let child = Scope::new(Some(root.clone()));
    let sym = child.resolve("abc").expect("parent symbol should resolve");

    assert_eq!(sym.name, "abc");
    assert_eq!(sym.def_id, DefId(10));
    assert_eq!(sym.span, span);
}

#[test]
fn scope_merge_copies_missing_symbols_only() {
    let left = Scope::new(None);
    let right = Scope::new(None);
    let a_span = Span::new(BytePos(0), BytePos(1), SourceId(0));
    let b_span = Span::new(BytePos(1), BytePos(2), SourceId(0));

    left.define("a".to_string(), a_span, DefId(1));
    right.define("a".to_string(), b_span, DefId(99));
    right.define("b".to_string(), b_span, DefId(2));

    left.merge_scope(&right);

    assert_eq!(left.resolve("a").unwrap().def_id, DefId(1));
    assert_eq!(left.resolve("b").unwrap().def_id, DefId(2));
}

#[test]
fn resolver_next_id_and_module_resolver_path_are_consistent() {
    let resolver = Resolver::new(Some(PathBuf::from("examples")));
    assert_eq!(resolver.next_id(), DefId(0));
    assert_eq!(resolver.next_id(), DefId(1));

    let sub = resolver
        .create_module_resolver(Path::new("examples/main.iz"))
        .expect("module resolver should be created");
    assert_eq!(sub.base_path.as_deref(), Some(Path::new("examples")));
}

#[test]
fn resolver_create_module_resolver_handles_parentless_path() {
    let resolver = Resolver::new(Some(PathBuf::from("examples")));
    let sub = resolver
        .create_module_resolver(Path::new("main.iz"))
        .expect("module resolver should be created");
    assert_eq!(sub.base_path.as_deref(), Some(Path::new(".")));
}

#[test]
fn resolver_resolve_source_file_defines_symbols_and_restores_scope() {
    let base = new_temp_dir("izel_resolve_source_file");
    let source = r#"
shape Point { x: i32 }
scroll Color { Red, Blue }
type MaybeI32 = ?i32
forge run(a: i32) {
    let ~tmp = a
    let (lhs, rhs) = pair
    give tmp
}
ward Core {
    forge inner(v: i32) {
        let value = v
        give value
    }
}
dual Pair { left: i32, right: i32 }
impl Point {
    forge area(self) { give 0 }
}
draw missing/path::*;
"#;

    let cst = parse_source(source);
    let mut resolver = Resolver::new(Some(base.clone()));
    resolver.resolve_source_file(&cst, source);

    assert!(resolver.root_scope.resolve("Point").is_some());
    assert!(resolver.root_scope.resolve("Color").is_some());
    assert!(resolver.root_scope.resolve("MaybeI32").is_some());
    assert!(resolver.root_scope.resolve("run").is_some());

    let core = resolver
        .root_scope
        .resolve("Core")
        .expect("ward symbol should be present in root scope");
    assert!(core.is_module);
    assert!(core.module_scope.is_some());

    assert!(resolver.root_scope.resolve("missing").is_some());
    assert!(!resolver.def_ids.read().unwrap().is_empty());
    assert!(Arc::ptr_eq(&resolver.root_scope, &resolver.current_scope));

    fs::remove_dir_all(base).expect("temp dir should be removable");
}

#[test]
fn resolver_load_module_covers_base_path_and_cached_module_branches() {
    let base = new_temp_dir("izel_resolve_load_module");
    fs::write(
        base.join("pkg_mod.iz"),
        "shape Imported { x: i32 }\nforge helper() { give 0 }\n",
    )
    .expect("module file should be written");

    let mut resolver = Resolver::new(Some(base.clone()));
    assert!(resolver.load_module("missing_module").is_none());

    let loaded_scope = resolver
        .load_module("pkg_mod")
        .expect("existing module should load");
    assert!(loaded_scope.resolve("Imported").is_some());
    assert!(loaded_scope.resolve("helper").is_some());
    assert!(resolver.loaded_csts.read().unwrap().contains_key("pkg_mod"));

    let cached_scope = Arc::new(Scope::new(None));
    let cached_span = Span::new(BytePos(10), BytePos(16), SourceId(0));
    resolver.root_scope.define_module(
        "cached".to_string(),
        cached_span,
        DefId(77),
        cached_scope.clone(),
    );
    let cached_loaded = resolver
        .load_module("cached")
        .expect("cached module should resolve without loading from disk");
    assert!(Arc::ptr_eq(&cached_loaded, &cached_scope));

    let mut no_base = Resolver::new(None);
    assert!(no_base.load_module("pkg_mod").is_none());

    fs::remove_dir_all(base).expect("temp dir should be removable");
}

#[test]
fn resolver_manual_cst_paths_cover_impl_let_draw_and_default_branches() {
    fn span(lo: u32, hi: u32) -> Span {
        Span::new(BytePos(lo), BytePos(hi), SourceId(0))
    }

    fn tok(kind: TokenKind, lo: u32, hi: u32) -> SyntaxElement {
        SyntaxElement::Token(Token::new(kind, span(lo, hi)))
    }

    let mut resolver = Resolver::default();

    // Hit load_module branch where a local symbol exists but is not a module.
    let name_span = span(0, 1);
    resolver
        .root_scope
        .define("n".to_string(), name_span, DefId(99));
    let base = new_temp_dir("izel_resolve_non_module_local");
    resolver.base_path = Some(base.clone());
    assert!(resolver.load_module("n").is_none());

    // Prepare an already-loaded module to cover draw traversal through an existing module symbol.
    let preloaded = Arc::new(Scope::new(None));
    preloaded.define("inner".to_string(), span(6, 7), DefId(200));
    resolver.current_scope.define_module(
        "m".to_string(),
        span(4, 5),
        DefId(199),
        preloaded.clone(),
    );

    let source = "i l n m";
    let impl_block = SyntaxNode::new(NodeKind::ImplBlock, vec![tok(TokenKind::Ident, 0, 1)]);
    let let_stmt = SyntaxNode::new(
        NodeKind::LetStmt,
        vec![tok(TokenKind::Let, 2, 3), tok(TokenKind::Ident, 4, 5)],
    );
    let draw_existing = SyntaxNode::new(
        NodeKind::DrawDecl,
        vec![
            tok(TokenKind::Ident, 6, 7),
            SyntaxElement::Node(SyntaxNode::new(
                NodeKind::ExprStmt,
                vec![tok(TokenKind::Ident, 0, 1)],
            )),
        ],
    );
    let draw_empty = SyntaxNode::new(NodeKind::DrawDecl, vec![]);
    let ward_missing_name = SyntaxNode::new(
        NodeKind::WardDecl,
        vec![
            tok(TokenKind::Ward, 8, 9),
            SyntaxElement::Node(SyntaxNode::new(
                NodeKind::ExprStmt,
                vec![tok(TokenKind::Ident, 0, 1)],
            )),
        ],
    );

    let root = SyntaxNode::new(
        NodeKind::SourceFile,
        vec![
            SyntaxElement::Node(impl_block),
            SyntaxElement::Node(let_stmt),
            SyntaxElement::Node(draw_existing),
            SyntaxElement::Node(draw_empty),
            SyntaxElement::Node(ward_missing_name),
        ],
    );

    resolver.resolve_source_file(&root, source);

    assert!(resolver.current_scope.resolve("n").is_some());
    assert!(resolver.current_scope.resolve("inner").is_some());

    fs::remove_dir_all(base).expect("temp dir should be removable");
}

#[test]
fn resolver_ignores_invalid_ident_spans_without_panicking() {
    let invalid_ident = Token::new(
        TokenKind::Ident,
        Span::new(BytePos(99), BytePos(120), SourceId(0)),
    );
    let root = SyntaxNode::new(
        NodeKind::SourceFile,
        vec![SyntaxElement::Token(invalid_ident)],
    );
    let mut resolver = Resolver::default();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        resolver.resolve_source_file(&root, "abc");
    }));

    assert!(result.is_ok());
}
