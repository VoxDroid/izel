use std::fs;
use std::path::PathBuf;

#[test]
fn test_custom_iterator_typechecks() {
    let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/custom_iterator.iz");
    let source = fs::read_to_string(&input).expect("failed to read fixture source");

    let source_id = izel_span::SourceId(0);
    let mut lexer = izel_lexer::Lexer::new(&source, source_id);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        let kind = token.kind;
        tokens.push(token);
        if kind == izel_lexer::TokenKind::Eof {
            break;
        }
    }

    let mut parser = izel_parser::Parser::new(tokens, source.clone());
    let cst = parser.parse_source_file();

    let base_path = input.parent().map(|p| p.to_path_buf());
    let mut resolver = izel_resolve::Resolver::new(base_path);
    resolver.resolve_source_file(&cst, &source);

    let ast_lowerer = izel_ast_lower::Lowerer::new(&source);
    let ast = ast_lowerer.lower_module(&cst);

    let mut typeck = izel_typeck::TypeChecker::with_builtins();
    typeck.span_to_def = resolver.def_ids.clone();

    let mut ast_modules = std::collections::HashMap::new();
    let loaded_csts = resolver
        .loaded_csts
        .read()
        .expect("loaded_csts lock poisoned");
    for (name, (loaded_cst, loaded_source)) in loaded_csts.iter() {
        let lowerer = izel_ast_lower::Lowerer::new(loaded_source);
        ast_modules.insert(name.clone(), lowerer.lower_module(loaded_cst));
    }
    drop(loaded_csts);

    typeck.check_project(&ast, ast_modules);

    assert!(
        typeck.diagnostics.is_empty(),
        "custom iterator fixture must typecheck cleanly, diagnostics: {:?}",
        typeck.diagnostics
    );
}
