use std::fs;
use std::path::PathBuf;

fn assert_fixture_typechecks(path: &str) {
    let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
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
        "fixture '{}' must typecheck cleanly, diagnostics: {:?}",
        path,
        typeck.diagnostics
    );
}

#[test]
fn test_custom_iterator_typechecks() {
    assert_fixture_typechecks("tests/fixtures/custom_iterator.iz");
}

#[test]
fn test_custom_witness_typechecks() {
    assert_fixture_typechecks("tests/fixtures/custom_witness.iz");
}

#[test]
fn test_phase7_self_hosting_sources_exist() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lexer = repo_root.join("compiler/lexer.iz");
    let parser = repo_root.join("compiler/parser.iz");
    let driver = repo_root.join("compiler/izelc.iz");

    for path in [&lexer, &parser, &driver] {
        assert!(path.exists(), "expected self-hosting source at {:?}", path);
    }
}

#[test]
fn test_phase7_izelc_pipeline_surface_present() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../compiler/izelc.iz");
    let src =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));

    let required = [
        "forge main",
        "forge compile_file",
        "~ tokens = tokenize(source)",
        "~ ast = parse_module(tokens)",
        "~ resolved = resolve_module(ast)",
        "~ checked = typecheck_module(resolved)",
        "~ hir = lower_to_hir(checked)",
        "~ mir = lower_to_mir(hir, config.check_contracts)",
        "~ optimized = run_mir_passes(mir)",
        "~ llvm_ir = codegen_llvm(optimized)",
    ];

    for symbol in required {
        assert!(
            src.contains(symbol),
            "missing self-hosted izelc pipeline symbol: {}",
            symbol
        );
    }
}

#[test]
fn test_phase7_bootstrap_harness_present() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = repo_root.join("tools/bootstrap/self_host_bootstrap.sh");
    let checksums = repo_root.join("tools/bootstrap/bootstrap_sources.sha256");

    assert!(script.exists(), "expected bootstrap script at {:?}", script);
    assert!(
        checksums.exists(),
        "expected checksum manifest at {:?}",
        checksums
    );
}

#[test]
fn test_phase7_bootstrap_harness_has_expected_steps() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/bootstrap/self_host_bootstrap.sh");
    let src =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));

    let required = [
        "sha256sum -c",
        "cargo build -p izel_driver",
        "target/debug/izelc",
        "compiler/izelc.iz",
        "--execute",
    ];

    for symbol in required {
        assert!(
            src.contains(symbol),
            "missing bootstrap harness step: {}",
            symbol
        );
    }
}

#[test]
fn test_phase7_public_registry_seed_present() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let registry_readme = repo_root.join("registry/README.md");
    let package_index = repo_root.join("registry/index/packages.json");

    assert!(
        registry_readme.exists(),
        "expected registry documentation at {:?}",
        registry_readme
    );
    assert!(
        package_index.exists(),
        "expected registry index seed at {:?}",
        package_index
    );

    let src = fs::read_to_string(&package_index)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", package_index, e));
    assert!(src.contains("\"registry\": \"izel-public\""));
    assert!(src.contains("\"name\": \"std\""));
}

#[test]
fn test_phase7_tree_sitter_grammar_assets_present() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let grammar_js = repo_root.join("tools/grammar/tree-sitter-izel/grammar.js");
    let package_json = repo_root.join("tools/grammar/tree-sitter-izel/package.json");
    let highlights = repo_root.join("tools/grammar/tree-sitter-izel/queries/highlights.scm");

    assert!(
        grammar_js.exists(),
        "expected tree-sitter grammar at {:?}",
        grammar_js
    );
    assert!(
        package_json.exists(),
        "expected tree-sitter package metadata at {:?}",
        package_json
    );
    assert!(
        highlights.exists(),
        "expected tree-sitter highlights query at {:?}",
        highlights
    );
}

#[test]
fn test_phase7_tree_sitter_grammar_contains_core_rules() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/grammar/tree-sitter-izel/grammar.js");
    let src =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));

    let required = [
        "name: \"izel\"",
        "forge_decl",
        "shape_decl",
        "scroll_decl",
        "ward_decl",
        "draw_decl",
        "binary_expr",
        "|>",
    ];

    for symbol in required {
        assert!(
            src.contains(symbol),
            "missing tree-sitter core grammar symbol: {}",
            symbol
        );
    }
}

#[test]
fn test_phase7_playground_assets_present() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let html = repo_root.join("tools/playground/index.html");
    let js = repo_root.join("tools/playground/main.js");
    let css = repo_root.join("tools/playground/styles.css");
    let wasm_toml = repo_root.join("tools/playground/wasm/Cargo.toml");
    let wasm_lib = repo_root.join("tools/playground/wasm/src/lib.rs");

    for path in [&html, &js, &css, &wasm_toml, &wasm_lib] {
        assert!(path.exists(), "expected playground asset at {:?}", path);
    }
}

#[test]
fn test_phase7_playground_contains_wasm_repl_wiring() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let js_path = repo_root.join("tools/playground/main.js");
    let wasm_lib = repo_root.join("tools/playground/wasm/src/lib.rs");

    let js = fs::read_to_string(&js_path)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", js_path, e));
    let lib = fs::read_to_string(&wasm_lib)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", wasm_lib, e));

    let required_js = [
        "./pkg/izel_playground.js",
        "repl_eval",
        "WASM playground loaded",
    ];
    let required_lib = ["wasm_bindgen", "pub fn repl_eval("];

    for symbol in required_js {
        assert!(
            js.contains(symbol),
            "missing playground JS symbol: {}",
            symbol
        );
    }
    for symbol in required_lib {
        assert!(
            lib.contains(symbol),
            "missing playground wasm bridge symbol: {}",
            symbol
        );
    }
}

#[test]
fn test_system_dependency_checker_present() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let checker = repo_root.join("tools/ci/check_system_deps.sh");

    assert!(
        checker.exists(),
        "expected system dependency checker at {:?}",
        checker
    );
}

#[test]
fn test_system_dependency_checker_covers_required_tools() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/ci/check_system_deps.sh");
    let src =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));

    let required = [
        "llvm-config",
        "ld.lld",
        "clang",
        "cmake",
        "zlib",
        "--report-only",
    ];

    for symbol in required {
        assert!(
            src.contains(symbol),
            "missing system dependency check symbol: {}",
            symbol
        );
    }
}

#[test]
fn test_commit_convention_checker_present() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let checker = repo_root.join("tools/ci/check_commit_message.sh");

    assert!(
        checker.exists(),
        "expected commit convention checker at {:?}",
        checker
    );
}

#[test]
fn test_commit_convention_checker_defines_required_types() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/ci/check_commit_message.sh");
    let src =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));

    let required = [
        "feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert",
        "--message",
        "--from-file",
        "Conventional Commit",
    ];

    for symbol in required {
        assert!(
            src.contains(symbol),
            "missing commit checker symbol: {}",
            symbol
        );
    }
}
