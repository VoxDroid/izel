use izel_resolve::{DefId, Resolver, Scope};
use izel_span::{BytePos, SourceId, Span};
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
