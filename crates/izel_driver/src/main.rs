use clap::Parser;
use izel_session::{Session, SessionOptions};
use anyhow::Result;

fn main() -> Result<()> {
    let options = SessionOptions::parse();
    let session = Session::new(options);

    println!("⬡ Izel Compiler (izelc) — Foundation Scaffolding Complete.");
    println!("Creator: @VoxDroid <izeno.contact@gmail.com>");
    println!("Repository: https://github.com/VoxDroid/izel\n");

    let source = std::fs::read_to_string(&session.options.input)?;
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
    let mut parser = izel_parser::Parser::new(tokens);
    let cst = parser.parse_source_file();
    
    println!("Generating LLVM IR...");
    let context = inkwell::context::Context::create();
    let mut codegen = izel_codegen::Codegen::new(&context, "main", &source);
    codegen.gen_source_file(&cst)?;

    println!("--- LLVM IR ---\n{}", codegen.emit_llvm_ir());
    println!("---------------\n");

    if session.options.run {
        codegen.run_jit()?;
    }
    
    Ok(())
}

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
