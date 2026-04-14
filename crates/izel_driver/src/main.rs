use anyhow::Result;
use clap::Parser;
use izel_session::{Session, SessionOptions};

fn main() -> Result<()> {
    let options = SessionOptions::parse();
    let session = Session::new(options);

    println!("⬡ Izel Compiler (izelc) — Active Prototype Pipeline.");
    println!("Creator: @VoxDroid <izeno.contact@gmail.com>");
    println!("Repository: https://github.com/VoxDroid/izel\n");

    if let Some(cmd) = &session.options.command {
        match cmd {
            izel_session::Command::Fmt { input } => {
                let source = std::fs::read_to_string(input)?;
                let formatted = izel_fmt::format_source(&source);
                println!("{}", formatted);
                return Ok(());
            }
            izel_session::Command::Lsp => {
                izel_lsp::run_server_sync();
                return Ok(());
            }
            izel_session::Command::Deps { manifest_path } => {
                let toml_str = std::fs::read_to_string(manifest_path)?;
                let manifest =
                    izel_pm::parse_manifest(&toml_str).map_err(|e| anyhow::anyhow!(e))?;
                println!(
                    "Loaded manifest for package: {} v{}",
                    manifest.package.name, manifest.package.version
                );
                izel_pm::resolve_dependencies_with_registry(
                    &manifest.dependencies,
                    &manifest.registry,
                )
                .map_err(|e| anyhow::anyhow!(e))?;
                return Ok(());
            }
        }
    }

    let input_path = session
        .options
        .input
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Input file required for compilation"))?;
    let source = std::fs::read_to_string(input_path)?;
    let source_id = izel_span::SourceId(0);
    let mut lexer = izel_lexer::Lexer::new(&source, source_id);

    println!("Lexing file: {:?}", session.options.input);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        let kind = token.kind;
        tokens.push(token);
        if kind == izel_lexer::TokenKind::Eof {
            break;
        }
    }

    println!("Parsing CST...");
    let mut parser = izel_parser::Parser::new(tokens, source.to_string());
    let cst = parser.parse_source_file();

    println!("Resolving symbols...");
    let base_path = std::path::Path::new(input_path)
        .parent()
        .map(|p| p.to_path_buf());
    let mut resolver = izel_resolve::Resolver::new(base_path);
    resolver.resolve_source_file(&cst, &source);

    println!("Desugaring AST...");
    let ast_lowerer = izel_ast_lower::Lowerer::new(&source);
    let _ast = ast_lowerer.lower_module(&cst);

    println!("Type checking...");
    let mut typeck = izel_typeck::TypeChecker::with_builtins();
    typeck.span_to_def = resolver.def_ids.clone();

    // Collect all loaded modules as ASTs for type checking
    let mut ast_modules = std::collections::HashMap::new();
    let loaded_csts = resolver.loaded_csts.read().unwrap();
    for (name, (cst, source)) in loaded_csts.iter() {
        let mod_ast_lowerer = izel_ast_lower::Lowerer::new(source);
        let ast = mod_ast_lowerer.lower_module(cst);
        ast_modules.insert(name.clone(), ast);
    }

    // Drop the lock before calling typecheck, as it might draw more modules (though not currently)
    drop(loaded_csts);

    typeck.check_project(&_ast, ast_modules);

    if !typeck.diagnostics.is_empty() {
        let mut source_map = izel_span::SourceMap::default();
        source_map.add(input_path.to_string_lossy().to_string(), source.clone());
        for diag in &typeck.diagnostics {
            izel_diagnostics::emit(&source_map, diag);
        }
        std::process::exit(1);
    }

    println!("Lowering AST to HIR...");
    let hir_lowerer = izel_hir::lower::HirLowerer::new(&resolver, &typeck.def_types);
    let mut all_hir_items = Vec::new();

    // Lower primary module
    let primary_hir = hir_lowerer.lower_module(&_ast);
    all_hir_items.extend(primary_hir.items);

    // Lower all loaded modules
    let loaded_csts_hir = resolver.loaded_csts.read().unwrap();
    for (name, (cst, source)) in loaded_csts_hir.iter() {
        println!("Lowering module to HIR: {}", name);
        let mod_ast_lowerer = izel_ast_lower::Lowerer::new(source);
        let ast = mod_ast_lowerer.lower_module(cst);
        let mod_hir = hir_lowerer.lower_module(&ast);
        all_hir_items.extend(mod_hir.items);
    }
    drop(loaded_csts_hir);

    let hir_module = izel_hir::HirModule {
        items: all_hir_items,
    };

    println!("Borrow checking & lowering to MIR...");
    let mut mir_lowerer = izel_mir::lower::MirLowerer::new();
    mir_lowerer.check_contracts = session.options.check_contracts;
    let mut borrow_checker = izel_borrow::BorrowChecker::new();
    let mut mir_bodies = std::collections::HashMap::new();

    fn collect_mir(
        items: &[izel_hir::HirItem],
        mir_lowerer: &mut izel_mir::lower::MirLowerer,
        borrow_checker: &mut izel_borrow::BorrowChecker,
        mir_bodies: &mut std::collections::HashMap<izel_resolve::DefId, izel_mir::MirBody>,
    ) {
        for item in items {
            match item {
                izel_hir::HirItem::Forge(f) => {
                    let mir = mir_lowerer.lower_forge(f);
                    if let Err(errors) = borrow_checker.check(&mir) {
                        for err in errors {
                            eprintln!("Borrow Check Error in '{}': {}", f.name, err);
                        }
                    }
                    mir_bodies.insert(f.def_id, mir.clone());
                }
                izel_hir::HirItem::Ward(w) => {
                    collect_mir(&w.items, mir_lowerer, borrow_checker, mir_bodies);
                }
                _ => {}
            }
        }
    }

    collect_mir(
        &hir_module.items,
        &mut mir_lowerer,
        &mut borrow_checker,
        &mut mir_bodies,
    );

    println!("Generating LLVM IR...");
    let context = inkwell::context::Context::create();
    let mut codegen = izel_codegen::Codegen::new(&context, "main", &source);
    codegen.gen_module(&hir_module, &mir_bodies)?;

    println!("--- LLVM IR ---\n{}", codegen.emit_llvm_ir());
    println!("---------------\n");

    if session.options.run {
        codegen.run_jit()?;
    }

    Ok(())
}

#[allow(dead_code)]
fn print_cst(element: &izel_parser::cst::SyntaxElement, indent: usize) {
    let space = "  ".repeat(indent);
    match element {
        izel_parser::cst::SyntaxElement::Node(node) => {
            println!("{}{:?}", space, node.kind);
            for child in &node.children {
                print_cst(child, indent + 1);
            }
        }
        izel_parser::cst::SyntaxElement::Token(token) => {
            println!("{}{:?}", space, token.kind);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::print_cst;
    use izel_lexer::{Token, TokenKind};
    use izel_parser::cst::{NodeKind, SyntaxElement, SyntaxNode};
    use izel_span::Span;

    #[test]
    fn print_cst_handles_node_and_token_variants() {
        let leaf = SyntaxElement::Token(Token::new(TokenKind::Ident, Span::dummy()));
        let node = SyntaxElement::Node(SyntaxNode::new(NodeKind::Ident, vec![leaf.clone()]));

        print_cst(&node, 0);
        print_cst(&leaf, 1);
    }
}
