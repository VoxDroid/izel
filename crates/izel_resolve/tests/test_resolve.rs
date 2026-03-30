use izel_lexer::{Lexer, TokenKind};
use izel_parser::{cst::SyntaxNode, Parser};
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
