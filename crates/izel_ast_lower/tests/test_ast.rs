use izel_ast_lower::Lowerer;
use izel_lexer::{Lexer, TokenKind};
use izel_parser::ast;
use izel_parser::Parser;
use izel_span::SourceId;

fn lower_module(source: &str) -> ast::Module {
    let mut lexer = Lexer::new(source, SourceId(0));
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        let kind = token.kind;
        tokens.push(token);
        if kind == TokenKind::Eof {
            break;
        }
    }

    let mut parser = Parser::new(tokens, source.to_string());
    let cst = parser.parse_source_file();
    Lowerer::new(source).lower_module(&cst)
}

#[test]
fn lowers_shape_and_forge_items() {
    let source = r#"
shape Point { x: i32, y: i32 }
forge main() { give 0 }
"#;
    let module = lower_module(source);

    assert!(module
        .items
        .iter()
        .any(|item| { matches!(item, ast::Item::Shape(shape) if shape.name == "Point") }));
    assert!(module
        .items
        .iter()
        .any(|item| { matches!(item, ast::Item::Forge(forge) if forge.name == "main") }));
}

#[test]
fn lowers_draw_path_segments() {
    let module = lower_module("draw std::io");
    let draw = module
        .items
        .iter()
        .find_map(|item| match item {
            ast::Item::Draw(draw) => Some(draw),
            _ => None,
        })
        .expect("expected one draw item");

    assert_eq!(draw.path, vec!["std".to_string(), "io".to_string()]);
    assert!(!draw.is_wildcard);
}
