use anyhow::Result;
use clap::Parser;
use izel_session::{Session, SessionOptions};

fn main() -> Result<()> {
    let options = SessionOptions::parse();
    let session = Session::new(options);

    println!("⬡ Izel Compiler (izelc) — Foundation Scaffolding Complete.");
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
                izel_pm::resolve_dependencies(&manifest.dependencies)
                    .map_err(|e| anyhow::anyhow!(e))?;
                return Ok(());
            }
        }
    }

    let input_path = session
        .options
        .input
        .as_ref()
        .expect("Input file required for compilation");
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
    let mut resolver = izel_resolve::Resolver::new();
    resolver.resolve_source_file(&cst, &source);

    println!("Desugaring AST...");
    let ast_lowerer = izel_ast_lower::Lowerer::new(&source);
    let _ast = ast_lowerer.lower_module(&cst);

    println!("Type checking...");
    let mut typeck = izel_typeck::TypeChecker::with_builtins();
    typeck.check_ast(&_ast);

    if !typeck.diagnostics.is_empty() {
        let mut source_map = izel_span::SourceMap::default();
        source_map.add(input_path.to_string_lossy().to_string(), source.clone());
        for diag in &typeck.diagnostics {
            izel_diagnostics::emit(&source_map, diag);
        }
        std::process::exit(1);
    }

    println!("Lowering AST to HIR...");
    let hir_lowerer = izel_hir::lower::HirLowerer::new();
    let hir_module = hir_lowerer.lower_module(&_ast);

    println!("Borrow checking...");
    let mut mir_lowerer = izel_mir::lower::MirLowerer::new();
    mir_lowerer.check_contracts = session.options.check_contracts;
    let mut borrow_checker = izel_borrow::BorrowChecker::new();

    for item in &hir_module.items {
        if let izel_hir::HirItem::Forge(f) = item {
            let mir = mir_lowerer.lower_forge(f);
            if let Err(errors) = borrow_checker.check(&mir) {
                for err in errors {
                    eprintln!("Borrow Check Error in '{}': {}", f.name, err);
                }
            }
        }
    }

    println!("Generating LLVM IR...");
    let context = inkwell::context::Context::create();
    let mut codegen = izel_codegen::Codegen::new(&context, "main", &source);
    codegen.gen_module(&_ast)?;

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
